use std::collections::HashMap;
use std::time::Instant;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Ole::CF_UNICODETEXT;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};

use super::canvas::Canvas;
use super::types::{Color, Rect};
use crate::theme::Theme;
use crate::tree::{
    NodeId, NodeKind, Style, Tree, ACC_HEADER, BAR_ITEM, CAL_HEADER, CAL_WEEK, DOCK_HEADER,
    GROUP_HEADER, LIST_ROW, OFF_LIMIT, POPUP_ROW, SCROLLBAR_W, SPLIT_ARROW, SPLIT_W,
    TABLE_HEADER, TABLE_ROW, TAB_HEADER, TERM_INPUT, TERM_ROW,
};

const MONTHS: [&str; 12] = [
    "Январь",
    "Февраль",
    "Март",
    "Апрель",
    "Май",
    "Июнь",
    "Июль",
    "Август",
    "Сентябрь",
    "Октябрь",
    "Ноябрь",
    "Декабрь",
];

const WEEKDAYS: [&str; 7] = ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"];

const CRUMB_SEP: f32 = 22.0;

const PAGER_CELL: f32 = 40.0;

fn crumb_width(n: usize) -> f32 {
    12.0 + n as f32 * 10.0
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
    }
}

fn first_weekday(y: i32, m: u32) -> u32 {
    let (yy, mm) = if m < 3 { (y - 1, m + 12) } else { (y, m) };
    let k = yy.rem_euclid(100);
    let j = yy.div_euclid(100);
    let h = (1 + (13 * (mm as i32 + 1)) / 5 + k + k / 4 + j / 4 + 5 * j).rem_euclid(7);
    (h + 5).rem_euclid(7) as u32
}

fn wrapped_ranges(
    dwrite: &IDWriteFactory,
    format: &IDWriteTextFormat,
    text: &[u16],
    width: f32,
    start: usize,
    end: usize,
) -> Vec<Rect> {
    if text.is_empty() || end <= start {
        return Vec::new();
    }
    unsafe {
        let layout = match dwrite.CreateTextLayout(text, format, width, 4096.0) {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };
        let mut count: u32 = 0;
        let _ = layout.HitTestTextRange(
            start as u32,
            (end - start) as u32,
            0.0,
            0.0,
            None,
            &mut count,
        );
        if count == 0 {
            return Vec::new();
        }
        let mut metrics = vec![DWRITE_HIT_TEST_METRICS::default(); count as usize];
        let mut actual: u32 = 0;
        if layout
            .HitTestTextRange(
                start as u32,
                (end - start) as u32,
                0.0,
                0.0,
                Some(&mut metrics),
                &mut actual,
            )
            .is_err()
        {
            return Vec::new();
        }
        metrics
            .iter()
            .take(actual as usize)
            .map(|m| Rect::new(m.left, m.top, m.width, m.height))
            .collect()
    }
}

fn hsv_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = h.clamp(0.0, 1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (r + m, g + m, b + m)
}

fn color_code(h: f32, s: f32, v: f32) -> f32 {
    let (r, g, b) = hsv_rgb(h, s, v);
    let ri = (r * 255.0).round() as u32;
    let gi = (g * 255.0).round() as u32;
    let bi = (b * 255.0).round() as u32;
    ((ri << 16) | (gi << 8) | bi) as f32
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CursorKind {
    Arrow,
    Hand,
    IBeam,
}

struct DialogView {
    title: Vec<u16>,
    message: Vec<u16>,
    buttons: Vec<Vec<u16>>,
    hover: Option<usize>,
    focus: Option<usize>,
    msg_scroll: f32,
}

struct Popup {
    id: NodeId,
    rect: Rect,
    items: Vec<Vec<u16>>,
    hover: Option<usize>,
    base: usize,
}

struct NoteView {
    title: Vec<u16>,
    text: Vec<u16>,
    action: Vec<u16>,
    kind: u8,
    corner: u8,
    secs: f32,
    born: Instant,
    cb: Option<Box<dyn FnMut(&mut Tree)>>,
}

pub struct Renderer {
    swap_chain: IDXGISwapChain1,
    context: ID2D1DeviceContext,
    rt: ID2D1RenderTarget,
    target: Option<ID2D1Bitmap1>,
    dwrite: IDWriteFactory,
    text_format: IDWriteTextFormat,
    text_format_left: IDWriteTextFormat,
    text_format_wrap: IDWriteTextFormat,
    text_format_tip: IDWriteTextFormat,
    tree: Tree,
    width: f32,
    height: f32,
    hovered: Option<NodeId>,
    pressed: Option<NodeId>,
    dragging: Option<NodeId>,
    focused: Option<NodeId>,
    hot: Option<NodeId>,
    text_selecting: bool,
    last_click: Option<(Instant, f32, f32)>,
    focus_ring: bool,
    scroll_drag: Option<NodeId>,
    split_drag: Option<NodeId>,
    range_drag: Option<(NodeId, bool)>,
    knob_drag: Option<(NodeId, f32, f32)>,
    color_drag: Option<(NodeId, u8)>,
    canvas_drag: Option<NodeId>,
    canvas_pan: Option<(NodeId, f32, f32, f32, f32)>,
    tree_bar: Option<NodeId>,
    popup: Option<Popup>,
    hover_since: Option<(NodeId, Instant)>,
    toast: Option<(Vec<u16>, Instant, f32)>,
    notes: Vec<NoteView>,
    theme: Theme,
    theme_index: usize,
    last_tick: Instant,
    inspector: bool,
    open_dropdown: Option<NodeId>,
    dropdown_hover: Option<usize>,
    mouse: (f32, f32),
    open_menu: Option<(f32, f32)>,
    menu_hover: Option<usize>,
    dialog: Option<DialogView>,
    glass: bool,
    wic: Option<IWICImagingFactory>,
    img_cache: HashMap<String, Option<ID2D1Bitmap>>,
    grad_cache: std::cell::RefCell<HashMap<(u32, u32), ID2D1LinearGradientBrush>>,
    layout_cache: std::cell::RefCell<super::canvas::LayoutCache>,
    fmt_cache: std::cell::RefCell<HashMap<(u16, u32, u8), IDWriteTextFormat>>,
    frame_latency: Option<HANDLE>,
    _dcomp: Option<IDCompositionDevice>,
    _dcomp_target: Option<IDCompositionTarget>,
    _dcomp_visual: Option<IDCompositionVisual>,
}

impl Renderer {
    /// Создаёт рендерер Direct2D, привязанный к окну `hwnd`.
    pub fn new(
        hwnd: HWND,
        tree: Tree,
        glass: bool,
        tint: f32,
        width: i32,
        height: i32,
    ) -> Result<Self> {
        unsafe {
            let feature_levels = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0];
            let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;

            let mut device: Option<ID3D11Device> = None;
            let mut chosen_level = D3D_FEATURE_LEVEL_11_0;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                flags,
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut device),
                Some(&mut chosen_level),
                None,
            )?;
            let device = device.unwrap();

            let dxgi_device: IDXGIDevice = device.cast()?;
            let adapter: IDXGIAdapter = dxgi_device.GetAdapter()?;
            let factory: IDXGIFactory2 = adapter.GetParent()?;

            let swap_chain: IDXGISwapChain1 = if glass {
                let desc = DXGI_SWAP_CHAIN_DESC1 {
                    Width: width.max(1) as u32,
                    Height: height.max(1) as u32,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    Stereo: FALSE,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                    BufferCount: 2,
                    Scaling: DXGI_SCALING_STRETCH,
                    SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                    AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
                    Flags: 0,
                };
                factory.CreateSwapChainForComposition(&device, &desc, None)?
            } else {
                let desc = DXGI_SWAP_CHAIN_DESC1 {
                    Width: 0,
                    Height: 0,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    Stereo: FALSE,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                    BufferCount: 2,
                    Scaling: DXGI_SCALING_NONE,
                    SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                    AlphaMode: DXGI_ALPHA_MODE_IGNORE,
                    Flags: DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0 as u32,
                };
                factory.CreateSwapChainForHwnd(&device, hwnd, &desc, None, None)?
            };
            let _ = factory.MakeWindowAssociation(hwnd, DXGI_MWA_NO_ALT_ENTER);

            let frame_latency = if glass {
                None
            } else if let Ok(sc2) = swap_chain.cast::<IDXGISwapChain2>() {
                let _ = sc2.SetMaximumFrameLatency(1);
                let h = sc2.GetFrameLatencyWaitableObject();
                if h.0.is_null() {
                    None
                } else {
                    Some(h)
                }
            } else {
                None
            };

            let (dcomp, dcomp_target, dcomp_visual) = if glass {
                let dcomp: IDCompositionDevice = DCompositionCreateDevice(&dxgi_device)?;
                let target = dcomp.CreateTargetForHwnd(hwnd, true)?;
                let visual = dcomp.CreateVisual()?;
                visual.SetContent(&swap_chain)?;
                target.SetRoot(&visual)?;
                dcomp.Commit()?;
                (Some(dcomp), Some(target), Some(visual))
            } else {
                (None, None, None)
            };

            let d2d_factory: ID2D1Factory1 =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let d2d_device = d2d_factory.CreateDevice(&dxgi_device)?;
            let context = d2d_device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
            context.SetDpi(96.0, 96.0);
            context.SetTextAntialiasMode(if glass {
                D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE
            } else {
                D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE
            });
            let rt: ID2D1RenderTarget = context.cast()?;

            let dwrite: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let base_family = crate::tree::base_font();
            let base_size = crate::tree::base_size();
            let fam = windows::core::PCWSTR(base_family.as_ptr());
            let text_format = dwrite.CreateTextFormat(
                fam,
                None,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                base_size * 1.2,
                w!("en-us"),
            )?;
            let _ = text_format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
            let _ = text_format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);

            let text_format_left = dwrite.CreateTextFormat(
                fam,
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                base_size,
                w!("en-us"),
            )?;
            let _ = text_format_left.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
            let _ = text_format_left.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);

            let text_format_wrap = dwrite.CreateTextFormat(
                fam,
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                base_size,
                w!("en-us"),
            )?;
            let _ = text_format_wrap.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
            let _ = text_format_wrap.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR);
            let _ = text_format_wrap.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP);

            let text_format_tip = dwrite.CreateTextFormat(
                fam,
                None,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                base_size * 1.2,
                w!("en-us"),
            )?;
            let _ = text_format_tip.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
            let _ = text_format_tip.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            let _ = text_format_tip.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);

            let wic: Option<IWICImagingFactory> = None;

            let theme_index = tree.theme();
            let theme = theme_from_index(theme_index);
            let mut renderer = Renderer {
                swap_chain,
                context,
                rt,
                target: None,
                dwrite,
                text_format,
                text_format_left,
                text_format_wrap,
                text_format_tip,
                tree,
                width: 1280.0,
                height: 720.0,
                hovered: None,
                pressed: None,
                dragging: None,
                focused: None,
                hot: None,
                text_selecting: false,
                last_click: None,
                focus_ring: false,
                scroll_drag: None,
                split_drag: None,
                range_drag: None,
                knob_drag: None,
                color_drag: None,
                canvas_drag: None,
                canvas_pan: None,
                tree_bar: None,
                popup: None,
                hover_since: None,
                toast: None,
                notes: Vec::new(),
                theme,
                theme_index,
                last_tick: Instant::now(),
                inspector: false,
                open_dropdown: None,
                dropdown_hover: None,
                mouse: (0.0, 0.0),
                open_menu: None,
                menu_hover: None,
                dialog: None,
                glass,
                wic,
                img_cache: HashMap::new(),
                grad_cache: std::cell::RefCell::new(HashMap::new()),
                layout_cache: std::cell::RefCell::new(Default::default()),
                fmt_cache: std::cell::RefCell::new(HashMap::new()),
                frame_latency,
                _dcomp: dcomp,
                _dcomp_target: dcomp_target,
                _dcomp_visual: dcomp_visual,
            };
            renderer.tree.set_tint(tint);
            renderer.create_target()?;
            Ok(renderer)
        }
    }

    fn create_target(&mut self) -> Result<()> {
        unsafe {
            let surface: IDXGISurface = self.swap_chain.GetBuffer(0)?;
            let props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: if self.glass {
                        D2D1_ALPHA_MODE_PREMULTIPLIED
                    } else {
                        D2D1_ALPHA_MODE_IGNORE
                    },
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                colorContext: std::mem::ManuallyDrop::new(None),
            };
            let bitmap = self
                .context
                .CreateBitmapFromDxgiSurface(&surface, Some(&props))?;
            self.context.SetTarget(&bitmap);
            self.target = Some(bitmap);
            Ok(())
        }
    }

    /// Меняет размер буферов под новый размер клиентской области.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width as f32;
        self.height = height as f32;
        self.grad_cache.borrow_mut().clear();
        self.layout_cache.borrow_mut().clear();
        unsafe {
            self.context.SetTarget(None);
            self.target = None;
            let flags = if self.frame_latency.is_some() {
                DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT
            } else {
                DXGI_SWAP_CHAIN_FLAG(0)
            };
            if self
                .swap_chain
                .ResizeBuffers(0, width, height, DXGI_FORMAT_UNKNOWN, flags)
                .is_err()
            {
                return;
            }
            let _ = self.create_target();
        }
    }

    /// Продвигает анимации, таймеры и спиннеры; true — нужна перерисовка.
    pub fn pump(&mut self) -> bool {
        let posted = self.tree.fire_frame();
        let mut built = self.tree.apply_build_queue()
            | self.tree.apply_css_queue()
            | self.tree.apply_text_queue();
        if self.poll_menu() {
            built = true;
        }
        let scrolled =
            self.tree.apply_canvas_queue() | self.tree.apply_tree_queue() | built;
        self.tree.sync_tree_geom();
        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f32();
        self.last_tick = now;
        let anim = self.tree.tick(dt);
        let fired = if self.tree.has_timers() {
            self.tree.tick_timers()
        } else {
            false
        };
        let spin = self.tree.has_spinner();
        if spin {
            self.tree.spin(dt);
        }
        anim
            || spin
            || fired
            || posted
            || scrolled
            || self.toast.is_some()
            || !self.notes.is_empty()
            || self.tip_pending()
    }

    /// Нельзя ли усыплять цикл: есть незавершённые таймеры.
    pub fn busy(&self) -> bool {
        self.tree.has_timers()
    }

    /// Забирает запросы файловых диалогов для показа.
    pub fn take_files(&mut self) -> Vec<crate::tree::FileReq> {
        self.tree.take_files()
    }

    /// Доставляет выбранный путь колбэку файлового диалога.
    pub fn deliver_file(&mut self, req: crate::tree::FileReq, path: String) {
        self.tree.deliver_file(req, path);
    }

    /// Сообщает о новом размере клиентской области окна.
    pub fn fire_resize(&mut self, w: f32, h: f32) {
        self.tree.fire_resize(w, h);
    }

    /// Продвигает анимации по таймеру; true, если нужна перерисовка.
    pub fn on_timer(&mut self) -> bool {
        self.pump()
    }

    fn tip_pending(&self) -> bool {
        match self.hover_since {
            Some((id, since)) => {
                self.tree.tip(id).is_some() && since.elapsed().as_secs_f32() < 1.2
            }
            None => false,
        }
    }

    /// Пересобирает базовые форматы из текущего шрифта приложения.
    fn rebuild_fonts(&mut self) {
        let base = crate::tree::base_font();
        let size = crate::tree::base_size();
        let fam = windows::core::PCWSTR(base.as_ptr());
        unsafe {
            if let Ok(f) = self.dwrite.CreateTextFormat(
                fam,
                None,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size * 1.2,
                w!("en-us"),
            ) {
                let _ = f.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
                let _ = f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
                self.text_format = f;
            }
            if let Ok(f) = self.dwrite.CreateTextFormat(
                fam,
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size,
                w!("en-us"),
            ) {
                let _ = f.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
                let _ = f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
                self.text_format_left = f;
            }
            if let Ok(f) = self.dwrite.CreateTextFormat(
                fam,
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size,
                w!("en-us"),
            ) {
                let _ = f.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
                let _ = f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR);
                let _ = f.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP);
                self.text_format_wrap = f;
            }
            if let Ok(f) = self.dwrite.CreateTextFormat(
                fam,
                None,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size * 1.2,
                w!("en-us"),
            ) {
                let _ = f.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
                let _ = f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
                let _ = f.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
                self.text_format_tip = f;
            }
        }
        self.fmt_cache.borrow_mut().clear();
        self.layout_cache.borrow_mut().clear();
    }

    /// Задаёт режим и тон фонового размытия.
    pub fn set_blur(&mut self, mode: u32, tint: u32) {
        self.tree.set_blur_mode(mode);
        self.tree.set_blur_tint(tint);
    }

    /// Текущий режим фонового размытия.
    pub fn blur_mode(&self) -> u32 {
        self.tree.blur_mode()
    }

    /// Текущий тон фонового размытия.
    pub fn blur_tint(&self) -> u32 {
        self.tree.blur_tint()
    }

    /// Гасить ли размытие при перемещении окна.
    pub fn drag_smooth(&self) -> bool {
        self.tree.drag_smooth()
    }

    /// Передаёт перетащенные файлы в зону приёма под точкой.
    pub fn on_drop(&mut self, x: f32, y: f32, paths: &str) -> bool {
        if let Some(id) = self.tree.hit_test(x, y) {
            if self.tree.is_drop(id) {
                self.tree.fire_input_text(id, paths);
                return true;
            }
        }
        false
    }

    /// Прокручивает таблицу под курсором колесом мыши.
    pub fn on_wheel(&mut self, delta: i32) -> bool {
        if self.dialog.is_some() {
            let content_h = self.dialog_msg_height(392.0);
            let max_scroll = (content_h - 96.0).max(0.0);
            if let Some(d) = self.dialog.as_mut() {
                let step = delta as f32 / 120.0 * 40.0;
                d.msg_scroll = (d.msg_scroll - step).clamp(0.0, max_scroll);
            }
            return true;
        }
        let (mx, my) = self.mouse;
        let mut node = self.hot;
        let mut guard = 0;
        while let Some(id) = node {
            if self.tree.has_wheel(id) {
                let (lx, ly) = self.tree.canvas_local(id, mx, my);
                self.tree.fire_wheel(id, delta as f32 / 120.0, lx, ly);
                return true;
            }
            node = self.tree.get(id).parent;
            guard += 1;
            if guard > 64 {
                break;
            }
        }
        if let Some(hot) = self.hot {
            let step = (delta as f32 / 120.0) * 0.05;
            if self.tree.is_canvas(hot) && self.tree.is_canvas_scroll(hot) {
                let d = -(delta as f32 / 120.0) * 60.0;
                if key_down(0x10) {
                    self.tree.canvas_scroll_by(hot, d, 0.0);
                } else {
                    self.tree.canvas_scroll_by(hot, 0.0, d);
                }
                return true;
            }
            if self.tree.is_slider(hot) {
                let v = (self.tree.slider_value(hot) + step).clamp(0.0, 1.0);
                self.tree.set_slider_value(hot, v);
                self.tree.fire_change(hot, v);
                return true;
            }
            if self.tree.is_dial(hot) {
                let v = (self.tree.dial_value(hot) + step).clamp(0.0, 1.0);
                self.tree.set_dial_value(hot, v);
                self.tree.fire_change(hot, v);
                return true;
            }
            if self.tree.is_tabs(hot) {
                let r = self.tree.get(hot).rect;
                if self.mouse.1 <= r.y + TAB_HEADER {
                    let n = self.tree.tabs_len(hot);
                    if n > 0 {
                        let cur = self.tree.tabs_selected(hot);
                        let next = if delta > 0 {
                            if cur == 0 { 0 } else { cur - 1 }
                        } else {
                            (cur + 1).min(n - 1)
                        };
                        if next != cur {
                            self.tree.set_tabs_selected(hot, next);
                            self.tree.fire_change(hot, next as f32);
                        }
                    }
                    return true;
                }
            }
        }
        if let Some(mut id) = self.hot {
            let mut guard = 0;
            while !self.tree.is_scroll(id)
                && !self.tree.is_list(id)
                && !self.tree.is_table(id)
                && !self.tree.is_tree(id)
                && !self.tree.is_propgrid(id)
                && !self.tree.is_term(id)
                && guard < 32
            {
                match self.tree.parent(id) {
                    Some(p) => id = p,
                    None => break,
                }
                guard += 1;
            }
            if self.tree.is_scroll(id) {
                let r = self.tree.get(id).rect;
                let content = self.tree.scroll_content(id);
                let max_scroll = (content - r.height).max(0.0);
                let cur = self.tree.scroll_offset(id);
                let next = (cur - (delta as f32 / 120.0) * 48.0).clamp(0.0, max_scroll);
                if (next - cur).abs() > 0.01 {
                    self.tree.set_scroll_offset(id, next);
                    return true;
                }
                return false;
            }
            if self.tree.is_list(id) {
                let (max_scroll, _, _) = self.list_metrics(id);
                let cur = self.tree.list_scroll(id);
                let next = (cur - (delta as f32 / 120.0) * LIST_ROW).clamp(0.0, max_scroll);
                if (next - cur).abs() > 0.01 {
                    self.tree.set_list_scroll(id, next);
                    return true;
                }
                return false;
            }
            if self.tree.is_term(id) {
                let r = self.tree.get(id).rect;
                let content = self.tree.term_len(id) as f32 * TERM_ROW;
                let visible = (r.height - TERM_INPUT - 16.0).max(0.0);
                let max_scroll = (content - visible).max(0.0);
                let cur = self.tree.term_scroll(id);
                let next = (cur - (delta as f32 / 120.0) * TERM_ROW).clamp(0.0, max_scroll);
                if (next - cur).abs() > 0.01 {
                    self.tree.set_term_scroll(id, next);
                    return true;
                }
                return false;
            }
            if self.tree.is_propgrid(id) {
                let r = self.tree.get(id).rect;
                let content = self.tree.prop_len(id) as f32 * LIST_ROW;
                let max_scroll = (content - r.height).max(0.0);
                let cur = self.tree.prop_scroll(id);
                let next = (cur - (delta as f32 / 120.0) * LIST_ROW).clamp(0.0, max_scroll);
                if (next - cur).abs() > 0.01 {
                    self.tree.set_prop_scroll(id, next);
                    return true;
                }
                return false;
            }
            if self.tree.is_tree(id) {
                let r = self.tree.get(id).rect;
                let content = self.tree.tree_visible(id).len() as f32 * LIST_ROW;
                let max_scroll = (content - r.height).max(0.0);
                let cur = self.tree.tree_scroll(id);
                let next = (cur - (delta as f32 / 120.0) * LIST_ROW).clamp(0.0, max_scroll);
                if (next - cur).abs() > 0.01 {
                    self.tree.set_tree_scroll(id, next);
                    return true;
                }
                return false;
            }
            if self.tree.is_table(id) {
                let r = self.tree.get(id).rect;
                let n = self.tree.table_len(id);
                let content = n as f32 * TABLE_ROW;
                let visible = (r.height - TABLE_HEADER).max(0.0);
                let max_scroll = (content - visible).max(0.0);
                let step = TABLE_ROW;
                let cur = self.tree.table_scroll(id);
                let next = (cur - (delta as f32 / 120.0) * step).clamp(0.0, max_scroll);
                if (next - cur).abs() > 0.01 {
                    self.tree.set_table_scroll(id, next);
                    return true;
                }
            }
        }
        false
    }

    fn list_metrics(&self, id: NodeId) -> (f32, f32, f32) {
        let r = self.tree.get(id).rect;
        let n = self.tree.list_len(id);
        let content = n as f32 * LIST_ROW;
        let visible = r.height.max(0.0);
        ((content - visible).max(0.0), content, visible)
    }

    fn list_row_at(&self, id: NodeId, y: f32) -> Option<usize> {
        let r = self.tree.get(id).rect;
        let n = self.tree.list_len(id);
        if n == 0 || y < r.y {
            return None;
        }
        let scroll = self.tree.list_scroll(id);
        let i = ((y - r.y + scroll) / LIST_ROW).floor();
        if i < 0.0 {
            return None;
        }
        let i = i as usize;
        if i < n {
            Some(i)
        } else {
            None
        }
    }

    fn scrollbar_zone(&self, id: NodeId, x: f32) -> bool {
        let r = self.tree.get(id).rect;
        x >= r.x + r.width - SCROLLBAR_W - 2.0
    }

    fn is_double_click(&mut self, x: f32, y: f32) -> bool {
        let now = Instant::now();
        let dbl = matches!(
            self.last_click,
            Some((t, lx, ly))
                if now.duration_since(t).as_millis() < 400
                    && (lx - x).abs() < 6.0
                    && (ly - y).abs() < 6.0
        );
        self.last_click = Some((now, x, y));
        dbl
    }

    fn splitter_bar_at(&self, x: f32, y: f32) -> Option<NodeId> {
        let mut found = None;
        self.tree.for_each(|id, node| {
            if !matches!(node.kind, NodeKind::Splitter { .. }) {
                return;
            }
            let r = node.rect;
            if r.x <= OFF_LIMIT {
                return;
            }
            let ratio = self.tree.split_ratio(id);
            let bar = if self.tree.split_vertical(id) {
                let w1 = (r.width - SPLIT_W) * ratio;
                Rect::new(r.x + w1, r.y, SPLIT_W, r.height)
            } else {
                let h1 = (r.height - SPLIT_W) * ratio;
                Rect::new(r.x, r.y + h1, r.width, SPLIT_W)
            };
            if bar.contains(x, y) {
                found = Some(id);
            }
        });
        found
    }

    fn set_split_from(&mut self, id: NodeId, x: f32, y: f32) {
        let r = self.tree.get(id).rect;
        let v = if self.tree.split_vertical(id) {
            if r.width <= SPLIT_W {
                return;
            }
            (x - r.x) / (r.width - SPLIT_W)
        } else {
            if r.height <= SPLIT_W {
                return;
            }
            (y - r.y) / (r.height - SPLIT_W)
        };
        self.tree.set_split_ratio(id, v);
    }

    fn scroll_from_y(&mut self, id: NodeId, y: f32) {
        let r = self.tree.get(id).rect;
        if self.tree.is_scroll(id) {
            let content = self.tree.scroll_content(id);
            let max_scroll = (content - r.height).max(0.0);
            if max_scroll <= 0.0 || r.height <= 0.0 {
                return;
            }
            let t = ((y - r.y) / r.height).clamp(0.0, 1.0);
            self.tree.set_scroll_offset(id, t * max_scroll);
            return;
        }
        let is_list = self.tree.is_list(id);
        let (max_scroll, track_y, track_h) = if is_list {
            let (m, _, _) = self.list_metrics(id);
            (m, r.y, r.height)
        } else {
            let n = self.tree.table_len(id);
            let content = n as f32 * TABLE_ROW;
            let visible = (r.height - TABLE_HEADER).max(0.0);
            ((content - visible).max(0.0), r.y + TABLE_HEADER, visible)
        };
        if max_scroll <= 0.0 || track_h <= 0.0 {
            return;
        }
        let t = ((y - track_y) / track_h).clamp(0.0, 1.0);
        let v = t * max_scroll;
        if is_list {
            self.tree.set_list_scroll(id, v);
        } else {
            self.tree.set_table_scroll(id, v);
        }
    }

    fn select_tree_row(&mut self, id: NodeId, row: usize) {
        self.tree.set_tree_selected(id, Some(row));
        self.tree.reveal_row(id, row);
        if self.tree.is_tree_msel(id) {
            self.tree.set_tree_multi(id, vec![row]);
            self.tree.fire_point(id, 7, row as i32, 0.0, 0.0);
        } else {
            self.tree.fire_change(id, row as f32);
        }
        self.tree.fire_point(id, 5, row as i32, 0.0, 0.0);
    }

    fn set_tree_scroll_from_y(&mut self, id: NodeId, y: f32) {
        let r = self.tree.get(id).rect;
        let head = self.tree.tree_head(id);
        let vis = self.tree.tree_visible(id).len() as f32;
        let view = (r.height - head).max(0.0);
        let max = (vis * LIST_ROW - view).max(0.0);
        let track_y = r.y + head + 2.0;
        let track_h = (view - 4.0).max(1.0);
        let t = ((y - track_y) / track_h).clamp(0.0, 1.0);
        self.tree.set_tree_scroll(id, t * max);
    }

    fn reveal_table_row(&mut self, id: NodeId, row: usize) {
        let r = self.tree.get(id).rect;
        let visible = (r.height - TABLE_HEADER).max(0.0);
        let row_top = row as f32 * TABLE_ROW;
        let row_bot = row_top + TABLE_ROW;
        let mut scroll = self.tree.table_scroll(id);
        if row_top < scroll {
            scroll = row_top;
        } else if row_bot > scroll + visible {
            scroll = row_bot - visible;
        }
        let n = self.tree.table_len(id);
        let content = n as f32 * TABLE_ROW;
        let max_scroll = (content - visible).max(0.0);
        self.tree.set_table_scroll(id, scroll.clamp(0.0, max_scroll));
    }

    /// Открывает контекстное меню по правому клику.
    pub fn on_right_down(&mut self, x: f32, y: f32) -> bool {
        if self.dialog.is_some() {
            return false;
        }
        if let Some(mut id) = self.tree.hit_test(x, y) {
            if self.tree.is_tree(id) {
                match self.tree_row_at(id, y) {
                    Some(item) => {
                        let held = if self.tree.is_tree_msel(id) {
                            self.tree.tree_multi(id).contains(&item)
                        } else {
                            self.tree.tree_selected(id) == Some(item)
                        };
                        if !held {
                            self.select_tree_row(id, item);
                        }
                    }
                    None => self.clear_tree_selection(id),
                }
            }
            let mut guard = 0;
            loop {
                if self.tree.has_point(id, 9) {
                    self.tree.fire_point(id, 9, 0, x, y);
                    return true;
                }
                match self.tree.parent(id) {
                    Some(p) if guard < 64 => {
                        id = p;
                        guard += 1;
                    }
                    _ => break,
                }
            }
        }
        let n = self.tree.window_menu_len();
        if n == 0 {
            return false;
        }
        self.tree.arm_menu();
        self.close_dropdown();
        let mw = 220.0;
        let mh = n as f32 * MENU_ROW;
        let mx = x.min((self.width - mw).max(0.0));
        let my = y.min((self.height - mh).max(0.0));
        self.open_menu = Some((mx, my));
        self.menu_hover = None;
        true
    }

    fn poll_menu(&mut self) -> bool {
        let req = self.tree.take_menu_req();
        let Some((items, x, y)) = req else {
            return false;
        };
        if items.is_empty() {
            self.tree.set_menu_live(Vec::new());
            self.open_menu = None;
            self.menu_hover = None;
            return true;
        }
        let n = items.len();
        self.tree.set_menu_live(items);
        self.close_dropdown();
        let mw = 220.0;
        let mh = n as f32 * MENU_ROW;
        let mx = x.min((self.width - mw).max(0.0)).max(0.0);
        let my = y.min((self.height - mh).max(0.0)).max(0.0);
        self.open_menu = Some((mx, my));
        self.menu_hover = None;
        true
    }

    fn tree_row_at(&self, id: NodeId, y: f32) -> Option<usize> {
        let r = self.tree.get(id).rect;
        let head = self.tree.tree_head(id);
        if y < r.y + head {
            return None;
        }
        let scroll = self.tree.tree_scroll(id);
        let vis = self.tree.tree_visible(id);
        let i = ((y - r.y - head + scroll) / LIST_ROW).floor();
        if i < 0.0 || (i as usize) >= vis.len() {
            return None;
        }
        Some(vis[i as usize])
    }

    fn clear_tree_selection(&mut self, id: NodeId) {
        self.tree.set_tree_selected(id, None);
        if self.tree.is_tree_msel(id) {
            self.tree.set_tree_multi(id, Vec::new());
            self.tree.fire_point(id, 7, -1, 0.0, 0.0);
        } else {
            self.tree.fire_change(id, -1.0);
        }
    }

    fn menu_rect(&self) -> Option<Rect> {
        let (mx, my) = self.open_menu?;
        let n = self.tree.menu_len();
        Some(Rect::new(mx, my, 220.0, n as f32 * MENU_ROW))
    }

    fn menu_option_at(&self, y: f32) -> Option<usize> {
        let rect = self.menu_rect()?;
        let n = self.tree.menu_len();
        if y < rect.y {
            return None;
        }
        let i = ((y - rect.y) / MENU_ROW).floor();
        if i < 0.0 {
            return None;
        }
        let i = i as usize;
        if i < n {
            Some(i)
        } else {
            None
        }
    }

    fn dialog_rects(&self) -> Option<(Rect, Vec<Rect>)> {
        let d = self.dialog.as_ref()?;
        let pw = 440.0;
        let ph = 220.0;
        let px = (self.width - pw) / 2.0;
        let py = (self.height - ph) / 2.0;
        let panel = Rect::new(px, py, pw, ph);
        let n = d.buttons.len().max(1);
        let pad = 16.0;
        let bh = 44.0;
        let by = py + ph - pad - bh;
        let bw = (pw - pad * (n as f32 + 1.0)) / n as f32;
        let mut btns = Vec::new();
        for i in 0..d.buttons.len() {
            let bx = px + pad + (bw + pad) * i as f32;
            btns.push(Rect::new(bx, by, bw, bh));
        }
        Some((panel, btns))
    }

    fn dialog_button_at(&self, x: f32, y: f32) -> Option<usize> {
        let (_, btns) = self.dialog_rects()?;
        btns.iter().position(|r| r.contains(x, y))
    }

    /// Полная высота сообщения диалога при переносе по ширине области.
    fn dialog_msg_height(&self, width: f32) -> f32 {
        let d = match self.dialog.as_ref() {
            Some(d) => d,
            None => return 0.0,
        };
        if d.message.is_empty() {
            return 0.0;
        }
        unsafe {
            if let Ok(layout) = self.dwrite.CreateTextLayout(
                &d.message,
                &self.text_format_wrap,
                width.max(1.0),
                100000.0,
            ) {
                let mut m = DWRITE_TEXT_METRICS::default();
                if layout.GetMetrics(&mut m).is_ok() {
                    return m.height;
                }
            }
        }
        0.0
    }

    /// Тип курсора под текущим положением мыши.
    pub fn cursor_kind(&self) -> CursorKind {
        match self.hot {
            Some(id) if self.tree.is_interactive(id) => CursorKind::Hand,
            Some(id) if self.tree.is_dropdown(id) => CursorKind::Hand,
            Some(id) if self.tree.is_split(id) => CursorKind::Hand,
            Some(id) if self.tree.is_menubar(id) => CursorKind::Hand,
            Some(id) if self.tree.is_crumbs(id) => CursorKind::Hand,
            Some(id) if self.tree.is_pager(id) => CursorKind::Hand,
            Some(id) if self.tree.is_rating(id) => CursorKind::Hand,
            Some(id) if self.tree.is_tabs(id) => CursorKind::Hand,
            Some(id) if self.tree.is_accordion(id) => CursorKind::Hand,
            Some(id) if self.tree.is_textbox(id) => CursorKind::IBeam,
            Some(id) if self.tree.is_term(id) => CursorKind::IBeam,
            _ => CursorKind::Arrow,
        }
    }

    fn textbox_index_at(&self, id: NodeId, x: f32) -> usize {
        let rect = self.tree.get(id).rect;
        let pad = 12.0;
        match self.tree.textbox_state(id) {
            Some(st) => {
                let local_x = x - (rect.x + pad - st.scroll);
                index_at_x(&self.dwrite, &self.text_format_left, &st.text, local_x)
            }
            None => 0,
        }
    }

    fn textarea_index_at(&self, id: NodeId, x: f32, y: f32) -> usize {
        let rect = self.tree.get(id).rect;
        let pad = 10.0;
        match self.tree.textbox_state(id) {
            Some(st) => {
                let width = (rect.width - 2.0 * pad).max(1.0);
                let lx = x - (rect.x + pad);
                let ly = y - (rect.y + pad);
                wrapped_index(&self.dwrite, &self.text_format_wrap, &st.text, width, lx, ly)
            }
            None => 0,
        }
    }

    fn dropdown_popup_rect(&self, id: NodeId) -> Rect {
        let r = self.tree.get(id).rect;
        let n = self.tree.dropdown_len(id);
        Rect::new(r.x, r.y + r.height, r.width, r.height * n as f32)
    }

    fn dropdown_option_at(&self, id: NodeId, y: f32) -> Option<usize> {
        let r = self.tree.get(id).rect;
        let n = self.tree.dropdown_len(id);
        if r.height <= 0.0 || n == 0 {
            return None;
        }
        let i = ((y - (r.y + r.height)) / r.height).floor();
        if i < 0.0 {
            return None;
        }
        let i = i as usize;
        if i < n {
            Some(i)
        } else {
            None
        }
    }

    fn close_dropdown(&mut self) {
        if let Some(dd) = self.open_dropdown.take() {
            self.tree.set_dropdown_open(dd, false);
        }
        self.dropdown_hover = None;
    }

    /// Выбирает строку таблицы и шлёт колбэк с колонкой.
    fn pick_table(&mut self, id: NodeId, x: f32, y: f32) {
        if let Some(row) = self.table_row_at(id, y) {
            let col = self.tree.table_col_at(id, x);
            self.tree.set_table_selected(id, Some(row));
            self.tree.fire_change(id, row as f32);
            self.tree.fire_point(id, 5, row as i32, col as f32, 0.0);
        }
    }

    fn table_row_at(&self, id: NodeId, y: f32) -> Option<usize> {
        let r = self.tree.get(id).rect;
        let n = self.tree.table_len(id);
        let top = r.y + TABLE_HEADER;
        if y < top || n == 0 {
            return None;
        }
        let scroll = self.tree.table_scroll(id);
        let i = ((y - top + scroll) / TABLE_ROW).floor();
        if i < 0.0 {
            return None;
        }
        let i = i as usize;
        if i < n {
            Some(i)
        } else {
            None
        }
    }

    /// Обрабатывает движение мыши. Возвращает true, если нужна перерисовка.
    pub fn on_mouse_move(&mut self, x: f32, y: f32) -> bool {
        self.mouse = (x, y);
        if self.dialog.is_some() {
            let hv = self.dialog_button_at(x, y);
            let mut changed = false;
            if let Some(d) = self.dialog.as_mut() {
                if d.hover != hv {
                    d.hover = hv;
                    changed = true;
                }
            }
            return changed;
        }
        let mut dirty = false;
        let hit = self.tree.hit_test(x, y);
        if self.hot != hit {
            self.hover_since = hit
                .filter(|&h| self.tree.tip(h).is_some())
                .map(|h| (h, Instant::now()));
            dirty = true;
        }
        self.hot = hit;
        if hit.map_or(false, |h| self.tree.is_table(h)) {
            dirty = true;
        }
        if let Some(dd) = self.open_dropdown {
            let popup = self.dropdown_popup_rect(dd);
            let hv = if popup.contains(x, y) {
                self.dropdown_option_at(dd, y)
            } else {
                None
            };
            if hv != self.dropdown_hover {
                self.dropdown_hover = hv;
                dirty = true;
            }
        }
        if self.open_menu.is_some() {
            if let Some(rect) = self.menu_rect() {
                let hv = if rect.contains(x, y) {
                    self.menu_option_at(y)
                } else {
                    None
                };
                if hv != self.menu_hover {
                    self.menu_hover = hv;
                    dirty = true;
                }
            }
        }
        let hover = hit.filter(|&id| self.tree.is_interactive(id));
        if hover != self.hovered {
            self.hovered = hover;
            dirty = true;
        }
        if let Some(id) = self.dragging {
            self.set_slider_from_x(id, x);
            dirty = true;
        }
        if let Some(id) = self.scroll_drag {
            self.scroll_from_y(id, y);
            dirty = true;
        }
        if let Some(id) = self.split_drag {
            self.set_split_from(id, x, y);
            dirty = true;
        }
        if let Some((id, upper)) = self.range_drag {
            self.set_range_from_x(id, upper, x);
            dirty = true;
        }
        if let Some((id, y0, v0)) = self.knob_drag {
            let v = (v0 + (y0 - y) / 160.0).clamp(0.0, 1.0);
            self.tree.set_dial_value(id, v);
            self.tree.fire_change(id, v);
            dirty = true;
        }
        if let Some((id, mode)) = self.color_drag {
            self.set_color_from(id, mode, x, y);
            dirty = true;
        }
        if let Some(id) = self.tree_bar {
            self.set_tree_scroll_from_y(id, y);
            dirty = true;
        }
        if let Some((id, sx, sy, px, py)) = self.canvas_pan {
            self.tree.set_canvas_view(id, px - (x - sx), py - (y - sy));
            dirty = true;
        }
        if let Some(id) = self.canvas_drag {
            if self.tree.has_point(id, 1) {
                let i = self.tree.canvas_hit(id, x, y).map_or(-1, |v| v as i32);
                let (lx, ly) = self.tree.canvas_local(id, x, y);
                self.tree.fire_point(id, 1, i, lx, ly);
                dirty = true;
            }
        }
        if let Some(p) = self.popup.as_mut() {
            let hv = if p.rect.contains(x, y) {
                let i = ((y - p.rect.y) / POPUP_ROW).floor();
                if i >= 0.0 && (i as usize) < p.items.len() {
                    Some(i as usize)
                } else {
                    None
                }
            } else {
                None
            };
            if hv != p.hover {
                p.hover = hv;
                dirty = true;
            }
        }
        if hit.map_or(false, |h| self.tree.is_list(h)) {
            dirty = true;
        }
        if self.text_selecting {
            if let Some(id) = self.focused {
                let idx = if self.tree.is_multiline(id) {
                    self.textarea_index_at(id, x, y)
                } else {
                    self.textbox_index_at(id, x)
                };
                if let Some(st) = self.tree.textbox_state_mut(id) {
                    st.set_caret(idx, true);
                }
                dirty = true;
            }
        }
        dirty
    }

    /// Обрабатывает нажатие левой кнопки. Возвращает true, если нужна перерисовка.
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> bool {
        if self.dialog.is_none() && self.open_menu.is_none() && self.popup.is_none() {
            if let Some(prev) = self.focused {
                if self.tree.is_textbox(prev) && self.tree.hit_test(x, y) != Some(prev) {
                    self.focused = None;
                    self.tree.fire_change(prev, 0.0);
                }
            }
        }
        if self.dialog.is_some() {
            if let Some(i) = self.dialog_button_at(x, y) {
                self.dialog = None;
                self.tree.fire_dialog(i as i32);
            }
            return true;
        }
        if let Some(layer) = self.tree.popup_layer() {
            if !self.tree.get(layer).rect.contains(x, y) {
                self.tree.close_popup();
                self.tree.fire_point(layer, 8, 0, 0.0, 0.0);
                return true;
            }
        }
        for (i, _, act) in self.note_rects() {
            if !self.notes[i].action.is_empty() && act.contains(x, y) {
                let mut cb = self.notes[i].cb.take();
                self.notes.remove(i);
                if let Some(c) = cb.as_mut() {
                    c(&mut self.tree);
                }
                return true;
            }
        }
        if let Some(p) = self.popup.take() {
            if p.rect.contains(x, y) {
                let i = ((y - p.rect.y) / POPUP_ROW).floor();
                if i >= 0.0 && (i as usize) < p.items.len() {
                    let idx = p.base * 1000 + i as usize;
                    self.tree.fire_change(p.id, idx as f32);
                }
                return true;
            }
        }
        if self.open_menu.is_some() {
            if let Some(rect) = self.menu_rect() {
                if rect.contains(x, y) {
                    if let Some(i) = self.menu_option_at(y) {
                        let root = self.tree.root();
                        self.tree.fire_change(root, i as f32);
                    }
                    self.open_menu = None;
                    self.menu_hover = None;
                    return true;
                }
            }
            self.open_menu = None;
            self.menu_hover = None;
        }
        if let Some(dd) = self.open_dropdown {
            let popup = self.dropdown_popup_rect(dd);
            let header = self.tree.get(dd).rect;
            if popup.contains(x, y) {
                if let Some(i) = self.dropdown_option_at(dd, y) {
                    self.tree.set_dropdown_selected(dd, i);
                    self.tree.fire_change(dd, i as f32);
                }
                self.close_dropdown();
                return true;
            }
            self.close_dropdown();
            if header.contains(x, y) {
                return true;
            }
        }

        self.text_selecting = false;
        self.focus_ring = false;
        self.tree.raise_front(x, y);
        let hit = self.tree.hit_test(x, y);
        let new_focus = hit.filter(|&id| self.tree.is_textbox(id) || self.tree.is_slider(id));
        self.focused = new_focus;

        if let Some(id) = self.splitter_bar_at(x, y) {
            if self.is_double_click(x, y) {
                self.tree.set_split_ratio(id, 0.5);
                return true;
            }
            self.split_drag = Some(id);
            self.set_split_from(id, x, y);
            return true;
        }
        if let Some(id) = hit {
            if self.tree.is_accordion(id) {
                let r = self.tree.get(id).rect;
                if y <= r.y + ACC_HEADER {
                    self.tree.toggle_acc(id);
                    self.focused = Some(id);
                    return true;
                }
            }
            if self.tree.is_scroll(id) && self.scrollbar_zone(id, x) {
                self.scroll_drag = Some(id);
                self.scroll_from_y(id, y);
                return true;
            }
            if (self.tree.is_list(id) || self.tree.is_table(id)) && self.scrollbar_zone(id, x) {
                self.scroll_drag = Some(id);
                self.scroll_from_y(id, y);
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_list(id) {
                if let Some(ri) = self.list_row_at(id, y) {
                    if self.tree.is_list_msel(id) {
                        let ctrl = key_down(0x11);
                        let shift = key_down(0x10);
                        let mut sel = self.tree.list_multi(id);
                        if shift {
                            let a = self.tree.list_selected(id).unwrap_or(ri);
                            let lo = a.min(ri);
                            let hi = a.max(ri);
                            sel = (lo..=hi).collect();
                        } else if ctrl {
                            match sel.iter().position(|v| *v == ri) {
                                Some(p) => {
                                    sel.remove(p);
                                }
                                None => sel.push(ri),
                            }
                        } else {
                            sel = vec![ri];
                        }
                        self.tree.set_list_multi(id, sel);
                        if !shift {
                            self.tree.set_list_selected(id, Some(ri));
                        }
                        self.tree.fire_point(id, 7, ri as i32, 0.0, 0.0);
                    } else {
                        self.tree.set_list_selected(id, Some(ri));
                        self.tree.fire_change(id, ri as f32);
                    }
                }
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_dropdown(id) {
                let sel = self.tree.dropdown_selected(id);
                self.tree.set_dropdown_open(id, true);
                self.open_dropdown = Some(id);
                self.dropdown_hover = Some(sel);
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_tabs(id) {
                let r = self.tree.get(id).rect;
                if y <= r.y + TAB_HEADER {
                    let n = self.tree.tabs_len(id);
                    if n > 0 {
                        let step = r.width / n as f32;
                        let i = (((x - r.x) / step).floor() as i32).clamp(0, n as i32 - 1) as usize;
                        self.tree.set_tabs_selected(id, i);
                        self.tree.fire_change(id, i as f32);
                    }
                }
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_canvas(id) {
                let i = self.tree.canvas_hit(id, x, y);
                let idx = i.map_or(-1, |v| v as i32);
                let dbl = self.is_double_click(x, y);
                let (lx, ly) = self.tree.canvas_local(id, x, y);
                self.tree.fire_change(id, idx as f32);
                self.tree.fire_point(id, 0, idx, lx, ly);
                if dbl {
                    self.tree.fire_point(id, 3, idx, lx, ly);
                }
                self.canvas_drag = Some(id);
                if self.tree.is_canvas_scroll(id) && !self.tree.has_point(id, 1) {
                    let (px, py) = self.tree.canvas_offset(id);
                    self.canvas_pan = Some((id, x, y, px, py));
                }
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_table(id) {
                self.pick_table(id, x, y);
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_textbox(id) {
                let idx = if self.tree.is_multiline(id) {
                    self.textarea_index_at(id, x, y)
                } else {
                    self.textbox_index_at(id, x)
                };
                let dbl = self.is_double_click(x, y);
                if let Some(st) = self.tree.textbox_state_mut(id) {
                    if dbl {
                        st.select_word(idx);
                    } else {
                        st.set_caret(idx, false);
                    }
                }
                self.text_selecting = !dbl;
                return true;
            }
            if self.tree.is_split(id) {
                let r = self.tree.get(id).rect;
                if x >= r.x + r.width - SPLIT_ARROW {
                    let items = self.tree.split_options(id);
                    let rect = Rect::new(
                        r.x,
                        r.y + r.height,
                        r.width,
                        POPUP_ROW * items.len() as f32,
                    );
                    self.popup = Some(Popup {
                        id,
                        rect,
                        items,
                        hover: None,
                        base: 0,
                    });
                } else {
                    self.tree.fire_click(id);
                }
                return true;
            }
            if self.tree.is_menubar(id) {
                let r = self.tree.get(id).rect;
                let n = self.tree.bar_len(id);
                let i = ((x - r.x) / BAR_ITEM).floor();
                if i >= 0.0 && (i as usize) < n {
                    let i = i as usize;
                    let items = self.tree.bar_items(id, i);
                    let rect = Rect::new(
                        r.x + i as f32 * BAR_ITEM,
                        r.y + r.height,
                        220.0,
                        POPUP_ROW * items.len() as f32,
                    );
                    self.popup = Some(Popup {
                        id,
                        rect,
                        items,
                        hover: None,
                        base: i,
                    });
                }
                return true;
            }
            if self.tree.is_interactive(id) {
                self.pressed = Some(id);
                return true;
            }
            if self.tree.is_slider(id) {
                self.dragging = Some(id);
                self.set_slider_from_x(id, x);
                return true;
            }
            if self.tree.is_dock(id) {
                let r = self.tree.get(id).rect;
                if y <= r.y + DOCK_HEADER {
                    self.tree.toggle_dock(id);
                    self.focused = Some(id);
                    return true;
                }
            }
            if self.tree.is_term(id) {
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_pager(id) {
                let r = self.tree.get(id).rect;
                let (page, total) = self.tree.pager_state(id);
                if total > 0 {
                    let i = ((x - r.x) / PAGER_CELL).floor();
                    let cells = total as i32 + 2;
                    if i >= 0.0 && (i as i32) < cells {
                        let i = i as i32;
                        let next = if i == 0 {
                            page.saturating_sub(1)
                        } else if i == cells - 1 {
                            (page + 1).min(total - 1)
                        } else {
                            (i - 1) as usize
                        };
                        if next != page {
                            self.tree.set_pager_page(id, next);
                            self.tree.fire_change(id, next as f32);
                        }
                    }
                }
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_rating(id) {
                let r = self.tree.get(id).rect;
                let (_, max) = self.tree.rating_state(id);
                if max > 0 {
                    let cell = (r.width / max as f32).max(1.0);
                    let i = ((x - r.x) / cell).floor();
                    if i >= 0.0 && (i as usize) < max {
                        let v = i as usize + 1;
                        self.tree.set_rating_value(id, v);
                        self.tree.fire_change(id, v as f32);
                    }
                }
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_crumbs(id) {
                let r = self.tree.get(id).rect;
                let items = self.tree.crumb_items(id);
                let mut cx = r.x + 10.0;
                for (i, it) in items.iter().enumerate() {
                    let w = crumb_width(it.len());
                    if x >= cx && x <= cx + w {
                        self.tree.crumb_truncate(id, i);
                        self.tree.fire_change(id, i as f32);
                        break;
                    }
                    cx += w + CRUMB_SEP;
                }
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_time(id) {
                let r = self.tree.get(id).rect;
                let up = y < r.y + r.height / 2.0;
                let left = x < r.x + r.width / 2.0;
                let step = if up { 1 } else { -1 };
                if left {
                    self.tree.time_shift(id, step, 0);
                } else {
                    self.tree.time_shift(id, 0, step * 5);
                }
                let code = self.tree.time_code(id);
                self.tree.fire_change(id, code);
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_propgrid(id) {
                let r = self.tree.get(id).rect;
                let n = self.tree.prop_len(id);
                let scroll = self.tree.prop_scroll(id);
                let i = ((y - r.y + scroll) / LIST_ROW).floor();
                if i >= 0.0 && (i as usize) < n {
                    let i = i as usize;
                    self.tree.set_prop_selected(id, Some(i));
                    self.tree.fire_change(id, i as f32);
                }
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_dial(id) {
                self.knob_drag = Some((id, y, self.tree.dial_value(id)));
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_color(id) {
                let (area, strip) = self.color_zones(id);
                if strip.contains(x, y) {
                    self.color_drag = Some((id, 1));
                    self.set_color_from(id, 1, x, y);
                } else if area.contains(x, y) {
                    self.color_drag = Some((id, 0));
                    self.set_color_from(id, 0, x, y);
                }
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_calendar(id) {
                let r = self.tree.get(id).rect;
                if y <= r.y + CAL_HEADER {
                    if x <= r.x + 40.0 {
                        self.tree.cal_shift(id, -1);
                    } else if x >= r.x + r.width - 40.0 {
                        self.tree.cal_shift(id, 1);
                    }
                    let code = self.tree.cal_code(id);
                    self.tree.fire_change(id, code);
                } else {
                    let (yr, mo, _) = self.tree.cal_ymd(id);
                    let cw = r.width / 7.0;
                    let rh = ((r.height - CAL_HEADER - CAL_WEEK) / 6.0).max(1.0);
                    let col = ((x - r.x) / cw).floor();
                    let row = ((y - r.y - CAL_HEADER - CAL_WEEK) / rh).floor();
                    if col >= 0.0 && col < 7.0 && row >= 0.0 && row < 6.0 {
                        let idx = row as i32 * 7 + col as i32;
                        let d = idx - first_weekday(yr, mo) as i32 + 1;
                        if d >= 1 && d <= days_in_month(yr, mo) as i32 {
                            self.tree.set_cal_day(id, d as u32);
                            let code = self.tree.cal_code(id);
                            self.tree.fire_change(id, code);
                        }
                    }
                }
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_tree(id) {
                let r = self.tree.get(id).rect;
                let vis = self.tree.tree_visible(id);
                let scroll = self.tree.tree_scroll(id);
                let head = self.tree.tree_head(id);
                if y < r.y + head {
                    self.focused = Some(id);
                    return true;
                }
                if self.scrollbar_zone(id, x) {
                    self.tree_bar = Some(id);
                    self.set_tree_scroll_from_y(id, y);
                    self.focused = Some(id);
                    return true;
                }
                let twice = self.is_double_click(x, y);
                let i = ((y - r.y - head + scroll) / LIST_ROW).floor();
                if i >= 0.0 && (i as usize) < vis.len() {
                    let item = vis[i as usize];
                    let col = self.tree.tree_col_at(id, x) as i32;
                    if let Some((depth, _, _, leaf)) = self.tree.tree_item(id, item) {
                        let bounds = self.tree.tree_bounds(id);
                        let ax = bounds[0].0 + 8.0 + depth as f32 * 18.0;
                        if !leaf && x >= ax && x <= ax + 20.0 {
                            self.tree.toggle_tree(id, item);
                            self.tree.clamp_tree_scroll(id);
                        } else {
                            if self.tree.is_tree_msel(id) {
                                let ctrl = key_down(0x11);
                                let shift = key_down(0x10);
                                let mut sel = self.tree.tree_multi(id);
                                if shift {
                                    let a = self.tree.tree_selected(id).unwrap_or(item);
                                    let lo = a.min(item);
                                    let hi = a.max(item);
                                    sel = vis
                                        .iter()
                                        .copied()
                                        .filter(|v| *v >= lo && *v <= hi)
                                        .collect();
                                } else if ctrl {
                                    match sel.iter().position(|v| *v == item) {
                                        Some(p) => {
                                            sel.remove(p);
                                        }
                                        None => sel.push(item),
                                    }
                                } else {
                                    sel = vec![item];
                                }
                                self.tree.set_tree_multi(id, sel);
                                if !shift {
                                    self.tree.set_tree_selected(id, Some(item));
                                }
                                self.tree.fire_point(id, 7, item as i32, 0.0, 0.0);
                            } else {
                                self.tree.set_tree_selected(id, Some(item));
                                self.tree.fire_change(id, item as f32);
                            }
                            self.tree.fire_point(id, 5, item as i32, col as f32, 0.0);
                            if twice {
                                self.tree.fire_point(id, 6, item as i32, col as f32, 0.0);
                            }
                        }
                    }
                } else {
                    self.clear_tree_selection(id);
                }
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_range(id) {
                let r = self.tree.get(id).rect;
                let (lo, hi) = self.tree.range_values(id);
                let t = if r.width > 0.0 {
                    ((x - r.x) / r.width).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let upper = (t - hi).abs() < (t - lo).abs();
                self.range_drag = Some((id, upper));
                self.set_range_from_x(id, upper, x);
                return true;
            }
        }
        true
    }

    /// Обрабатывает отпускание левой кнопки. Возвращает true, если нужна перерисовка.
    pub fn on_mouse_up(&mut self) -> bool {
        let click_id = if self.pressed.is_some() && self.pressed == self.hovered {
            self.pressed
        } else {
            None
        };
        let was_pressed = self.pressed.take().is_some();
        let was_dragging = self.dragging.take().is_some();
        let was_scroll = self.scroll_drag.take().is_some();
        let was_split = self.split_drag.take().is_some();
        let was_range = self.range_drag.take().is_some();
        let was_knob = self.knob_drag.take().is_some();
        let was_color = self.color_drag.take().is_some();
        let canvas_up = self.canvas_drag.take();
        let was_pan = self.canvas_pan.take().is_some() | self.tree_bar.take().is_some();
        let was_selecting = self.text_selecting;
        self.text_selecting = false;
        if let Some(id) = click_id {
            self.dispatch(id);
        }
        let mut canvas_fired = false;
        if let Some(id) = canvas_up {
            let (mx, my) = self.mouse;
            let i = self.tree.canvas_hit(id, mx, my).map_or(-1, |v| v as i32);
            let (lx, ly) = self.tree.canvas_local(id, mx, my);
            canvas_fired = self.tree.fire_point(id, 2, i, lx, ly);
        }
        was_pressed
            || was_dragging
            || was_scroll
            || was_split
            || was_range
            || was_knob
            || was_color
            || was_selecting
            || canvas_fired
            || was_pan
            || click_id.is_some()
    }

    /// Обрабатывает символьный ввод. Возвращает true, если нужна перерисовка.
    pub fn on_char(&mut self, ch: u16) -> bool {
        const BACKSPACE: u16 = 0x08;
        if let Some(id) = self.focused {
            if self.tree.is_term(id) {
                if ch == 0x0D {
                    if let Some(cmd) = self.tree.term_take_input(id) {
                        let echo: Vec<u16> = format!("$ {}", cmd).encode_utf16().collect();
                        self.tree.term_push(id, echo);
                        self.tree.fire_input_text(id, &cmd);
                    }
                    return true;
                }
                if let Some(st) = self.tree.term_input_mut(id) {
                    if ch == BACKSPACE {
                        st.backspace();
                        return true;
                    }
                    if ch >= 0x20 {
                        st.insert(&[ch]);
                        return true;
                    }
                }
                return false;
            }
            let mut changed = false;
            if let Some(st) = self.tree.textbox_state_mut(id) {
                if ch == BACKSPACE {
                    st.backspace();
                    changed = true;
                } else if ch >= 0x20 {
                    st.insert(&[ch]);
                    changed = true;
                }
            }
            if changed {
                self.tree.fire_text_input(id);
                return true;
            }
        }
        false
    }

    /// Вставляет текст, зафиксированный IME, в поле ввода.
    pub fn on_ime_text(&mut self, text: &[u16]) -> bool {
        if let Some(id) = self.focused {
            if self.tree.is_textbox(id) {
                let filtered: Vec<u16> = text.iter().copied().filter(|&c| c >= 0x20).collect();
                if !filtered.is_empty() {
                    if let Some(st) = self.tree.textbox_state_mut(id) {
                        st.insert(&filtered);
                    }
                    self.tree.fire_text_input(id);
                    return true;
                }
            }
        }
        false
    }

    /// Позиция каретки активного поля в клиентских координатах.
    pub fn ime_caret(&self) -> Option<(f32, f32)> {
        let id = self.focused?;
        if !self.tree.is_textbox(id) {
            return None;
        }
        let r = self.tree.get(id).rect;
        let st = self.tree.textbox_state(id)?;
        let pad = 12.0;
        let base_x = r.x + pad - st.scroll;
        let cx = base_x + x_at_index(&self.dwrite, &self.text_format_left, &st.text, st.caret);
        let cy = r.y + r.height - 10.0;
        Some((cx, cy))
    }

    /// Обрабатывает нажатие клавиши. Возвращает true, если нужна перерисовка.
    pub fn on_key(&mut self, vk: u32) -> bool {
        if self.dialog.is_some() {
            if vk == 0x1B {
                self.dialog = None;
                self.tree.fire_dialog(-1);
                return true;
            }
            if vk == 0x09 {
                let shift = key_down(0x10);
                if let Some(d) = self.dialog.as_mut() {
                    let n = d.buttons.len();
                    if n > 0 {
                        d.focus = Some(match d.focus {
                            Some(i) => {
                                if shift {
                                    (i + n - 1) % n
                                } else {
                                    (i + 1) % n
                                }
                            }
                            None => {
                                if shift {
                                    n - 1
                                } else {
                                    0
                                }
                            }
                        });
                    }
                }
                return true;
            }
            if vk == 0x0D || vk == 0x20 {
                let idx = self.dialog.as_ref().and_then(|d| {
                    d.focus.or(if d.buttons.is_empty() {
                        None
                    } else {
                        Some(d.buttons.len() - 1)
                    })
                });
                self.dialog = None;
                match idx {
                    Some(i) => self.tree.fire_dialog(i as i32),
                    None => self.tree.fire_dialog(-1),
                }
                return true;
            }
            return true;
        }
        if vk == 0x1B {
            if let Some(layer) = self.tree.close_popup() {
                self.focused = None;
                self.tree.fire_point(layer, 8, 0, 0.0, 0.0);
                return true;
            }
        }
        if self.open_menu.is_none()
            && self.open_dropdown.is_none()
            && self.focused.map_or(true, |id| !self.tree.is_textbox(id))
        {
            let mut mods = 0u8;
            if key_down(0x11) {
                mods |= 1;
            }
            if key_down(0x10) {
                mods |= 2;
            }
            if key_down(0x12) {
                mods |= 4;
            }
            if self.tree.fire_hotkey(mods, vk) {
                return true;
            }
        }
        if self.open_menu.is_none() && self.open_dropdown.is_none() {
            if let Some(id) = self.focused {
                if self.tree.is_tree(id) {
                    let step = match vk {
                        0x26 => Some(-1),
                        0x28 => Some(1),
                        0x21 => Some(-8),
                        0x22 => Some(8),
                        _ => None,
                    };
                    if let Some(s) = step {
                        if let Some(next) = self.tree.tree_step(id, s) {
                            self.select_tree_row(id, next);
                        }
                        return true;
                    }
                    if vk == 0x24 || vk == 0x23 {
                        if let Some(next) = self.tree.tree_edge(id, vk == 0x23) {
                            self.select_tree_row(id, next);
                        }
                        return true;
                    }
                    if vk == 0x27 {
                        if let Some(cur) = self.tree.tree_selected(id) {
                            if !self.tree.tree_leaf(id, cur) && !self.tree.tree_open(id, cur) {
                                self.tree.open_row(id, cur, true);
                            } else if let Some(next) = self.tree.tree_step(id, 1) {
                                self.select_tree_row(id, next);
                            }
                        }
                        return true;
                    }
                    if vk == 0x25 {
                        if let Some(cur) = self.tree.tree_selected(id) {
                            if !self.tree.tree_leaf(id, cur) && self.tree.tree_open(id, cur) {
                                self.tree.open_row(id, cur, false);
                            } else if let Some(p) = self.tree.tree_parent(id, cur) {
                                self.select_tree_row(id, p);
                            }
                        }
                        return true;
                    }
                }
            }
        }
        if self.open_menu.is_some() {
            let n = self.tree.menu_len();
            match vk {
                0x1B => {
                    self.open_menu = None;
                    self.menu_hover = None;
                    return true;
                }
                0x26 => {
                    let cur = self.menu_hover.unwrap_or(0);
                    self.menu_hover = Some(if cur == 0 { 0 } else { cur - 1 });
                    return true;
                }
                0x28 => {
                    let cur = self.menu_hover.unwrap_or(0);
                    let next = if n == 0 { 0 } else { (cur + 1).min(n - 1) };
                    self.menu_hover = Some(next);
                    return true;
                }
                0x0D | 0x20 => {
                    if let Some(i) = self.menu_hover {
                        let root = self.tree.root();
                        self.tree.fire_change(root, i as f32);
                    }
                    self.open_menu = None;
                    self.menu_hover = None;
                    return true;
                }
                _ => return true,
            }
        }
        const VK_TAB: u32 = 0x09;
        const VK_RETURN: u32 = 0x0D;
        const VK_SPACE: u32 = 0x20;
        const VK_END: u32 = 0x23;
        const VK_HOME: u32 = 0x24;
        const VK_LEFT: u32 = 0x25;
        const VK_RIGHT: u32 = 0x27;
        const VK_DELETE: u32 = 0x2E;
        const KEY_A: u32 = 0x41;
        const KEY_Y: u32 = 0x59;
        const KEY_Z: u32 = 0x5A;
        const KEY_C: u32 = 0x43;
        const KEY_V: u32 = 0x56;
        const KEY_X: u32 = 0x58;
        const VK_F12: u32 = 0x7B;
        const VK_UP: u32 = 0x26;
        const VK_DOWN: u32 = 0x28;
        const VK_ESCAPE: u32 = 0x1B;

        if vk == VK_F12 {
            if !self.tree.inspect() {
                return false;
            }
            self.inspector = !self.inspector;
            return true;
        }

        if vk == VK_TAB {
            self.close_dropdown();
            self.move_focus(!key_down(0x10));
            return true;
        }

        if vk == VK_ESCAPE {
            self.close_dropdown();
            self.open_menu = None;
            self.menu_hover = None;
            self.popup = None;
            let was = self.focused.take();
            if let Some(id) = was {
                if self.tree.is_textbox(id) {
                    self.tree.fire_change(id, 0.0);
                }
            }
            return true;
        }

        if let Some(id) = self.focused {
            if vk == VK_RETURN && self.tree.is_textbox(id) && !self.tree.is_multiline(id) {
                self.tree.fire_change(id, 1.0);
                return true;
            }
            if self.tree.is_dropdown(id) {
                let n = self.tree.dropdown_len(id);
                if self.open_dropdown == Some(id) {
                    match vk {
                        VK_UP => {
                            let cur = self.dropdown_hover.unwrap_or(0);
                            self.dropdown_hover = Some(if cur == 0 { 0 } else { cur - 1 });
                            return true;
                        }
                        VK_DOWN => {
                            let cur = self.dropdown_hover.unwrap_or(0);
                            let next = if n == 0 { 0 } else { (cur + 1).min(n - 1) };
                            self.dropdown_hover = Some(next);
                            return true;
                        }
                        VK_RETURN | VK_SPACE => {
                            if let Some(hv) = self.dropdown_hover {
                                self.tree.set_dropdown_selected(id, hv);
                                self.tree.fire_change(id, hv as f32);
                            }
                            self.close_dropdown();
                            return true;
                        }
                        VK_ESCAPE => {
                            self.close_dropdown();
                            return true;
                        }
                        _ => return false,
                    }
                } else if vk == VK_RETURN || vk == VK_SPACE {
                    self.close_dropdown();
                    let sel = self.tree.dropdown_selected(id);
                    self.tree.set_dropdown_open(id, true);
                    self.open_dropdown = Some(id);
                    self.dropdown_hover = Some(sel);
                    return true;
                }
                return false;
            }
            if self.tree.is_tabs(id) {
                let n = self.tree.tabs_len(id);
                if n > 0 && (vk == VK_LEFT || vk == VK_RIGHT) {
                    let cur = self.tree.tabs_selected(id);
                    let next = if vk == VK_LEFT {
                        if cur == 0 { 0 } else { cur - 1 }
                    } else {
                        (cur + 1).min(n - 1)
                    };
                    if next != cur {
                        self.tree.set_tabs_selected(id, next);
                        self.tree.fire_change(id, next as f32);
                    }
                    return true;
                }
                return false;
            }
            if self.tree.is_table(id) {
                let n = self.tree.table_len(id);
                if n > 0 && (vk == VK_UP || vk == VK_DOWN) {
                    let cur = self.tree.table_selected(id).unwrap_or(0);
                    let next = if vk == VK_UP {
                        if cur == 0 { 0 } else { cur - 1 }
                    } else {
                        (cur + 1).min(n - 1)
                    };
                    self.tree.set_table_selected(id, Some(next));
                    self.reveal_table_row(id, next);
                    self.tree.fire_change(id, next as f32);
                    return true;
                }
                return false;
            }
            if self.tree.is_list(id) {
                let n = self.tree.list_len(id);
                if n > 0 && (vk == VK_UP || vk == VK_DOWN) {
                    let cur = self.tree.list_selected(id).unwrap_or(0);
                    let next = if vk == VK_UP {
                        if cur == 0 { 0 } else { cur - 1 }
                    } else {
                        (cur + 1).min(n - 1)
                    };
                    self.tree.set_list_selected(id, Some(next));
                    let vis = self.tree.get(id).rect.height.max(0.0);
                    let top = next as f32 * LIST_ROW;
                    let mut ns = self.tree.list_scroll(id);
                    if top < ns {
                        ns = top;
                    }
                    if top + LIST_ROW > ns + vis {
                        ns = top + LIST_ROW - vis;
                    }
                    self.tree.set_list_scroll(id, ns.max(0.0));
                    if self.tree.is_list_msel(id) {
                        self.tree.set_list_multi(id, vec![next]);
                        self.tree.fire_point(id, 7, next as i32, 0.0, 0.0);
                    } else {
                        self.tree.fire_change(id, next as f32);
                    }
                    return true;
                }
                return false;
            }
            if self.tree.is_textbox(id) {
                let shift = key_down(0x10);
                let ctrl = key_down(0x11);
                let mut handled = false;
                let mut changed = false;
                let multi = self.tree.is_multiline(id);
                if multi && (vk == 0x26 || vk == 0x28) {
                    let rect = self.tree.get(id).rect;
                    let (text, caret) = match self.tree.textbox_state(id) {
                        Some(s) => (s.text.clone(), s.caret),
                        None => (Vec::new(), 0),
                    };
                    let width = (rect.width - 20.0).max(1.0);
                    let (cx, cy, lh) =
                        wrapped_caret(&self.dwrite, &self.text_format_wrap, &text, width, caret);
                    let ny = if vk == 0x26 { cy - lh * 0.5 } else { cy + lh * 1.5 };
                    let idx = wrapped_index(
                        &self.dwrite,
                        &self.text_format_wrap,
                        &text,
                        width,
                        cx,
                        ny.max(0.0),
                    );
                    if let Some(st) = self.tree.textbox_state_mut(id) {
                        st.set_caret(idx, shift);
                    }
                    handled = true;
                }
                if let Some(st) = self.tree.textbox_state_mut(id) {
                    match vk {
                        VK_LEFT => {
                            st.move_left(shift);
                            handled = true;
                        }
                        VK_RIGHT => {
                            st.move_right(shift);
                            handled = true;
                        }
                        VK_HOME => {
                            st.home(shift);
                            handled = true;
                        }
                        VK_END => {
                            st.end(shift);
                            handled = true;
                        }
                        VK_DELETE => {
                            st.delete_forward();
                            handled = true;
                            changed = true;
                        }
                        0x0D if multi => {
                            st.insert(&[0x0A]);
                            handled = true;
                            changed = true;
                        }
                        _ => {}
                    }
                    if !handled && ctrl {
                        match vk {
                            KEY_A => {
                                st.select_all();
                                handled = true;
                            }
                            KEY_Z => {
                                st.undo();
                                handled = true;
                                changed = true;
                            }
                            KEY_Y => {
                                st.redo();
                                handled = true;
                                changed = true;
                            }
                            KEY_C => {
                                let (a, b) = st.sel_range();
                                if a != b {
                                    let sel = to_crlf(&st.text[a..b]);
                                    set_clipboard_text(&sel);
                                }
                                handled = true;
                            }
                            KEY_X => {
                                let (a, b) = st.sel_range();
                                if a != b {
                                    let sel = to_crlf(&st.text[a..b]);
                                    set_clipboard_text(&sel);
                                    st.backspace();
                                    changed = true;
                                }
                                handled = true;
                            }
                            KEY_V => {
                                let clip = get_clipboard_text();
                                let filtered: Vec<u16> = clip
                                    .into_iter()
                                    .filter(|&c| c != 0x0D)
                                    .filter(|&c| c >= 0x20 || (multi && c == 0x0A))
                                    .collect();
                                if !filtered.is_empty() {
                                    st.insert(&filtered);
                                    changed = true;
                                }
                                handled = true;
                            }
                            _ => {}
                        }
                    }
                }
                if changed {
                    self.tree.fire_text_input(id);
                }
                return handled;
            }
            if self.tree.is_interactive(id) {
                if vk == VK_RETURN || vk == VK_SPACE {
                    self.dispatch(id);
                    return true;
                }
                return false;
            }
            if self.tree.is_slider(id) {
                if vk == VK_LEFT || vk == VK_RIGHT {
                    let cur = match &self.tree.get(id).kind {
                        NodeKind::Slider { value } => *value,
                        _ => 0.0,
                    };
                    let step = if vk == VK_LEFT { -0.05 } else { 0.05 };
                    let v = (cur + step).clamp(0.0, 1.0);
                    self.tree.set_slider_value(id, v);
                    self.tree.fire_change(id, v);
                    return true;
                }
                return false;
            }
        }

        false
    }

    fn move_focus(&mut self, forward: bool) {
        let list = self.tree.focusables();
        if list.is_empty() {
            self.focused = None;
            return;
        }
        let idx = self.focused.and_then(|f| list.iter().position(|&x| x == f));
        let next = match idx {
            Some(i) => {
                if forward {
                    (i + 1) % list.len()
                } else {
                    (i + list.len() - 1) % list.len()
                }
            }
            None => {
                if forward {
                    0
                } else {
                    list.len() - 1
                }
            }
        };
        self.focused = Some(list[next]);
        self.text_selecting = false;
        self.focus_ring = true;
    }

    fn dispatch(&mut self, id: NodeId) {
        if self.tree.is_accordion(id) {
            self.tree.toggle_acc(id);
            return;
        }
        if self.tree.is_switch(id) {
            self.tree.toggle_switch(id);
            let on = self.tree.switch_on(id);
            self.tree.fire_change(id, if on { 1.0 } else { 0.0 });
            return;
        }
        if self.tree.is_radio(id) {
            self.tree.select_radio(id);
            self.tree.fire_change(id, 1.0);
            return;
        }
        if self.tree.is_toggle(id) {
            self.tree.flip_toggle(id);
            let on = self.tree.toggle_on(id);
            self.tree.fire_change(id, if on { 1.0 } else { 0.0 });
            return;
        }
        if self.tree.is_checkbox(id) {
            self.tree.toggle_checkbox(id);
        }
        self.tree.fire_click(id);
    }

    fn set_slider_from_x(&mut self, id: NodeId, x: f32) {
        let rect = self.tree.get(id).rect;
        if rect.width <= 0.0 {
            return;
        }
        let value = ((x - rect.x) / rect.width).clamp(0.0, 1.0);
        self.tree.set_slider_value(id, value);
        self.tree.fire_change(id, value);
    }

    fn color_zones(&self, id: NodeId) -> (Rect, Rect) {
        let r = self.tree.get(id).rect;
        let pad = 8.0;
        let strip_h = 18.0;
        let sw = 46.0;
        let area = Rect::new(
            r.x + pad,
            r.y + pad,
            (r.width - 3.0 * pad - sw).max(1.0),
            (r.height - 3.0 * pad - strip_h).max(1.0),
        );
        let strip = Rect::new(area.x, area.y + area.height + pad, area.width, strip_h);
        (area, strip)
    }

    fn set_color_from(&mut self, id: NodeId, mode: u8, x: f32, y: f32) {
        let (area, strip) = self.color_zones(id);
        let (h, s, v) = self.tree.color_hsv(id);
        let (nh, ns, nv) = if mode == 1 {
            let t = ((x - strip.x) / strip.width).clamp(0.0, 1.0);
            (t, s, v)
        } else {
            let ns = ((x - area.x) / area.width).clamp(0.0, 1.0);
            let nv = 1.0 - ((y - area.y) / area.height).clamp(0.0, 1.0);
            (h, ns, nv)
        };
        self.tree.set_color_hsv(id, nh, ns, nv);
        self.tree.fire_change(id, color_code(nh, ns, nv));
    }

    fn set_range_from_x(&mut self, id: NodeId, upper: bool, x: f32) {
        let rect = self.tree.get(id).rect;
        if rect.width <= 0.0 {
            return;
        }
        let t = ((x - rect.x) / rect.width).clamp(0.0, 1.0);
        let (lo, hi) = self.tree.range_values(id);
        let (a, b) = if upper { (lo.min(t), t) } else { (t, hi.max(t)) };
        self.tree.set_range(id, a, b);
        self.tree.fire_change(id, t);
    }

    fn update_scroll(&mut self) {
        if let Some(id) = self.focused {
            if self.tree.is_textbox(id) && !self.tree.is_multiline(id) {
                let rect = self.tree.get(id).rect;
                let avail = (rect.width - 24.0).max(0.0);
                let (text, caret) = match self.tree.textbox_state(id) {
                    Some(s) => (s.text.clone(), s.caret),
                    None => return,
                };
                let caret_px = x_at_index(&self.dwrite, &self.text_format_left, &text, caret);
                if let Some(s) = self.tree.textbox_state_mut(id) {
                    if caret_px - s.scroll < 0.0 {
                        s.scroll = caret_px;
                    }
                    if caret_px - s.scroll > avail {
                        s.scroll = caret_px - avail;
                    }
                    if s.scroll < 0.0 {
                        s.scroll = 0.0;
                    }
                }
            }
        }
    }

    /// Пересчитывает раскладку и перерисовывает окно из дерева элементов.
    fn preload_images(&mut self) {
        let mut paths: Vec<String> = Vec::new();
        self.tree.for_each(|_, node| {
            if let NodeKind::Image { path, .. } = &node.kind {
                if !path.is_empty() {
                    paths.push(path.clone());
                }
            }
            if let Some(icon) = &node.icon {
                if !icon.is_empty() {
                    paths.push(icon.clone());
                }
            }
        });
        self.tree.tree_icons(&mut paths);
        if paths.is_empty() {
            return;
        }
        if self.wic.is_none() {
            self.wic = unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()
            };
        }
        let wic = match &self.wic {
            Some(w) => w.clone(),
            None => return,
        };
        for p in paths {
            if self.img_cache.contains_key(&p) {
                continue;
            }
            let bmp = if p.starts_with("mem:") {
                match self.tree.image_bytes(&p) {
                    Some(mut data) => load_bitmap_mem(&wic, &self.rt, &mut data),
                    None => None,
                }
            } else {
                load_bitmap(&wic, &self.rt, &p)
            };
            self.img_cache.insert(p, bmp);
        }
    }

    fn poll_toast(&mut self) {
        if let Some((text, secs)) = self.tree.take_toast() {
            self.toast = Some((text, Instant::now(), secs));
        }
        if let Some((_, at, secs)) = &self.toast {
            if at.elapsed().as_secs_f32() > *secs {
                self.toast = None;
            }
        }
    }

    fn poll_notes(&mut self) {
        for data in self.tree.take_notes() {
            self.notes.push(NoteView {
                title: data.title,
                text: data.text,
                action: data.action,
                kind: data.kind,
                corner: data.corner,
                secs: data.secs,
                born: Instant::now(),
                cb: data.cb,
            });
        }
        self.notes
            .retain(|n| n.secs <= 0.0 || n.born.elapsed().as_secs_f32() < n.secs);
    }

    fn note_rects(&self) -> Vec<(usize, Rect, Rect)> {
        let mut out = Vec::new();
        let mut stack = [0usize; 5];
        for i in 0..self.notes.len() {
            let kind = self.notes[i].kind;
            let corner = self.notes[i].corner.min(4);
            let (r, act) = if kind == 1 && corner == 4 {
                let w = 460.0f32.min(self.width - 40.0);
                let h = 56.0;
                let r = Rect::new(
                    (self.width - w) / 2.0,
                    self.height - 28.0 - h - stack[4] as f32 * (h + 10.0),
                    w,
                    h,
                );
                stack[4] += 1;
                let act = Rect::new(r.x + r.width - 110.0, r.y + 10.0, 96.0, h - 20.0);
                (r, act)
            } else {
                let w = 320.0f32.min(self.width - 40.0);
                let h = if kind == 1 { 56.0 } else { 76.0 };
                let slot = if corner == 4 { 1 } else { corner } as usize;
                let step = stack[slot] as f32 * (h + 10.0);
                let x = if slot == 0 || slot == 2 {
                    20.0
                } else {
                    self.width - w - 20.0
                };
                let y = if slot <= 1 {
                    20.0 + step
                } else {
                    self.height - 20.0 - h - step
                };
                stack[slot] += 1;
                let r = Rect::new(x, y, w, h);
                let act = if kind == 1 {
                    Rect::new(r.x + r.width - 110.0, r.y + 10.0, 96.0, h - 20.0)
                } else {
                    Rect::new(r.x + r.width - 100.0, r.y + h - 34.0, 86.0, 26.0)
                };
                (r, act)
            };
            out.push((i, r, act));
        }
        out
    }

    fn poll_dialog(&mut self) {
        self.tree.poll_css();
        if self.dialog.is_some() {
            return;
        }
        if let Some(data) = self.tree.take_pending_dialog() {
            self.tree.set_dialog_cb(data.cb);
            self.dialog = Some(DialogView {
                title: data.title,
                message: data.message,
                buttons: data.buttons,
                hover: None,
                focus: None,
                msg_scroll: 0.0,
            });
        }
    }

    pub fn render(&mut self) {
        if let Some(i) = self.tree.take_pending_theme() {
            self.theme_index = i;
            self.theme = theme_from_index(i);
        }
        if self.tree.take_pending_font() {
            self.rebuild_fonts();
        }
        if let Some((target, text, all)) = self.tree.take_pending_focus() {
            self.focused = target;
            if let Some(id) = target {
                if self.tree.is_textbox(id) {
                    if let Some(st) = self.tree.textbox_state_mut(id) {
                        if let Some(t) = &text {
                            st.select_all();
                            st.insert(t);
                        }
                        if all {
                            st.select_all();
                        }
                    }
                }
            }
        }
        self.poll_dialog();
        self.poll_toast();
        self.poll_notes();
        self.pump();
        if self.tree.take_img_dirty() {
            self.preload_images();
        }
        self.tree.layout(Rect::new(0.0, 0.0, self.width, self.height));
        self.tree.publish_rects();
        self.update_scroll();
        self.preload_images();
        let hovered = self.hovered;
        let pressed = self.pressed;
        let focused = self.focused;
        let ring = if self.focus_ring { self.focused } else { None };
        let hot = self.hot;
        let mouse = self.mouse;
        let theme = self.theme;
        if let Some(h) = self.frame_latency {
            unsafe {
                windows::Win32::System::Threading::WaitForSingleObject(h, 1000);
            }
        }
        unsafe {
            self.rt.BeginDraw();
        }
        {
            let canvas = Canvas::new(&self.rt, &self.grad_cache, &self.layout_cache, &self.dwrite);
            let format = &self.text_format;
            let format_left = &self.text_format_left;
            let format_wrap = &self.text_format_wrap;
            let img_cache = &self.img_cache;
            let dwrite = &self.dwrite;
            let fmt_cache = &self.fmt_cache;
            let view_w = self.width;
            let view_h = self.height;
            let clear = if self.glass {
                Color::rgba(
                    theme.background.r,
                    theme.background.g,
                    theme.background.b,
                    self.tree.tint(),
                )
            } else {
                theme.background
            };
            canvas.clear(clear);
            let clip_map = self.tree.clip_map();
            let flag_map = self.tree.flag_map();
            self.tree.for_each(|id, node| {
                if node.rect.x <= OFF_LIMIT || node.rect.y <= OFF_LIMIT {
                    return;
                }
                let r = node.rect;
                let m = 48.0;
                if r.x + r.width < -m
                    || r.y + r.height < -m
                    || r.x > view_w + m
                    || r.y > view_h + m
                {
                    return;
                }
                let outer = clip_map.get(id.index()).copied().flatten();
                if let Some(c) = outer {
                    if c.width <= 0.0
                        || c.height <= 0.0
                        || r.x > c.x + c.width
                        || r.y > c.y + c.height
                        || r.x + r.width < c.x
                        || r.y + r.height < c.y
                    {
                        return;
                    }
                    canvas.push_clip(c);
                }
                let flags = flag_map.get(id.index()).copied().unwrap_or(0);
                let is_off = flags & 1 != 0;
                let is_pwd = flags & 2 != 0;
                let mut style = node.style;
                if focused == Some(id) && !is_off {
                    merge_style(&mut style, &node.style_focus);
                }
                let is_button = matches!(node.kind, NodeKind::Button { .. });
                if hot == Some(id) && !is_button && !is_off {
                    merge_style(&mut style, &node.style_hover);
                }
                if is_off {
                    style.text = Some(theme.track);
                    if !matches!(node.kind, NodeKind::Container) {
                        style.fill = Some(theme.track);
                    }
                }
                let (cf, cl, cw);
                let (format, format_left, format_wrap) =
                    if style.font.is_some() || style.size.is_some() {
                        cf = pick_format(dwrite, fmt_cache, style, 0, DWRITE_FONT_WEIGHT_SEMI_BOLD, 24.0);
                        cl = pick_format(dwrite, fmt_cache, style, 1, DWRITE_FONT_WEIGHT_NORMAL, 20.0);
                        cw = pick_format(dwrite, fmt_cache, style, 2, DWRITE_FONT_WEIGHT_NORMAL, 20.0);
                        (&cf, &cl, &cw)
                    } else {
                        (format, format_left, format_wrap)
                    };
                match &node.kind {
                    NodeKind::Container => {}
                    NodeKind::Image { path, fit } => {
                        if let Some(Some(bmp)) = img_cache.get(path) {
                            canvas.draw_bitmap(bmp, node.rect, *fit);
                        }
                    }
                    NodeKind::Frame { radius } => {
                        let fill = style.fill.unwrap_or(theme.surface);
                        let rad = style.radius.unwrap_or(*radius);
                        if let Some(e) = style.elev {
                            if e > 0.0 {
                                draw_soft_shadow(&canvas, node.rect, rad, e);
                            }
                        }
                        if let Some((a, b)) = style.grad {
                            canvas.fill_rounded_gradient(node.rect, rad, a, b, style.grad_dir);
                        } else {
                            canvas.fill_rounded_rect(node.rect, rad, fill);
                        }
                    }
                    NodeKind::Label { text } => {
                        let color = style.text.unwrap_or(theme.content);
                        let mut tr = node.rect;
                        let mut fmt = if style.wrap == Some(true) {
                            format_wrap
                        } else {
                            format
                        };
                        if let Some(icon) = &node.icon {
                            if let Some(Some(bmp)) = img_cache.get(icon) {
                                let sz = (node.rect.height - 6.0).clamp(12.0, 28.0);
                                let iy = node.rect.y + (node.rect.height - sz) / 2.0;
                                canvas.draw_bitmap(bmp, Rect::new(node.rect.x, iy, sz, sz), 0);
                                let off = sz + 8.0;
                                tr = Rect::new(
                                    node.rect.x + off,
                                    node.rect.y,
                                    (node.rect.width - off).max(0.0),
                                    node.rect.height,
                                );
                                if style.wrap != Some(true) {
                                    fmt = format_left;
                                }
                            }
                        }
                        canvas.draw_text(text, fmt, tr, color);
                    }
                    NodeKind::Button { label, radius } => {
                        let rad = style.radius.unwrap_or(*radius);
                        let icon_only = label.is_empty() && node.icon.is_some();
                        if !icon_only {
                            if let Some(e) = style.elev {
                                if e > 0.0 {
                                    draw_soft_shadow(&canvas, node.rect, rad, e);
                                }
                            }
                        }
                        let (base, hov, prs) = match style.fill {
                            Some(f) => (f, f.lighten(0.1), f.darken(0.1)),
                            None => (theme.accent, theme.accent_hover, theme.accent_pressed),
                        };
                        let hov = node.style_hover.fill.unwrap_or(hov);
                        let fill = if pressed == Some(id) {
                            prs
                        } else if hovered == Some(id) {
                            hov
                        } else {
                            base
                        };
                        let text_color = style.text.unwrap_or(theme.on_accent);
                        if !icon_only {
                            if focused == Some(id) {
                                let ring = Rect::new(
                                    node.rect.x - 3.0,
                                    node.rect.y - 3.0,
                                    node.rect.width + 6.0,
                                    node.rect.height + 6.0,
                                );
                                canvas.fill_rounded_rect(ring, rad + 3.0, theme.content);
                            }
                            if let Some((a, b)) = style.grad {
                                canvas.fill_rounded_gradient(node.rect, rad, a, b, style.grad_dir);
                            } else {
                                canvas.fill_rounded_rect(node.rect, rad, fill);
                            }
                        }
                        let mut tr = node.rect;
                        if let Some(icon) = &node.icon {
                            if let Some(Some(bmp)) = img_cache.get(icon) {
                                if icon_only {
                                    let pad = 4.0;
                                    let r = Rect::new(
                                        node.rect.x + pad,
                                        node.rect.y + pad,
                                        (node.rect.width - pad * 2.0).max(0.0),
                                        (node.rect.height - pad * 2.0).max(0.0),
                                    );
                                    canvas.draw_bitmap(bmp, r, 0);
                                } else {
                                    let sz = (node.rect.height - 10.0).clamp(12.0, 26.0);
                                    let iy = node.rect.y + (node.rect.height - sz) / 2.0;
                                    let ix = node.rect.x + 12.0;
                                    canvas.draw_bitmap(bmp, Rect::new(ix, iy, sz, sz), 0);
                                    let off = sz + 20.0;
                                    tr = Rect::new(
                                        node.rect.x + off,
                                        node.rect.y,
                                        (node.rect.width - off).max(0.0),
                                        node.rect.height,
                                    );
                                }
                            }
                        }
                        if !icon_only {
                            canvas.draw_text(label, format, tr, text_color);
                        }
                    }
                    NodeKind::Slider { value } => {
                        let v = value.clamp(0.0, 1.0);
                        let r = node.rect;
                        let track_h = 6.0;
                        let cy = r.y + r.height / 2.0;
                        let track = Rect::new(r.x, cy - track_h / 2.0, r.width, track_h);
                        canvas.fill_rounded_rect(track, track_h / 2.0, theme.track);
                        let filled_w = r.width * v;
                        let fill = style.fill.unwrap_or(theme.accent);
                        let filled = Rect::new(r.x, cy - track_h / 2.0, filled_w, track_h);
                        canvas.fill_rounded_rect(filled, track_h / 2.0, fill);
                        let knob_d = 16.0;
                        let hi = (r.x + r.width - knob_d).max(r.x);
                        let knob_x = (r.x + filled_w - knob_d / 2.0).clamp(r.x, hi);
                        let knob = Rect::new(knob_x, cy - knob_d / 2.0, knob_d, knob_d);
                        let knob_color = if ring == Some(id) { theme.accent } else { theme.content };
                        canvas.fill_rounded_rect(knob, knob_d / 2.0, knob_color);
                    }
                    NodeKind::Progress { value } => {
                        let v = value.clamp(0.0, 1.0);
                        let r = node.rect;
                        let bar_h = 10.0;
                        let cy = r.y + r.height / 2.0;
                        let track = Rect::new(r.x, cy - bar_h / 2.0, r.width, bar_h);
                        canvas.fill_rounded_rect(track, bar_h / 2.0, theme.track);
                        let fill = style.fill.unwrap_or(theme.accent);
                        let filled = Rect::new(r.x, cy - bar_h / 2.0, r.width * v, bar_h);
                        canvas.fill_rounded_rect(filled, bar_h / 2.0, fill);
                    }
                    NodeKind::Checkbox { label, checked } => {
                        let r = node.rect;
                        let box_d = 22.0;
                        let bx = r.x;
                        let by = r.y + (r.height - box_d) / 2.0;
                        let box_rect = Rect::new(bx, by, box_d, box_d);
                        if focused == Some(id) {
                            let ring = Rect::new(bx - 3.0, by - 3.0, box_d + 6.0, box_d + 6.0);
                            canvas.fill_rounded_rect(ring, 8.0, theme.content);
                        }
                        if *checked {
                            let fill = style.fill.unwrap_or(theme.accent);
                            canvas.fill_rounded_rect(box_rect, 5.0, fill);
                            let check: Vec<u16> = "\u{2713}".encode_utf16().collect();
                            canvas.draw_text(&check, format, box_rect, theme.on_accent);
                        } else {
                            canvas.fill_rounded_rect(box_rect, 5.0, theme.track);
                            let inner = Rect::new(bx + 2.0, by + 2.0, box_d - 4.0, box_d - 4.0);
                            canvas.fill_rounded_rect(inner, 4.0, theme.surface);
                        }
                        let label_rect = Rect::new(
                            bx + box_d + 10.0,
                            r.y,
                            (r.width - box_d - 10.0).max(0.0),
                            r.height,
                        );
                        let color = style.text.unwrap_or(theme.content);
                        canvas.draw_text(label, format_left, label_rect, color);
                    }
                    NodeKind::Switch { label, on } => {
                        let r = node.rect;
                        let tw = 44.0;
                        let th = 24.0;
                        let tx = r.x;
                        let ty = r.y + (r.height - th) / 2.0;
                        let track = Rect::new(tx, ty, tw, th);
                        if focused == Some(id) {
                            let ring = Rect::new(tx - 3.0, ty - 3.0, tw + 6.0, th + 6.0);
                            canvas.fill_rounded_rect(ring, th / 2.0 + 3.0, theme.content);
                        }
                        let track_col = if *on {
                            style.fill.unwrap_or(theme.accent)
                        } else {
                            theme.track
                        };
                        canvas.fill_rounded_rect(track, th / 2.0, track_col);
                        let kd = th - 6.0;
                        let kx = if *on { tx + tw - kd - 3.0 } else { tx + 3.0 };
                        let ky = ty + 3.0;
                        canvas.fill_rounded_rect(
                            Rect::new(kx, ky, kd, kd),
                            kd / 2.0,
                            theme.on_accent,
                        );
                        let label_rect = Rect::new(
                            tx + tw + 10.0,
                            r.y,
                            (r.width - tw - 10.0).max(0.0),
                            r.height,
                        );
                        let color = style.text.unwrap_or(theme.content);
                        canvas.draw_text(label, format_left, label_rect, color);
                    }
                    NodeKind::Radio { label, on, .. } => {
                        let r = node.rect;
                        let d = 22.0;
                        let bx = r.x;
                        let by = r.y + (r.height - d) / 2.0;
                        let outer = Rect::new(bx, by, d, d);
                        if focused == Some(id) {
                            let ring = Rect::new(bx - 3.0, by - 3.0, d + 6.0, d + 6.0);
                            canvas.fill_rounded_rect(ring, d / 2.0 + 3.0, theme.content);
                        }
                        let border = if *on {
                            style.fill.unwrap_or(theme.accent)
                        } else {
                            theme.track
                        };
                        canvas.fill_rounded_rect(outer, d / 2.0, border);
                        let inner = Rect::new(bx + 2.0, by + 2.0, d - 4.0, d - 4.0);
                        canvas.fill_rounded_rect(inner, (d - 4.0) / 2.0, theme.surface);
                        if *on {
                            let dot = 10.0;
                            let dx = bx + (d - dot) / 2.0;
                            let dy = by + (d - dot) / 2.0;
                            canvas.fill_rounded_rect(
                                Rect::new(dx, dy, dot, dot),
                                dot / 2.0,
                                style.fill.unwrap_or(theme.accent),
                            );
                        }
                        let label_rect = Rect::new(
                            bx + d + 10.0,
                            r.y,
                            (r.width - d - 10.0).max(0.0),
                            r.height,
                        );
                        let color = style.text.unwrap_or(theme.content);
                        canvas.draw_text(label, format_left, label_rect, color);
                    }
                    NodeKind::Toggle { label, on } => {
                        let r = node.rect;
                        let radius = 10.0;
                        if focused == Some(id) {
                            let ring =
                                Rect::new(r.x - 3.0, r.y - 3.0, r.width + 6.0, r.height + 6.0);
                            canvas.fill_rounded_rect(ring, radius + 3.0, theme.content);
                        }
                        let fill = if *on {
                            style.fill.unwrap_or(theme.accent)
                        } else {
                            style.fill.unwrap_or(theme.surface)
                        };
                        canvas.fill_rounded_rect(r, radius, fill);
                        let tc = if *on {
                            theme.on_accent
                        } else {
                            style.text.unwrap_or(theme.content)
                        };
                        canvas.draw_text(label, format, r, tc);
                    }
                    NodeKind::Separator { vertical } => {
                        let r = node.rect;
                        let col = style.fill.unwrap_or(theme.track);
                        let th = 2.0;
                        let line = if *vertical {
                            Rect::new(r.x + (r.width - th) / 2.0, r.y, th, r.height)
                        } else {
                            Rect::new(r.x, r.y + (r.height - th) / 2.0, r.width, th)
                        };
                        canvas.fill_rounded_rect(line, th / 2.0, col);
                    }
                    NodeKind::Meter { value, segments } => {
                        let r = node.rect;
                        let n = (*segments).max(1);
                        let bh = r.height.min(24.0);
                        let by = r.y + (r.height - bh) / 2.0;
                        let gap = 3.0;
                        let sw = ((r.width - gap * (n - 1) as f32) / n as f32).max(1.0);
                        let filled = (value.clamp(0.0, 1.0) * n as f32).round() as usize;
                        let col = style.fill.unwrap_or(theme.accent);
                        for i in 0..n {
                            let x = r.x + i as f32 * (sw + gap);
                            let c = if i < filled { col } else { theme.track };
                            canvas.fill_rounded_rect(Rect::new(x, by, sw, bh), 3.0, c);
                        }
                    }
                    NodeKind::Chart { values } => {
                        let r = node.rect;
                        let n = values.len();
                        if n == 0 {
                            if outer.is_some() {
                                canvas.pop_clip();
                            }
                            return;
                        }
                        let base = Rect::new(r.x, r.y + r.height - 2.0, r.width, 2.0);
                        canvas.fill_rounded_rect(base, 1.0, theme.track);
                        let gap = 8.0;
                        let bw = ((r.width - gap * (n - 1) as f32) / n as f32).max(1.0);
                        let max = values.iter().cloned().fold(0.0f32, f32::max).max(0.0001);
                        let area = (r.height - 6.0).max(0.0);
                        let col = style.fill.unwrap_or(theme.accent);
                        for (i, v) in values.iter().enumerate() {
                            let hh = (v.max(0.0) / max * area).max(2.0);
                            let x = r.x + i as f32 * (bw + gap);
                            let y = r.y + r.height - 2.0 - hh;
                            canvas.fill_rounded_rect(Rect::new(x, y, bw, hh), 4.0, col);
                        }
                    }
                    NodeKind::Range { lo, hi } => {
                        let r = node.rect;
                        let a = lo.clamp(0.0, 1.0);
                        let b = hi.clamp(a, 1.0);
                        let track_h = 6.0;
                        let cy = r.y + r.height / 2.0;
                        let track = Rect::new(r.x, cy - track_h / 2.0, r.width, track_h);
                        canvas.fill_rounded_rect(track, track_h / 2.0, theme.track);
                        let fill = style.fill.unwrap_or(theme.accent);
                        let sel = Rect::new(
                            r.x + r.width * a,
                            cy - track_h / 2.0,
                            (r.width * (b - a)).max(0.0),
                            track_h,
                        );
                        canvas.fill_rounded_rect(sel, track_h / 2.0, fill);
                        let kd = 16.0;
                        let cap = (r.x + r.width - kd).max(r.x);
                        for t in [a, b] {
                            let kx = (r.x + r.width * t - kd / 2.0).clamp(r.x, cap);
                            let knob = Rect::new(kx, cy - kd / 2.0, kd, kd);
                            canvas.fill_rounded_rect(knob, kd / 2.0, theme.content);
                        }
                    }
                    NodeKind::Status { text } => {
                        let r = node.rect;
                        let fill = style.fill.unwrap_or(theme.surface);
                        canvas.fill_rounded_rect(r, style.radius.unwrap_or(6.0), fill);
                        let tr = Rect::new(r.x + 12.0, r.y, (r.width - 24.0).max(0.0), r.height);
                        let col = style.text.unwrap_or(theme.content);
                        canvas.draw_text(text, format_left, tr, col);
                    }
                    NodeKind::Split { label, radius, .. } => {
                        let r = node.rect;
                        let rad = style.radius.unwrap_or(*radius);
                        let fill = style.fill.unwrap_or(theme.accent);
                        canvas.fill_rounded_rect(r, rad, fill);
                        let text_color = style.text.unwrap_or(theme.on_accent);
                        let main =
                            Rect::new(r.x, r.y, (r.width - SPLIT_ARROW).max(0.0), r.height);
                        canvas.draw_text(label, format, main, text_color);
                        let line = Rect::new(
                            r.x + r.width - SPLIT_ARROW,
                            r.y + 6.0,
                            1.0,
                            (r.height - 12.0).max(0.0),
                        );
                        canvas.fill_rounded_rect(line, 0.5, text_color);
                        let arrow: Vec<u16> = "\u{25BC}".encode_utf16().collect();
                        let ar =
                            Rect::new(r.x + r.width - SPLIT_ARROW, r.y, SPLIT_ARROW, r.height);
                        canvas.draw_text(&arrow, format, ar, text_color);
                    }
                    NodeKind::MenuBar { titles, .. } => {
                        let r = node.rect;
                        let fill = style.fill.unwrap_or(theme.surface);
                        canvas.fill_rounded_rect(r, style.radius.unwrap_or(6.0), fill);
                        let col = style.text.unwrap_or(theme.content);
                        for (i, t) in titles.iter().enumerate() {
                            let tx = r.x + i as f32 * BAR_ITEM;
                            canvas.draw_text(
                                t,
                                format,
                                Rect::new(tx, r.y, BAR_ITEM, r.height),
                                col,
                            );
                        }
                    }
                    NodeKind::Dial { value, label } => {
                        let r = node.rect;
                        let d = r.width.min(r.height);
                        let cx = r.x + r.width / 2.0;
                        let cy = r.y + r.height / 2.0;
                        let radius = d / 2.0 - 8.0;
                        let dot = (d / 12.0).max(3.0);
                        let steps = 40;
                        let start = 0.75 * std::f32::consts::TAU;
                        let sweep = 0.75 * std::f32::consts::TAU;
                        let v = value.clamp(0.0, 1.0);
                        let filled = (steps as f32 * v) as usize;
                        let fill_col = style.fill.unwrap_or(theme.accent);
                        for i in 0..steps {
                            let t = i as f32 / (steps - 1) as f32;
                            let a = start + sweep * t;
                            let px = cx + a.cos() * radius - dot / 2.0;
                            let py = cy + a.sin() * radius - dot / 2.0;
                            let col = if i < filled { fill_col } else { theme.track };
                            canvas.fill_rounded_rect(
                                Rect::new(px, py, dot, dot),
                                dot / 2.0,
                                col,
                            );
                        }
                        let body = (radius - dot * 1.6).max(6.0);
                        canvas.fill_rounded_rect(
                            Rect::new(cx - body, cy - body, body * 2.0, body * 2.0),
                            body,
                            theme.surface,
                        );
                        let a = start + sweep * v;
                        let hx = cx + a.cos() * (body - 8.0) - 4.0;
                        let hy = cy + a.sin() * (body - 8.0) - 4.0;
                        canvas.fill_rounded_rect(Rect::new(hx, hy, 8.0, 8.0), 4.0, fill_col);
                        let tc = style.text.unwrap_or(theme.content);
                        canvas.draw_text(label, format, r, tc);
                    }
                    NodeKind::TreeView {
                        items,
                        selected,
                        scroll,
                        cols,
                        widths,
                        multi,
                        ..
                    } => {
                        let r = node.rect;
                        let rad = style.radius.unwrap_or(8.0);
                        canvas.fill_rounded_rect(r, rad, style.fill.unwrap_or(theme.surface));
                        canvas.push_clip(r);
                        let mut vis: Vec<usize> = Vec::new();
                        let mut skip: Option<usize> = None;
                        for (i, it) in items.iter().enumerate() {
                            if let Some(d) = skip {
                                if it.depth > d {
                                    continue;
                                }
                                skip = None;
                            }
                            vis.push(i);
                            if !it.leaf && !it.open {
                                skip = Some(it.depth);
                            }
                        }
                        let color = style.text.unwrap_or(theme.content);
                        let bounds =
                            crate::tree::column_bounds(r, cols.len().max(1), widths);
                        let head_h = if cols.is_empty() {
                            0.0
                        } else {
                            crate::tree::TREE_HEADER
                        };
                        if !cols.is_empty() {
                            canvas.fill_rounded_rect(
                                Rect::new(r.x, r.y, r.width, head_h),
                                0.0,
                                theme.track,
                            );
                            for (c, title) in cols.iter().enumerate() {
                                let (cx, cw) = bounds[c];
                                canvas.draw_text(
                                    title,
                                    format_left,
                                    Rect::new(cx + 8.0, r.y, (cw - 12.0).max(0.0), head_h),
                                    color,
                                );
                            }
                        }
                        let top = r.y + head_h;
                        let view = (r.height - head_h).max(0.0);
                        canvas.push_clip(Rect::new(r.x, top, r.width, view));
                        let first = (*scroll / LIST_ROW).floor().max(0.0) as usize;
                        let span = (view / LIST_ROW).ceil() as usize + 2;
                        let last = (first + span).min(vis.len());
                        for row in first..last {
                            let i = vis[row];
                            let it = &items[i];
                            let ry = top + row as f32 * LIST_ROW - *scroll;
                            let hl = Rect::new(
                                r.x + 3.0,
                                ry + 2.0,
                                (r.width - 6.0).max(0.0),
                                LIST_ROW - 4.0,
                            );
                            if let Some(bg) = it.bg {
                                canvas.fill_rounded_rect(hl, 5.0, Color::hexa(bg));
                            }
                            if *selected == Some(i) || multi.contains(&i) {
                                canvas.fill_rounded_rect(hl, 5.0, theme.selection);
                            }
                            let fg = match it.fg {
                                Some(c) => Color::hexa(c),
                                None => color,
                            };
                            for (c, cell) in it.cbg.iter().enumerate() {
                                let Some(col) = cell else { continue };
                                let Some((cx, cw)) = bounds.get(c) else {
                                    break;
                                };
                                canvas.fill_rounded_rect(
                                    Rect::new(*cx + 1.0, ry + 2.0, (*cw - 2.0).max(0.0),
                                              LIST_ROW - 4.0),
                                    4.0,
                                    Color::hexa(*col),
                                );
                            }
                            let cell_fg = |c: usize| match it.cfg.get(c) {
                                Some(Some(v)) => Color::hexa(*v),
                                _ => fg,
                            };
                            let ax = bounds[0].0 + 8.0 + it.depth as f32 * 18.0;
                            if !it.leaf {
                                let arrow: Vec<u16> = if it.open {
                                    "\u{25BC}".encode_utf16().collect()
                                } else {
                                    "\u{25B6}".encode_utf16().collect()
                                };
                                canvas.draw_text(
                                    &arrow,
                                    format_left,
                                    Rect::new(ax, ry, 20.0, LIST_ROW),
                                    fg,
                                );
                            }
                            let mut tx = ax + 22.0;
                            if let Some(icon) = &it.icon {
                                if let Some(Some(bmp)) = img_cache.get(icon) {
                                    let sz = (LIST_ROW - 8.0).clamp(12.0, 24.0);
                                    canvas.draw_bitmap(
                                        bmp,
                                        Rect::new(tx, ry + (LIST_ROW - sz) / 2.0, sz, sz),
                                        0,
                                    );
                                    tx += sz + 6.0;
                                }
                            }
                            let name_end = if cols.is_empty() {
                                r.x + r.width - 8.0
                            } else {
                                bounds[0].0 + bounds[0].1
                            };
                            canvas.draw_text(
                                &it.label,
                                format_left,
                                Rect::new(tx, ry, (name_end - tx - 6.0).max(0.0), LIST_ROW),
                                cell_fg(0),
                            );
                            for (c, val) in it.values.iter().enumerate() {
                                if c + 1 >= bounds.len() {
                                    break;
                                }
                                let (cx, cw) = bounds[c + 1];
                                canvas.draw_text(
                                    val,
                                    format_left,
                                    Rect::new(cx + 8.0, ry, (cw - 12.0).max(0.0), LIST_ROW),
                                    cell_fg(c + 1),
                                );
                            }
                        }
                        canvas.pop_clip();
                        for c in 1..cols.len() {
                            canvas.fill_rounded_rect(
                                Rect::new(bounds[c].0, r.y, 1.0, r.height),
                                0.0,
                                theme.track,
                            );
                        }
                        let content = vis.len() as f32 * LIST_ROW;
                        draw_scrollbar(
                            &canvas,
                            Rect::new(r.x, top, r.width, view),
                            content,
                            view,
                            *scroll,
                            theme.track,
                            theme.content,
                        );
                        if ring == Some(id) {
                            canvas.stroke_rect(r, 2.0, theme.accent);
                        }
                        canvas.pop_clip();
                    }
                    NodeKind::Calendar { year, month, day } => {
                        let r = node.rect;
                        let rad = style.radius.unwrap_or(8.0);
                        canvas.fill_rounded_rect(r, rad, style.fill.unwrap_or(theme.surface));
                        let color = style.text.unwrap_or(theme.content);
                        let title: Vec<u16> =
                            format!("{} {}", MONTHS[(*month as usize - 1).min(11)], year)
                                .encode_utf16()
                                .collect();
                        canvas.draw_text(
                            &title,
                            format,
                            Rect::new(r.x, r.y, r.width, CAL_HEADER),
                            color,
                        );
                        let prev: Vec<u16> = "\u{25C0}".encode_utf16().collect();
                        let next: Vec<u16> = "\u{25B6}".encode_utf16().collect();
                        canvas.draw_text(
                            &prev,
                            format,
                            Rect::new(r.x, r.y, 40.0, CAL_HEADER),
                            color,
                        );
                        canvas.draw_text(
                            &next,
                            format,
                            Rect::new(r.x + r.width - 40.0, r.y, 40.0, CAL_HEADER),
                            color,
                        );
                        let cw = r.width / 7.0;
                        let rh = ((r.height - CAL_HEADER - CAL_WEEK) / 6.0).max(1.0);
                        for i in 0..7 {
                            let wd: Vec<u16> = WEEKDAYS[i].encode_utf16().collect();
                            canvas.draw_text(
                                &wd,
                                format,
                                Rect::new(
                                    r.x + i as f32 * cw,
                                    r.y + CAL_HEADER,
                                    cw,
                                    CAL_WEEK,
                                ),
                                theme.track,
                            );
                        }
                        let first = first_weekday(*year, *month);
                        let n = days_in_month(*year, *month);
                        for d in 1..=n {
                            let idx = first + d - 1;
                            let cx = r.x + (idx % 7) as f32 * cw;
                            let cy = r.y + CAL_HEADER + CAL_WEEK + (idx / 7) as f32 * rh;
                            let cell = Rect::new(cx + 2.0, cy + 2.0, cw - 4.0, rh - 4.0);
                            let sel = d == *day;
                            if sel {
                                canvas.fill_rounded_rect(
                                    cell,
                                    6.0,
                                    style.fill.unwrap_or(theme.accent),
                                );
                            }
                            let t: Vec<u16> = d.to_string().encode_utf16().collect();
                            let tc = if sel { theme.on_accent } else { color };
                            canvas.draw_text(&t, format, cell, tc);
                        }
                    }
                    NodeKind::Color { hue, sat, val } => {
                        let r = node.rect;
                        let rad = style.radius.unwrap_or(8.0);
                        canvas.fill_rounded_rect(r, rad, style.fill.unwrap_or(theme.surface));
                        let pad = 8.0;
                        let strip_h = 18.0;
                        let sw = 46.0;
                        let area = Rect::new(
                            r.x + pad,
                            r.y + pad,
                            (r.width - 3.0 * pad - sw).max(1.0),
                            (r.height - 3.0 * pad - strip_h).max(1.0),
                        );
                        let cells = 16;
                        let cw = area.width / cells as f32;
                        let ch = area.height / cells as f32;
                        for i in 0..cells {
                            for j in 0..cells {
                                let s = i as f32 / (cells - 1) as f32;
                                let v = 1.0 - j as f32 / (cells - 1) as f32;
                                let (cr, cg, cb) = hsv_rgb(*hue, s, v);
                                canvas.fill_rounded_rect(
                                    Rect::new(
                                        area.x + i as f32 * cw,
                                        area.y + j as f32 * ch,
                                        cw + 1.0,
                                        ch + 1.0,
                                    ),
                                    0.0,
                                    Color::rgb(cr, cg, cb),
                                );
                            }
                        }
                        let px = area.x + area.width * sat.clamp(0.0, 1.0);
                        let py = area.y + area.height * (1.0 - val.clamp(0.0, 1.0));
                        canvas.stroke_rect(
                            Rect::new(px - 5.0, py - 5.0, 10.0, 10.0),
                            2.0,
                            theme.on_accent,
                        );
                        let strip =
                            Rect::new(area.x, area.y + area.height + pad, area.width, strip_h);
                        let steps = 48;
                        let ws = strip.width / steps as f32;
                        for i in 0..steps {
                            let h = i as f32 / (steps - 1) as f32;
                            let (cr, cg, cb) = hsv_rgb(h, 1.0, 1.0);
                            canvas.fill_rounded_rect(
                                Rect::new(
                                    strip.x + i as f32 * ws,
                                    strip.y,
                                    ws + 1.0,
                                    strip.height,
                                ),
                                0.0,
                                Color::rgb(cr, cg, cb),
                            );
                        }
                        let hx = strip.x + strip.width * hue.clamp(0.0, 1.0);
                        canvas.fill_rounded_rect(
                            Rect::new(hx - 2.0, strip.y - 2.0, 4.0, strip.height + 4.0),
                            2.0,
                            theme.content,
                        );
                        let (cr, cg, cb) = hsv_rgb(*hue, *sat, *val);
                        let swatch = Rect::new(
                            r.x + r.width - pad - sw,
                            r.y + pad,
                            sw,
                            (r.height - 2.0 * pad).max(0.0),
                        );
                        canvas.fill_rounded_rect(swatch, 6.0, Color::rgb(cr, cg, cb));
                    }
                    NodeKind::Time { hour, minute } => {
                        let r = node.rect;
                        let rad = style.radius.unwrap_or(8.0);
                        canvas.fill_rounded_rect(r, rad, style.fill.unwrap_or(theme.surface));
                        let color = style.text.unwrap_or(theme.content);
                        let half = r.width / 2.0;
                        let arrow_h = (r.height * 0.28).max(16.0);
                        let up: Vec<u16> = "\u{25B2}".encode_utf16().collect();
                        let dn: Vec<u16> = "\u{25BC}".encode_utf16().collect();
                        for (i, part) in [*hour, *minute].iter().enumerate() {
                            let px = r.x + i as f32 * half;
                            canvas.draw_text(
                                &up,
                                format,
                                Rect::new(px, r.y, half, arrow_h),
                                theme.track,
                            );
                            canvas.draw_text(
                                &dn,
                                format,
                                Rect::new(px, r.y + r.height - arrow_h, half, arrow_h),
                                theme.track,
                            );
                            let t: Vec<u16> =
                                format!("{:02}", part).encode_utf16().collect();
                            canvas.draw_text(
                                &t,
                                format,
                                Rect::new(px, r.y + arrow_h, half, (r.height - 2.0 * arrow_h).max(0.0)),
                                color,
                            );
                        }
                        let colon: Vec<u16> = ":".encode_utf16().collect();
                        canvas.draw_text(&colon, format, r, color);
                    }
                    NodeKind::PropGrid {
                        rows,
                        selected,
                        scroll,
                    } => {
                        let r = node.rect;
                        let rad = style.radius.unwrap_or(8.0);
                        canvas.fill_rounded_rect(r, rad, style.fill.unwrap_or(theme.surface));
                        canvas.push_clip(r);
                        let color = style.text.unwrap_or(theme.content);
                        let split = r.x + r.width * 0.45;
                        let first = (scroll / LIST_ROW).floor().max(0.0) as usize;
                        let span = (r.height / LIST_ROW).ceil() as usize + 2;
                        let last = (first + span).min(rows.len());
                        for i in first..last {
                            let (k, v) = &rows[i];
                            let ry = r.y + i as f32 * LIST_ROW - scroll;
                            if *selected == Some(i) {
                                let hl = Rect::new(
                                    r.x + 3.0,
                                    ry + 2.0,
                                    (r.width - 6.0).max(0.0),
                                    LIST_ROW - 4.0,
                                );
                                canvas.fill_rounded_rect(hl, 5.0, theme.selection);
                            }
                            let kr = Rect::new(
                                r.x + 12.0,
                                ry,
                                (split - r.x - 20.0).max(0.0),
                                LIST_ROW,
                            );
                            canvas.draw_text(k, format_left, kr, theme.track);
                            let vr = Rect::new(
                                split + 10.0,
                                ry,
                                (r.x + r.width - split - 22.0).max(0.0),
                                LIST_ROW,
                            );
                            canvas.draw_text(v, format_left, vr, color);
                            let line =
                                Rect::new(r.x + 6.0, ry + LIST_ROW - 1.0, (r.width - 12.0).max(0.0), 1.0);
                            canvas.fill_rounded_rect(line, 0.5, theme.track);
                        }
                        let content = rows.len() as f32 * LIST_ROW;
                        draw_scrollbar(
                            &canvas,
                            r,
                            content,
                            r.height,
                            *scroll,
                            theme.track,
                            theme.content,
                        );
                        canvas.pop_clip();
                    }
                    NodeKind::Badge { text, dot } => {
                        let r = node.rect;
                        let fill = style.fill.unwrap_or(theme.accent);
                        if *dot {
                            let d = r.height.min(r.width).min(12.0);
                            let bx = r.x + r.width - d;
                            let by = r.y + (r.height - d) / 2.0;
                            canvas.fill_rounded_rect(
                                Rect::new(bx, by, d, d),
                                d / 2.0,
                                fill,
                            );
                        } else {
                            canvas.fill_rounded_rect(r, r.height / 2.0, fill);
                            let tc = style.text.unwrap_or(theme.on_accent);
                            canvas.draw_text(text, format, r, tc);
                        }
                    }
                    NodeKind::Crumbs { items } => {
                        let r = node.rect;
                        let color = style.text.unwrap_or(theme.content);
                        let sep: Vec<u16> = "\u{203A}".encode_utf16().collect();
                        let mut cx = r.x + 10.0;
                        let last = items.len().saturating_sub(1);
                        for (i, it) in items.iter().enumerate() {
                            let w = crumb_width(it.len());
                            let tc = if i == last {
                                color
                            } else {
                                style.fill.unwrap_or(theme.accent)
                            };
                            canvas.draw_text(
                                it,
                                format_left,
                                Rect::new(cx, r.y, w, r.height),
                                tc,
                            );
                            cx += w;
                            if i != last {
                                canvas.draw_text(
                                    &sep,
                                    format,
                                    Rect::new(cx, r.y, CRUMB_SEP, r.height),
                                    theme.track,
                                );
                                cx += CRUMB_SEP;
                            }
                        }
                    }
                    NodeKind::Pager { page, total } => {
                        let r = node.rect;
                        let color = style.text.unwrap_or(theme.content);
                        let fill = style.fill.unwrap_or(theme.accent);
                        let cells = *total + 2;
                        for i in 0..cells {
                            let cx = r.x + i as f32 * PAGER_CELL;
                            let cell = Rect::new(cx + 2.0, r.y + 2.0, PAGER_CELL - 4.0, (r.height - 4.0).max(0.0));
                            let label: Vec<u16> = if i == 0 {
                                "\u{25C0}".encode_utf16().collect()
                            } else if i == cells - 1 {
                                "\u{25B6}".encode_utf16().collect()
                            } else {
                                i.to_string().encode_utf16().collect()
                            };
                            let active = i > 0 && i < cells - 1 && i - 1 == *page;
                            if active {
                                canvas.fill_rounded_rect(cell, 6.0, fill);
                            } else {
                                canvas.stroke_rect(cell, 1.0, theme.track);
                            }
                            let tc = if active { theme.on_accent } else { color };
                            canvas.draw_text(&label, format, cell, tc);
                        }
                    }
                    NodeKind::Rating { value, max } => {
                        let r = node.rect;
                        let n = (*max).max(1);
                        let cell = r.width / n as f32;
                        let fill = style.fill.unwrap_or(theme.accent);
                        let star: Vec<u16> = "\u{2605}".encode_utf16().collect();
                        for i in 0..n {
                            let cx = r.x + i as f32 * cell;
                            let col = if i < *value { fill } else { theme.track };
                            canvas.draw_text(
                                &star,
                                format,
                                Rect::new(cx, r.y, cell, r.height),
                                col,
                            );
                        }
                    }
                    NodeKind::Canvas {
                        shapes,
                        ox,
                        oy,
                        rw,
                        rh,
                        scroll,
                        ..
                    } => {
                        let view = node.rect;
                        let rad = style.radius.unwrap_or(8.0);
                        canvas.fill_rounded_rect(view, rad, style.fill.unwrap_or(theme.surface));
                        canvas.push_clip(view);
                        let r = Rect::new(view.x - *ox, view.y - *oy, view.width, view.height);
                        for s in shapes.iter() {
                            let c = Color::hexa(s.color);
                            let a = s.args;
                            match s.kind {
                                0 => {
                                    let sr =
                                        Rect::new(r.x + a[0], r.y + a[1], a[2], a[3]);
                                    if a[5] > 0.0 {
                                        canvas.stroke_rect(sr, a[5], c);
                                    } else {
                                        canvas.fill_rounded_rect(sr, a[4], c);
                                    }
                                }
                                1 => {
                                    let d = a[2] * 2.0;
                                    let sr = Rect::new(
                                        r.x + a[0] - a[2],
                                        r.y + a[1] - a[2],
                                        d,
                                        d,
                                    );
                                    if a[3] > 0.0 {
                                        canvas.stroke_ellipse(sr, a[3], c);
                                    } else {
                                        canvas.fill_ellipse(sr, c);
                                    }
                                }
                                2 => {
                                    let w = a[4].max(1.0);
                                    let x1 = r.x + a[0];
                                    let y1 = r.y + a[1];
                                    let x2 = r.x + a[2];
                                    let y2 = r.y + a[3];
                                    canvas.draw_line(x1, y1, x2, y2, w, c);
                                }
                                3 => {
                                    let tr = Rect::new(
                                        r.x + a[0],
                                        r.y + a[1],
                                        a[2].max(1.0),
                                        a[3].max(1.0),
                                    );
                                    canvas.draw_text(&s.text, format_left, tr, c);
                                }
                                4 => {
                                    canvas.draw_arrow(
                                        r.x + a[0],
                                        r.y + a[1],
                                        r.x + a[2],
                                        r.y + a[3],
                                        a[4],
                                        a[5],
                                        c,
                                    );
                                }
                                5 => {
                                    canvas.stroke_arc(
                                        r.x + a[0],
                                        r.y + a[1],
                                        a[2],
                                        a[3],
                                        a[4],
                                        a[5].max(1.0),
                                        c,
                                    );
                                }
                                6 => {
                                    canvas.fill_sector(
                                        r.x + a[0],
                                        r.y + a[1],
                                        a[2],
                                        a[3],
                                        a[4],
                                        c,
                                    );
                                }
                                _ => {
                                    let pts: Vec<(f32, f32)> = s
                                        .pts
                                        .chunks_exact(2)
                                        .map(|p| (r.x + p[0], r.y + p[1]))
                                        .collect();
                                    if a[0] > 0.0 {
                                        canvas.stroke_polygon(&pts, a[0], c);
                                    } else {
                                        canvas.fill_polygon(&pts, c);
                                    }
                                }
                            }
                        }
                        if *scroll {
                            draw_canvas_bars(
                                &canvas, view, *rw, *rh, *ox, *oy, theme.track, theme.content,
                            );
                        }
                        canvas.pop_clip();
                    }
                    NodeKind::Term {
                        lines,
                        input,
                        prompt,
                        scroll,
                    } => {
                        let r = node.rect;
                        let rad = style.radius.unwrap_or(8.0);
                        let bg = style.fill.unwrap_or(theme.surface);
                        canvas.fill_rounded_rect(r, rad, bg);
                        canvas.push_clip(r);
                        let color = style.text.unwrap_or(theme.content);
                        let view = (r.height - TERM_INPUT - 16.0).max(0.0);
                        for (i, line) in lines.iter().enumerate() {
                            let ly = r.y + 8.0 + i as f32 * TERM_ROW - scroll;
                            if ly + TERM_ROW < r.y || ly > r.y + view + 8.0 {
                                continue;
                            }
                            canvas.draw_text(
                                line,
                                format_left,
                                Rect::new(r.x + 12.0, ly, (r.width - 24.0).max(0.0), TERM_ROW),
                                color,
                            );
                        }
                        let iy = r.y + r.height - TERM_INPUT - 6.0;
                        let ir = Rect::new(r.x + 8.0, iy, (r.width - 16.0).max(0.0), TERM_INPUT);
                        canvas.fill_rounded_rect(ir, 6.0, theme.track);
                        canvas.draw_text(
                            prompt,
                            format_left,
                            Rect::new(ir.x + 8.0, ir.y, 24.0, ir.height),
                            theme.accent,
                        );
                        canvas.draw_text(
                            &input.text,
                            format_left,
                            Rect::new(ir.x + 32.0, ir.y, (ir.width - 44.0).max(0.0), ir.height),
                            color,
                        );
                        if focused == Some(id) {
                            let cx = ir.x
                                + 32.0
                                + x_at_index(
                                    &self.dwrite,
                                    format_left,
                                    &input.text,
                                    input.caret,
                                );
                            canvas.fill_rounded_rect(
                                Rect::new(cx, ir.y + 6.0, 2.0, ir.height - 12.0),
                                1.0,
                                color,
                            );
                        }
                        let content = lines.len() as f32 * TERM_ROW;
                        draw_scrollbar(
                            &canvas,
                            r,
                            content,
                            view,
                            *scroll,
                            theme.track,
                            theme.content,
                        );
                        canvas.pop_clip();
                    }
                    NodeKind::Dock { title, open, .. } => {
                        let r = node.rect;
                        let rad = style.radius.unwrap_or(10.0);
                        canvas.fill_rounded_rect(r, rad, style.fill.unwrap_or(theme.surface));
                        let head = Rect::new(r.x, r.y, r.width, DOCK_HEADER);
                        canvas.fill_rounded_rect(head, rad, theme.track);
                        let color = style.text.unwrap_or(theme.content);
                        let arrow: Vec<u16> = if *open {
                            "\u{25BE}".encode_utf16().collect()
                        } else {
                            "\u{25B8}".encode_utf16().collect()
                        };
                        canvas.draw_text(
                            &arrow,
                            format,
                            Rect::new(r.x + 4.0, r.y, 24.0, DOCK_HEADER),
                            color,
                        );
                        if *open {
                            canvas.draw_text(
                                title,
                                format_left,
                                Rect::new(r.x + 30.0, r.y, (r.width - 40.0).max(0.0), DOCK_HEADER),
                                color,
                            );
                        }
                    }
                    NodeKind::Drop { label } => {
                        let r = node.rect;
                        canvas.fill_rounded_rect(
                            r,
                            style.radius.unwrap_or(10.0),
                            style.fill.unwrap_or(theme.surface),
                        );
                        let col = theme.accent;
                        let dash: f32 = 10.0;
                        let step = dash * 2.0;
                        let mut x0 = r.x + 6.0;
                        while x0 < r.x + r.width - 8.0 {
                            let w = dash.min(r.x + r.width - 8.0 - x0);
                            canvas.fill_rounded_rect(Rect::new(x0, r.y + 6.0, w, 2.0), 1.0, col);
                            canvas.fill_rounded_rect(
                                Rect::new(x0, r.y + r.height - 8.0, w, 2.0),
                                1.0,
                                col,
                            );
                            x0 += step;
                        }
                        let mut y0 = r.y + 6.0;
                        while y0 < r.y + r.height - 8.0 {
                            let hh = dash.min(r.y + r.height - 8.0 - y0);
                            canvas.fill_rounded_rect(Rect::new(r.x + 6.0, y0, 2.0, hh), 1.0, col);
                            canvas.fill_rounded_rect(
                                Rect::new(r.x + r.width - 8.0, y0, 2.0, hh),
                                1.0,
                                col,
                            );
                            y0 += step;
                        }
                        canvas.draw_text(
                            label,
                            format,
                            r,
                            style.text.unwrap_or(theme.content),
                        );
                    }
                    NodeKind::Spinner { phase } => {
                        let r = node.rect;
                        let d = r.width.min(r.height).min(48.0);
                        let cx = r.x + r.width / 2.0;
                        let cy = r.y + r.height / 2.0;
                        let radius = d / 2.0 - 4.0;
                        let dot = (d / 10.0).max(2.5);
                        let col = style.fill.unwrap_or(theme.accent);
                        for i in 0..8 {
                            let t = i as f32 / 8.0;
                            let a = (t + *phase) * std::f32::consts::TAU;
                            let alpha = 0.15 + 0.85 * t;
                            let px = cx + a.cos() * radius - dot / 2.0;
                            let py = cy + a.sin() * radius - dot / 2.0;
                            canvas.fill_rounded_rect(
                                Rect::new(px, py, dot, dot),
                                dot / 2.0,
                                Color::rgba(col.r, col.g, col.b, alpha),
                            );
                        }
                    }
                    NodeKind::Gauge { value, label } => {
                        let r = node.rect;
                        let d = r.width.min(r.height);
                        let cx = r.x + r.width / 2.0;
                        let cy = r.y + r.height / 2.0;
                        let radius = d / 2.0 - 8.0;
                        let th = (d / 10.0).max(4.0);
                        let steps = 48;
                        let start = 0.75 * std::f32::consts::TAU;
                        let sweep = 0.75 * std::f32::consts::TAU;
                        let filled = (steps as f32 * value.clamp(0.0, 1.0)) as usize;
                        let track_col = theme.track;
                        let fill_col = style.fill.unwrap_or(theme.accent);
                        for i in 0..steps {
                            let t = i as f32 / (steps - 1) as f32;
                            let a = start + sweep * t;
                            let px = cx + a.cos() * radius - th / 2.0;
                            let py = cy + a.sin() * radius - th / 2.0;
                            let col = if i < filled { fill_col } else { track_col };
                            canvas.fill_rounded_rect(
                                Rect::new(px, py, th, th),
                                th / 2.0,
                                col,
                            );
                        }
                        let tc = style.text.unwrap_or(theme.content);
                        canvas.draw_text(label, format, r, tc);
                    }
                    NodeKind::Accordion {
                        title,
                        open,
                        radius,
                        ..
                    } => {
                        let r = node.rect;
                        let rad = style.radius.unwrap_or(*radius);
                        let head = Rect::new(r.x, r.y, r.width, ACC_HEADER);
                        let fill = if hovered == Some(id) {
                            style.fill.unwrap_or(theme.track)
                        } else {
                            style.fill.unwrap_or(theme.surface)
                        };
                        canvas.fill_rounded_rect(head, rad, fill);
                        let arrow: Vec<u16> = if *open {
                            "\u{25BC}".encode_utf16().collect()
                        } else {
                            "\u{25B6}".encode_utf16().collect()
                        };
                        let color = style.text.unwrap_or(theme.content);
                        canvas.draw_text(
                            &arrow,
                            format_left,
                            Rect::new(r.x + 12.0, r.y, 24.0, ACC_HEADER),
                            color,
                        );
                        canvas.draw_text(
                            title,
                            format_left,
                            Rect::new(r.x + 40.0, r.y, (r.width - 52.0).max(0.0), ACC_HEADER),
                            color,
                        );
                        if ring == Some(id) {
                            canvas.stroke_rect(head, 2.0, theme.accent);
                        }
                    }
                    NodeKind::Stack { .. } => {}
                    NodeKind::Splitter { ratio, vertical } => {
                        let r = node.rect;
                        let bar = if *vertical {
                            let w1 = (r.width - SPLIT_W) * *ratio;
                            Rect::new(r.x + w1, r.y, SPLIT_W, r.height)
                        } else {
                            let h1 = (r.height - SPLIT_W) * *ratio;
                            Rect::new(r.x, r.y + h1, r.width, SPLIT_W)
                        };
                        let col = style.fill.unwrap_or(theme.track);
                        canvas.fill_rounded_rect(bar, SPLIT_W / 2.0, col);
                        let g = 3.0;
                        let grip = if *vertical {
                            Rect::new(
                                bar.x + (SPLIT_W - g) / 2.0,
                                bar.y + bar.height / 2.0 - 16.0,
                                g,
                                32.0,
                            )
                        } else {
                            Rect::new(
                                bar.x + bar.width / 2.0 - 16.0,
                                bar.y + (SPLIT_W - g) / 2.0,
                                32.0,
                                g,
                            )
                        };
                        canvas.fill_rounded_rect(grip, g / 2.0, theme.content);
                    }
                    NodeKind::Scroll { offset, content } => {
                        let r = node.rect;
                        if let Some(f) = style.fill {
                            canvas.fill_rounded_rect(r, style.radius.unwrap_or(8.0), f);
                        }
                        draw_scrollbar(
                            &canvas,
                            r,
                            *content,
                            r.height,
                            *offset,
                            theme.track,
                            theme.content,
                        );
                    }
                    NodeKind::Group { title, radius } => {
                        let r = node.rect;
                        let rad = style.radius.unwrap_or(*radius);
                        if let Some(f) = style.fill {
                            canvas.fill_rounded_rect(r, rad, f);
                        }
                        canvas.stroke_rect(r, 1.0, theme.track);
                        let tr = Rect::new(
                            r.x + 14.0,
                            r.y + 2.0,
                            (r.width - 28.0).max(0.0),
                            GROUP_HEADER,
                        );
                        let color = style.text.unwrap_or(theme.content);
                        canvas.draw_text(title, format_left, tr, color);
                    }
                    NodeKind::Link { label } => {
                        let r = node.rect;
                        let color = style.text.unwrap_or(theme.accent);
                        canvas.draw_text(label, format_left, r, color);
                        if hovered == Some(id) || focused == Some(id) {
                            let w = text_width(dwrite, format_left, label);
                            let uy = r.y + r.height / 2.0 + 10.0;
                            canvas.fill_rounded_rect(
                                Rect::new(r.x, uy, w.min(r.width), 1.5),
                                0.75,
                                color,
                            );
                        }
                    }
                    NodeKind::List {
                        items,
                        selected,
                        scroll,
                        multi,
                        ..
                    } => {
                        let r = node.rect;
                        canvas.push_clip(r);
                        let bg = style.fill.unwrap_or(theme.surface);
                        canvas.fill_rounded_rect(r, style.radius.unwrap_or(8.0), bg);
                        let hover_row = if hot == Some(id) && mouse.1 >= r.y {
                            let ri = ((mouse.1 - r.y + *scroll) / LIST_ROW).floor();
                            if ri >= 0.0 && (ri as usize) < items.len() {
                                Some(ri as usize)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let first = (*scroll / LIST_ROW).floor().max(0.0) as usize;
                        let span = (r.height / LIST_ROW).ceil() as usize + 2;
                        let last = (first + span).min(items.len());
                        for i in first..last {
                            let item = &items[i];
                            let iy = r.y - *scroll + LIST_ROW * i as f32;
                            let row_rect = Rect::new(r.x, iy, r.width, LIST_ROW);
                            if *selected == Some(i) || multi.contains(&i) {
                                canvas.fill_rounded_rect(row_rect, 0.0, theme.selection);
                            } else if hover_row == Some(i) {
                                canvas.fill_rounded_rect(row_rect, 0.0, theme.track);
                            }
                            let tr = Rect::new(
                                r.x + 12.0,
                                iy,
                                (r.width - 24.0 - SCROLLBAR_W).max(0.0),
                                LIST_ROW,
                            );
                            let color = style.text.unwrap_or(theme.content);
                            canvas.draw_text(item, format_left, tr, color);
                        }
                        draw_scrollbar(
                            &canvas,
                            r,
                            items.len() as f32 * LIST_ROW,
                            r.height,
                            *scroll,
                            theme.track,
                            theme.content,
                        );
                        if ring == Some(id) {
                            canvas.stroke_rect(r, 2.0, theme.accent);
                        }
                        canvas.pop_clip();
                    }
                    NodeKind::TextBox { state } => {
                        let mut masked = crate::tree::TextState::new();
                        let state = if is_pwd {
                            masked.text = vec![0x2022u16; state.text.len()];
                            masked.caret = state.caret.min(masked.text.len());
                            masked.anchor = state.anchor.min(masked.text.len());
                            masked.scroll = state.scroll;
                            &masked
                        } else {
                            state
                        };
                        if node.multiline {
                            let r = node.rect;
                            let is_focused = focused == Some(id);
                            let border = if is_focused { theme.accent } else { theme.track };
                            let rad = style.radius.unwrap_or(8.0);
                            canvas.fill_rounded_rect(r, rad, border);
                            let inner =
                                Rect::new(r.x + 2.0, r.y + 2.0, r.width - 4.0, r.height - 4.0);
                            let inner_fill = style.fill.unwrap_or(theme.surface);
                            canvas.fill_rounded_rect(inner, (rad - 2.0).max(0.0), inner_fill);
                            let pad = 10.0;
                            let text_rect = Rect::new(
                                r.x + pad,
                                r.y + pad,
                                (r.width - 2.0 * pad).max(0.0),
                                (r.height - 2.0 * pad).max(0.0),
                            );
                            canvas.push_clip(text_rect);
                            let tcolor = style.text.unwrap_or(theme.content);
                            let (sa, sb) = state.sel_range();
                            if sa != sb {
                                for hr in wrapped_ranges(
                                    dwrite,
                                    format_wrap,
                                    &state.text,
                                    text_rect.width,
                                    sa,
                                    sb,
                                ) {
                                    canvas.fill_rounded_rect(
                                        Rect::new(
                                            text_rect.x + hr.x,
                                            text_rect.y + hr.y,
                                            hr.width,
                                            hr.height,
                                        ),
                                        2.0,
                                        theme.selection,
                                    );
                                }
                            }
                            if !state.text.is_empty() {
                                canvas.draw_text(&state.text, format_wrap, text_rect, tcolor);
                            }
                            if is_focused {
                                let width = text_rect.width.max(1.0);
                                let (cx, cy, lh) = wrapped_caret(
                                    dwrite,
                                    format_wrap,
                                    &state.text,
                                    width,
                                    state.caret,
                                );
                                let caret = Rect::new(
                                    text_rect.x + cx,
                                    text_rect.y + cy,
                                    2.0,
                                    lh.max(16.0),
                                );
                                canvas.fill_rounded_rect(caret, 1.0, theme.accent);
                            }
                            canvas.pop_clip();
                            if outer.is_some() {
                                canvas.pop_clip();
                            }
                            return;
                        }
                        let r = node.rect;
                        let is_focused = focused == Some(id);
                        let border = if is_focused { theme.accent } else { theme.track };
                        let rad = style.radius.unwrap_or(8.0);
                        canvas.fill_rounded_rect(r, rad, border);
                        let inner =
                            Rect::new(r.x + 2.0, r.y + 2.0, r.width - 4.0, r.height - 4.0);
                        let inner_fill = style.fill.unwrap_or(theme.surface);
                        canvas.fill_rounded_rect(inner, (rad - 2.0).max(0.0), inner_fill);

                        let pad = 12.0;
                        let base_x = r.x + pad - state.scroll;
                        let clip = Rect::new(r.x + 4.0, r.y, (r.width - 8.0).max(0.0), r.height);
                        canvas.push_clip(clip);

                        let (sa, sb) = state.sel_range();
                        if sa != sb {
                            let xa = x_at_index(dwrite, format_left, &state.text, sa);
                            let xb = x_at_index(dwrite, format_left, &state.text, sb);
                            let sel = Rect::new(
                                base_x + xa,
                                r.y + 8.0,
                                (xb - xa).max(0.0),
                                (r.height - 16.0).max(0.0),
                            );
                            canvas.fill_rounded_rect(sel, 3.0, theme.selection);
                        }

                        let text_rect = Rect::new(base_x, r.y, 100000.0, r.height);
                        canvas.draw_text(&state.text, format_left, text_rect, theme.content);

                        if is_focused {
                            let cx = base_x
                                + x_at_index(dwrite, format_left, &state.text, state.caret);
                            let caret =
                                Rect::new(cx, r.y + 10.0, 2.0, (r.height - 20.0).max(2.0));
                            canvas.fill_rounded_rect(caret, 1.0, theme.accent);
                        }
                        canvas.pop_clip();
                    }
                    NodeKind::Dropdown {
                        options,
                        selected,
                        ..
                    } => {
                        let r = node.rect;
                        let border = if ring == Some(id) { theme.accent } else { theme.track };
                        let fill = style.fill.unwrap_or(theme.surface);
                        canvas.fill_rounded_rect(r, 8.0, fill);
                        canvas.stroke_rect(r, if ring == Some(id) { 2.0 } else { 1.0 }, border);
                        let label = options.get(*selected).cloned().unwrap_or_default();
                        let color = style.text.unwrap_or(theme.content);
                        let tr = Rect::new(r.x + 12.0, r.y, (r.width - 40.0).max(0.0), r.height);
                        canvas.draw_text(&label, format_left, tr, color);
                        let chev: Vec<u16> = "\u{25BE}".encode_utf16().collect();
                        let cr = Rect::new(r.x + r.width - 28.0, r.y, 20.0, r.height);
                        canvas.draw_text(&chev, format, cr, color);
                    }
                    NodeKind::Tabs { labels, selected } => {
                        let r = node.rect;
                        let header = Rect::new(r.x, r.y, r.width, TAB_HEADER);
                        let header_fill = style.fill.unwrap_or(theme.surface);
                        canvas.fill_rounded_rect(header, 8.0, header_fill);
                        let n = labels.len().max(1);
                        let tab_w = r.width / n as f32;
                        for (i, lab) in labels.iter().enumerate() {
                            let tx = r.x + tab_w * i as f32;
                            let tab_rect = Rect::new(tx, r.y, tab_w, TAB_HEADER);
                            if i == *selected {
                                let hl =
                                    Rect::new(tx + 4.0, r.y + 4.0, tab_w - 8.0, TAB_HEADER - 8.0);
                                canvas.fill_rounded_rect(hl, 6.0, theme.accent);
                            }
                            let color = if i == *selected {
                                theme.on_accent
                            } else {
                                theme.content
                            };
                            canvas.draw_text(lab, format, tab_rect, color);
                        }
                        if ring == Some(id) {
                            canvas.stroke_rect(header, 2.0, theme.accent);
                        }
                    }
                    NodeKind::Table {
                        columns,
                        rows,
                        selected,
                        scroll,
                        hline,
                        vline,
                        cbg,
                        widths,
                        mins,
                    } => {
                        let r = node.rect;
                        canvas.push_clip(r);
                        let bg = style.fill.unwrap_or(theme.surface);
                        canvas.fill_rounded_rect(r, 8.0, bg);
                        let ncol = columns.len().max(1);
                        let bnd = crate::tree::column_bounds_min(r, ncol, widths, mins);
                        let top = r.y + TABLE_HEADER;
                        let hover_row = if hot == Some(id) && mouse.1 >= top {
                            let ri = ((mouse.1 - top + *scroll) / TABLE_ROW).floor();
                            if ri >= 0.0 {
                                Some(ri as usize)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let vis_h = (r.y + r.height - top).max(0.0);
                        let first = (*scroll / TABLE_ROW).floor().max(0.0) as usize;
                        let span = (vis_h / TABLE_ROW).ceil() as usize + 2;
                        let last = (first + span).min(rows.len());
                        for ri in first..last {
                            let row = &rows[ri];
                            let ry = top - *scroll + TABLE_ROW * ri as f32;
                            let row_rect = Rect::new(r.x, ry, r.width, TABLE_ROW);
                            if *selected == Some(ri) {
                                canvas.fill_rounded_rect(row_rect, 0.0, theme.selection);
                            } else if hover_row == Some(ri) {
                                canvas.fill_rounded_rect(row_rect, 0.0, theme.track);
                            }
                            for (c, cell) in row.iter().enumerate() {
                                let (cx, col_w) = match bnd.get(c) {
                                    Some(v) => *v,
                                    None => continue,
                                };
                                if let Some((_, col)) =
                                    cbg.iter().find(|((rr, cc), _)| *rr == ri && *cc == c)
                                {
                                    canvas.fill_rounded_rect(
                                        Rect::new(cx + 1.0, ry + 1.0, (col_w - 2.0).max(0.0),
                                                  TABLE_ROW - 2.0),
                                        0.0,
                                        Color::hexa(*col),
                                    );
                                }
                                let cr = Rect::new(
                                    cx + 10.0,
                                    ry,
                                    (col_w - 20.0).max(0.0),
                                    TABLE_ROW,
                                );
                                canvas.draw_text(cell, format_left, cr, theme.content);
                            }
                            if *hline > 0.0 {
                                canvas.fill_rounded_rect(
                                    Rect::new(r.x, ry + TABLE_ROW - *hline, r.width, *hline),
                                    0.0,
                                    theme.track,
                                );
                            }
                        }
                        if *vline > 0.0 {
                            for c in 1..ncol {
                                let cx = match bnd.get(c) {
                                    Some((x, _)) => *x,
                                    None => continue,
                                };
                                canvas.fill_rounded_rect(
                                    Rect::new(cx, r.y, *vline, r.height),
                                    0.0,
                                    theme.track,
                                );
                            }
                        }
                        let header = Rect::new(r.x, r.y, r.width, TABLE_HEADER);
                        canvas.fill_rounded_rect(header, 8.0, theme.track);
                        for (c, col) in columns.iter().enumerate() {
                            let (cx, col_w) = match bnd.get(c) {
                                Some(v) => *v,
                                None => continue,
                            };
                            let hr =
                                Rect::new(cx + 10.0, r.y, (col_w - 20.0).max(0.0), TABLE_HEADER);
                            canvas.draw_text(col, format_left, hr, theme.content);
                        }
                        let body = Rect::new(
                            r.x,
                            r.y + TABLE_HEADER,
                            r.width,
                            (r.height - TABLE_HEADER).max(0.0),
                        );
                        draw_scrollbar(
                            &canvas,
                            body,
                            rows.len() as f32 * TABLE_ROW,
                            body.height,
                            *scroll,
                            theme.track,
                            theme.content,
                        );
                        if ring == Some(id) {
                            canvas.stroke_rect(r, 2.0, theme.accent);
                        }
                        canvas.pop_clip();
                    }
                }
                if outer.is_some() {
                    canvas.pop_clip();
                }
            });

            if let Some(dd) = self.open_dropdown {
                let header = self.tree.get(dd).rect;
                let n = self.tree.dropdown_len(dd);
                let row_h = header.height;
                let popup = Rect::new(
                    header.x,
                    header.y + header.height,
                    header.width,
                    row_h * n as f32,
                );
                canvas.fill_rounded_rect(popup, 8.0, theme.surface);
                canvas.stroke_rect(popup, 1.0, theme.track);
                if let NodeKind::Dropdown { options, .. } = &self.tree.get(dd).kind {
                    for (i, opt) in options.iter().enumerate() {
                        let ry = popup.y + row_h * i as f32;
                        if self.dropdown_hover == Some(i) {
                            let hl = Rect::new(
                                popup.x + 3.0,
                                ry + 2.0,
                                popup.width - 6.0,
                                row_h - 4.0,
                            );
                            canvas.fill_rounded_rect(hl, 5.0, theme.selection);
                        }
                        let tr =
                            Rect::new(popup.x + 12.0, ry, (popup.width - 24.0).max(0.0), row_h);
                        canvas.draw_text(opt, format_left, tr, theme.content);
                    }
                }
            }

            if let Some(p) = self.popup.as_ref() {
                canvas.fill_rounded_rect(p.rect, 8.0, theme.surface);
                canvas.stroke_rect(p.rect, 1.0, theme.track);
                for (i, item) in p.items.iter().enumerate() {
                    let ry = p.rect.y + POPUP_ROW * i as f32;
                    if p.hover == Some(i) {
                        let hl = Rect::new(
                            p.rect.x + 3.0,
                            ry + 2.0,
                            p.rect.width - 6.0,
                            POPUP_ROW - 4.0,
                        );
                        canvas.fill_rounded_rect(hl, 5.0, theme.selection);
                    }
                    let tr = Rect::new(
                        p.rect.x + 12.0,
                        ry,
                        (p.rect.width - 24.0).max(0.0),
                        POPUP_ROW,
                    );
                    canvas.draw_text(item, format_left, tr, theme.content);
                }
            }

            if let Some(rect) = self.menu_rect() {
                canvas.fill_rounded_rect(rect, 8.0, theme.surface);
                canvas.stroke_rect(rect, 1.0, theme.track);
                let n = self.tree.menu_len();
                for i in 0..n {
                    let ry = rect.y + MENU_ROW * i as f32;
                    if self.menu_hover == Some(i) {
                        let hl = Rect::new(
                            rect.x + 3.0,
                            ry + 2.0,
                            rect.width - 6.0,
                            MENU_ROW - 4.0,
                        );
                        canvas.fill_rounded_rect(hl, 5.0, theme.selection);
                    }
                    if let Some(item) = self.tree.menu_item(i) {
                        let tr =
                            Rect::new(rect.x + 12.0, ry, (rect.width - 24.0).max(0.0), MENU_ROW);
                        canvas.draw_text(item, format_left, tr, theme.content);
                    }
                }
            }

            if self.inspector {
                let hot = self.hot;
                self.tree.for_each(|id, node| {
                    canvas.stroke_rect(node.rect, 1.0, INSPECT_LINE);
                    if hot == Some(id) {
                        canvas.fill_rounded_rect(node.rect, 0.0, INSPECT_FILL);
                    }
                });
                if let Some(id) = hot {
                    let node = self.tree.get(id);
                    let r = node.rect;
                    let info = format!(
                        "{} {}x{}",
                        kind_name(&node.kind),
                        r.width as i32,
                        r.height as i32
                    );
                    let text: Vec<u16> = info.encode_utf16().collect();
                    let bh = 22.0;
                    let label = Rect::new(r.x, (r.y - bh).max(0.0), 240.0, bh);
                    canvas.fill_rounded_rect(label, 4.0, INSPECT_LABEL_BG);
                    let tr =
                        Rect::new(label.x + 6.0, label.y, label.width - 12.0, label.height);
                    canvas.draw_text(&text, format_left, tr, INSPECT_LABEL_FG);
                }
            }
        for (pr, ph, multi) in self.tree.empty_placeholders() {
                let tr = if multi {
                    Rect::new(pr.x + 10.0, pr.y + 6.0, (pr.width - 20.0).max(0.0), 28.0)
                } else {
                    Rect::new(pr.x + 12.0, pr.y, (pr.width - 24.0).max(0.0), pr.height)
                };
                canvas.draw_text(ph, format_left, tr, theme.track);
            }

        if self.dialog.is_none() {
                if let Some((id, since)) = self.hover_since {
                    if since.elapsed().as_secs_f32() > 0.6 {
                        if let Some(tip) = self.tree.tip(id) {
                            let (rad, pad_x, pad_y) = self.tree.tip_style();
                            let (tw, th) = text_metrics(&self.dwrite, &self.text_format_tip, tip);
                            let bw = tw + pad_x * 2.0;
                            let bh = th + pad_y * 2.0;
                            let bx = (self.mouse.0 + 14.0).min(self.width - bw - 6.0);
                            let by = (self.mouse.1 + 22.0).min(self.height - bh - 6.0);
                            let r = Rect::new(bx.max(4.0), by.max(4.0), bw, bh);
                            canvas.fill_rounded_rect(r, rad, theme.content);
                            canvas.draw_text(tip, &self.text_format_tip, r, theme.surface);
                        }
                    }
                }
            }

            if let Some((text, at, secs)) = &self.toast {
                let e = at.elapsed().as_secs_f32();
                if e <= *secs {
                    let fade = ((*secs - e) / 0.35).clamp(0.0, 1.0);
                    let tw = text_width(&self.dwrite, &self.text_format_tip, text);
                    let bw = (tw + 48.0).min(self.width - 40.0);
                    let bh = 48.0;
                    let r = Rect::new(
                        (self.width - bw) / 2.0,
                        self.height - bh - 28.0,
                        bw,
                        bh,
                    );
                    let a = theme.accent;
                    let fg = theme.on_accent;
                    canvas.fill_rounded_rect(r, 12.0, Color::rgba(a.r, a.g, a.b, fade));
                    canvas.draw_text(text, &self.text_format_tip, r, Color::rgba(fg.r, fg.g, fg.b, fade));
                }
            }

            for (i, r, act) in self.note_rects() {
                let n = &self.notes[i];
                canvas.fill_rounded_rect(r, 12.0, theme.surface);
                canvas.stroke_rect(r, 1.0, theme.track);
                if n.kind == 1 {
                    let tr = Rect::new(r.x + 16.0, r.y, (r.width - 140.0).max(0.0), r.height);
                    canvas.draw_text(&n.text, format_left, tr, theme.content);
                } else {
                    let tr = Rect::new(r.x + 16.0, r.y + 8.0, (r.width - 32.0).max(0.0), 26.0);
                    canvas.draw_text(&n.title, format_left, tr, theme.content);
                    let mr =
                        Rect::new(r.x + 16.0, r.y + 34.0, (r.width - 120.0).max(0.0), 30.0);
                    canvas.draw_text(&n.text, format_left, mr, theme.track);
                }
                if !n.action.is_empty() {
                    canvas.fill_rounded_rect(act, 6.0, theme.accent);
                    canvas.draw_text(&n.action, format, act, theme.on_accent);
                }
            }

        if let Some((panel, btns)) = self.dialog_rects() {
                let backdrop = Rect::new(0.0, 0.0, self.width, self.height);
                canvas.fill_rounded_rect(backdrop, 0.0, DIALOG_SCRIM);
                canvas.fill_rounded_rect(panel, 14.0, theme.surface);
                canvas.stroke_rect(panel, 1.0, theme.track);
                if let Some(d) = self.dialog.as_ref() {
                    let tr = Rect::new(panel.x + 24.0, panel.y + 20.0, panel.width - 48.0, 34.0);
                    canvas.draw_text(&d.title, format_left, tr, theme.content);
                    let mr = Rect::new(panel.x + 24.0, panel.y + 60.0, panel.width - 48.0, 96.0);
                    let content_h = self.dialog_msg_height(mr.width).max(mr.height);
                    let max_scroll = (content_h - mr.height).max(0.0);
                    let sc = d.msg_scroll.clamp(0.0, max_scroll);
                    canvas.push_clip(mr);
                    canvas.draw_text(
                        &d.message,
                        format_wrap,
                        Rect::new(mr.x, mr.y - sc, mr.width, content_h),
                        theme.content,
                    );
                    canvas.pop_clip();
                    if max_scroll > 0.0 {
                        draw_scrollbar(
                            &canvas, mr, content_h, mr.height, sc,
                            theme.track, theme.content,
                        );
                    }
                    for (i, br) in btns.iter().enumerate() {
                        let is_primary = i + 1 == btns.len();
                        let hovered_btn = d.hover == Some(i);
                        let fill = if is_primary {
                            if hovered_btn {
                                theme.accent_hover
                            } else {
                                theme.accent
                            }
                        } else if hovered_btn {
                            theme.track
                        } else {
                            theme.surface
                        };
                        if d.focus == Some(i) {
                            let ring = Rect::new(
                                br.x - 3.0,
                                br.y - 3.0,
                                br.width + 6.0,
                                br.height + 6.0,
                            );
                            canvas.fill_rounded_rect(ring, 13.0, theme.content);
                        }
                        canvas.fill_rounded_rect(*br, 10.0, fill);
                        if !is_primary {
                            canvas.stroke_rect(*br, 1.0, theme.track);
                        }
                        let color = if is_primary {
                            theme.on_accent
                        } else {
                            theme.content
                        };
                        canvas.draw_text(&d.buttons[i], format, *br, color);
                    }
                }
            }
        }
        unsafe {
            let _ = self.rt.EndDraw(None, None);
            let _ = self.swap_chain.Present(1, DXGI_PRESENT(0));
        }
    }
}

fn key_down(vk: i32) -> bool {
    unsafe { GetKeyState(vk) < 0 }
}

fn merge_style(base: &mut Style, over: &Style) {
    if over.fill.is_some() {
        base.fill = over.fill;
    }
    if over.text.is_some() {
        base.text = over.text;
    }
    if over.radius.is_some() {
        base.radius = over.radius;
    }
    if over.wrap.is_some() {
        base.wrap = over.wrap;
    }
    if over.elev.is_some() {
        base.elev = over.elev;
    }
    if over.grad.is_some() {
        base.grad = over.grad;
        base.grad_dir = over.grad_dir;
    }
}

fn draw_soft_shadow(canvas: &Canvas, rect: Rect, radius: f32, elev: f32) {
    let layers = 6;
    let dy = elev * 0.4;
    for i in 0..layers {
        let t = i as f32;
        let spread = elev * (1.0 - t / layers as f32);
        let a = 0.10 * (t + 1.0) / layers as f32;
        let r = Rect::new(
            rect.x - spread,
            rect.y - spread + dy,
            rect.width + spread * 2.0,
            rect.height + spread * 2.0,
        );
        canvas.fill_rounded_rect(r, radius + spread, Color::rgba(0.0, 0.0, 0.0, a));
    }
}

const MENU_ROW: f32 = 30.0;
const DIALOG_SCRIM: Color = Color::rgba(0.0, 0.0, 0.0, 0.5);
const INSPECT_LINE: Color = Color::rgba(1.0, 0.2, 0.6, 0.9);
const INSPECT_FILL: Color = Color::rgba(1.0, 0.2, 0.6, 0.18);
const INSPECT_LABEL_BG: Color = Color::rgba(0.0, 0.0, 0.0, 0.85);
const INSPECT_LABEL_FG: Color = Color::rgba(1.0, 1.0, 1.0, 1.0);

fn kind_name(kind: &NodeKind) -> &'static str {
    crate::tree::kind_tag(kind)
}

#[allow(clippy::too_many_arguments)]
fn draw_canvas_bars(
    canvas: &Canvas,
    view: Rect,
    rw: f32,
    rh: f32,
    ox: f32,
    oy: f32,
    track_col: Color,
    thumb_col: Color,
) {
    if rh > view.height && view.height > 0.0 {
        let bar = Rect::new(
            view.x + view.width - SCROLLBAR_W - 2.0,
            view.y + 2.0,
            SCROLLBAR_W,
            (view.height - 4.0).max(0.0),
        );
        canvas.fill_rounded_rect(bar, SCROLLBAR_W / 2.0, track_col);
        let th = (bar.height * (view.height / rh)).max(24.0);
        let t = (oy / (rh - view.height)).clamp(0.0, 1.0);
        let ty = bar.y + (bar.height - th) * t;
        canvas.fill_rounded_rect(
            Rect::new(bar.x, ty, SCROLLBAR_W, th),
            SCROLLBAR_W / 2.0,
            thumb_col,
        );
    }
    if rw > view.width && view.width > 0.0 {
        let bar = Rect::new(
            view.x + 2.0,
            view.y + view.height - SCROLLBAR_W - 2.0,
            (view.width - 4.0).max(0.0),
            SCROLLBAR_W,
        );
        canvas.fill_rounded_rect(bar, SCROLLBAR_W / 2.0, track_col);
        let tw = (bar.width * (view.width / rw)).max(24.0);
        let t = (ox / (rw - view.width)).clamp(0.0, 1.0);
        let tx = bar.x + (bar.width - tw) * t;
        canvas.fill_rounded_rect(
            Rect::new(tx, bar.y, tw, SCROLLBAR_W),
            SCROLLBAR_W / 2.0,
            thumb_col,
        );
    }
}

fn draw_scrollbar(
    canvas: &Canvas,
    track: Rect,
    content: f32,
    visible: f32,
    scroll: f32,
    track_col: Color,
    thumb_col: Color,
) {
    if content <= visible || visible <= 0.0 {
        return;
    }
    let bar = Rect::new(
        track.x + track.width - SCROLLBAR_W - 2.0,
        track.y + 2.0,
        SCROLLBAR_W,
        (track.height - 4.0).max(0.0),
    );
    canvas.fill_rounded_rect(bar, SCROLLBAR_W / 2.0, track_col);
    let ratio = (visible / content).clamp(0.05, 1.0);
    let th = (bar.height * ratio).max(24.0);
    let max_scroll = (content - visible).max(1.0);
    let t = (scroll / max_scroll).clamp(0.0, 1.0);
    let ty = bar.y + (bar.height - th) * t;
    canvas.fill_rounded_rect(
        Rect::new(bar.x, ty, SCROLLBAR_W, th),
        SCROLLBAR_W / 2.0,
        thumb_col,
    );
}

fn theme_from_index(index: usize) -> Theme {
    match index {
        0 => Theme::white(),
        1 => Theme::light(),
        2 => Theme::dark(),
        _ => Theme::black(),
    }
}

/// Разбирает `путь|кадр` на файл и номер кадра.
fn split_frame(spec: &str) -> (&str, u32) {
    match spec.rsplit_once('|') {
        Some((p, n)) => (p, n.parse().unwrap_or(0)),
        None => (spec, 0),
    }
}

fn load_bitmap(
    wic: &IWICImagingFactory,
    rt: &ID2D1RenderTarget,
    spec: &str,
) -> Option<ID2D1Bitmap> {
    unsafe {
        let (path, index) = split_frame(spec);
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let decoder = wic
            .CreateDecoderFromFilename(
                PCWSTR(wide.as_ptr()),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnLoad,
            )
            .ok()?;
        let total = decoder.GetFrameCount().unwrap_or(1);
        let frame = decoder.GetFrame(index.min(total.saturating_sub(1))).ok()?;
        let conv = wic.CreateFormatConverter().ok()?;
        conv.Initialize(
            &frame,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeMedianCut,
        )
        .ok()?;
        rt.CreateBitmapFromWicBitmap(&conv, None).ok()
    }
}

fn load_bitmap_mem(
    wic: &IWICImagingFactory,
    rt: &ID2D1RenderTarget,
    data: &mut [u8],
) -> Option<ID2D1Bitmap> {
    unsafe {
        let stream = wic.CreateStream().ok()?;
        stream.InitializeFromMemory(data).ok()?;
        let decoder = wic
            .CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnLoad)
            .ok()?;
        let frame = decoder.GetFrame(0).ok()?;
        let conv = wic.CreateFormatConverter().ok()?;
        conv.Initialize(
            &frame,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeMedianCut,
        )
        .ok()?;
        rt.CreateBitmapFromWicBitmap(&conv, None).ok()
    }
}

fn index_at_x(dwrite: &IDWriteFactory, format: &IDWriteTextFormat, text: &[u16], x: f32) -> usize {
    if text.is_empty() {
        return 0;
    }
    unsafe {
        if let Ok(layout) = dwrite.CreateTextLayout(text, format, 100000.0, 100.0) {
            let mut trailing = BOOL(0);
            let mut inside = BOOL(0);
            let mut m = DWRITE_HIT_TEST_METRICS::default();
            if layout
                .HitTestPoint(x.max(0.0), 0.0, &mut trailing, &mut inside, &mut m)
                .is_ok()
            {
                let mut idx = m.textPosition as usize;
                if trailing.as_bool() {
                    idx += 1;
                }
                return idx.min(text.len());
            }
        }
    }
    text.len()
}

fn x_at_index(dwrite: &IDWriteFactory, format: &IDWriteTextFormat, text: &[u16], index: usize) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    unsafe {
        if let Ok(layout) = dwrite.CreateTextLayout(text, format, 100000.0, 100.0) {
            let mut px = 0.0f32;
            let mut py = 0.0f32;
            let mut m = DWRITE_HIT_TEST_METRICS::default();
            if layout
                .HitTestTextPosition(index as u32, false, &mut px, &mut py, &mut m)
                .is_ok()
            {
                return px;
            }
        }
    }
    0.0
}

fn pick_format(
    dwrite: &IDWriteFactory,
    cache: &std::cell::RefCell<HashMap<(u16, u32, u8), IDWriteTextFormat>>,
    style: crate::tree::Style,
    slot: u8,
    weight: DWRITE_FONT_WEIGHT,
    default_size: f32,
) -> IDWriteTextFormat {
    let fam = style.font.unwrap_or(0);
    let size = style.size.unwrap_or(default_size).max(1.0);
    let key = (fam, size.to_bits(), slot);
    if let Some(f) = cache.borrow().get(&key) {
        return f.clone();
    }
    let name = crate::tree::font_utf16(fam);
    let made = unsafe {
        dwrite.CreateTextFormat(
            windows::core::PCWSTR(name.as_ptr()),
            None,
            weight,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            size,
            w!("en-us"),
        )
    };
    let fmt = match made {
        Ok(f) => f,
        Err(_) => unsafe {
            dwrite
                .CreateTextFormat(
                    w!("Segoe UI"),
                    None,
                    weight,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    size,
                    w!("en-us"),
                )
                .expect("segoe")
        },
    };
    unsafe {
        match slot {
            0 => {
                let _ = fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
                let _ = fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            }
            2 => {
                let _ = fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
                let _ = fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR);
                let _ = fmt.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP);
            }
            _ => {
                let _ = fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
                let _ = fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            }
        }
    }
    cache.borrow_mut().insert(key, fmt.clone());
    fmt
}

fn text_width(dwrite: &IDWriteFactory, format: &IDWriteTextFormat, text: &[u16]) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    unsafe {
        if let Ok(layout) = dwrite.CreateTextLayout(text, format, 100000.0, 100.0) {
            let mut m = DWRITE_TEXT_METRICS::default();
            if layout.GetMetrics(&mut m).is_ok() {
                return m.width;
            }
        }
    }
    0.0
}

fn text_metrics(dwrite: &IDWriteFactory, format: &IDWriteTextFormat, text: &[u16]) -> (f32, f32) {
    if text.is_empty() {
        return (0.0, 0.0);
    }
    unsafe {
        if let Ok(layout) = dwrite.CreateTextLayout(text, format, 100000.0, 100.0) {
            let mut m = DWRITE_TEXT_METRICS::default();
            if layout.GetMetrics(&mut m).is_ok() {
                return (m.width, m.height);
            }
        }
    }
    (0.0, 0.0)
}

fn wrapped_caret(
    dwrite: &IDWriteFactory,
    format: &IDWriteTextFormat,
    text: &[u16],
    width: f32,
    index: usize,
) -> (f32, f32, f32) {
    unsafe {
        if let Ok(layout) = dwrite.CreateTextLayout(text, format, width.max(1.0), 100000.0) {
            let mut px = 0.0f32;
            let mut py = 0.0f32;
            let mut m = DWRITE_HIT_TEST_METRICS::default();
            if layout
                .HitTestTextPosition(index as u32, false, &mut px, &mut py, &mut m)
                .is_ok()
            {
                let lh = if m.height > 0.0 { m.height } else { 22.0 };
                return (px, py, lh);
            }
        }
    }
    (0.0, 0.0, 22.0)
}

fn wrapped_index(
    dwrite: &IDWriteFactory,
    format: &IDWriteTextFormat,
    text: &[u16],
    width: f32,
    x: f32,
    y: f32,
) -> usize {
    if text.is_empty() {
        return 0;
    }
    unsafe {
        if let Ok(layout) = dwrite.CreateTextLayout(text, format, width.max(1.0), 100000.0) {
            let mut trailing = BOOL(0);
            let mut inside = BOOL(0);
            let mut m = DWRITE_HIT_TEST_METRICS::default();
            if layout
                .HitTestPoint(x.max(0.0), y.max(0.0), &mut trailing, &mut inside, &mut m)
                .is_ok()
            {
                let mut idx = m.textPosition as usize;
                if trailing.as_bool() {
                    idx += 1;
                }
                return idx.min(text.len());
            }
        }
    }
    text.len()
}

fn to_crlf(text: &[u16]) -> Vec<u16> {
    let mut out = Vec::with_capacity(text.len() + 8);
    for &c in text {
        if c == 0x0A {
            out.push(0x0D);
        }
        out.push(c);
    }
    out
}

pub fn set_clipboard_text(text: &[u16]) {
    unsafe {
        if OpenClipboard(None).is_err() {
            return;
        }
        let _ = EmptyClipboard();
        let count = text.len() + 1;
        let bytes = count * std::mem::size_of::<u16>();
        if let Ok(hmem) = GlobalAlloc(GMEM_MOVEABLE, bytes) {
            let ptr = GlobalLock(hmem) as *mut u16;
            if !ptr.is_null() {
                std::ptr::copy_nonoverlapping(text.as_ptr(), ptr, text.len());
                *ptr.add(text.len()) = 0;
                let _ = GlobalUnlock(hmem);
                let _ = SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(hmem.0)));
            }
        }
        let _ = CloseClipboard();
    }
}

pub fn get_clipboard_text() -> Vec<u16> {
    let mut out = Vec::new();
    unsafe {
        if OpenClipboard(None).is_err() {
            return out;
        }
        if let Ok(handle) = GetClipboardData(CF_UNICODETEXT.0 as u32) {
            let hmem = HGLOBAL(handle.0);
            let ptr = GlobalLock(hmem) as *const u16;
            if !ptr.is_null() {
                let mut i = 0isize;
                loop {
                    let c = *ptr.offset(i);
                    if c == 0 {
                        break;
                    }
                    out.push(c);
                    i += 1;
                    if i > 1_000_000 {
                        break;
                    }
                }
                let _ = GlobalUnlock(hmem);
            }
        }
        let _ = CloseClipboard();
    }
    out
}
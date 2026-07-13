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
    NodeId, NodeKind, Style, Tree, ACC_HEADER, GROUP_HEADER, LIST_ROW, SCROLLBAR_W, SPLIT_W,
    TABLE_HEADER, TABLE_ROW, TAB_HEADER,
};

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
    tree: Tree,
    width: f32,
    height: f32,
    hovered: Option<NodeId>,
    pressed: Option<NodeId>,
    dragging: Option<NodeId>,
    focused: Option<NodeId>,
    hot: Option<NodeId>,
    text_selecting: bool,
    scroll_drag: Option<NodeId>,
    split_drag: Option<NodeId>,
    hover_since: Option<(NodeId, Instant)>,
    toast: Option<(Vec<u16>, Instant, f32)>,
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
                    Flags: 0,
                };
                factory.CreateSwapChainForHwnd(&device, hwnd, &desc, None, None)?
            };
            let _ = factory.MakeWindowAssociation(hwnd, DXGI_MWA_NO_ALT_ENTER);

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
            let rt: ID2D1RenderTarget = context.cast()?;

            let dwrite: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let text_format = dwrite.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                24.0,
                w!("en-us"),
            )?;
            let _ = text_format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
            let _ = text_format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);

            let text_format_left = dwrite.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                20.0,
                w!("en-us"),
            )?;
            let _ = text_format_left.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
            let _ = text_format_left.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);

            let text_format_wrap = dwrite.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                20.0,
                w!("en-us"),
            )?;
            let _ = text_format_wrap.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
            let _ = text_format_wrap.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR);
            let _ = text_format_wrap.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP);

            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let wic: Option<IWICImagingFactory> =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok();

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
                tree,
                width: 1280.0,
                height: 720.0,
                hovered: None,
                pressed: None,
                dragging: None,
                focused: None,
                hot: None,
                text_selecting: false,
                scroll_drag: None,
                split_drag: None,
                hover_since: None,
                toast: None,
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
        unsafe {
            self.context.SetTarget(None);
            self.target = None;
            if self
                .swap_chain
                .ResizeBuffers(0, width, height, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(0))
                .is_err()
            {
                return;
            }
            let _ = self.create_target();
        }
    }

    /// Продвигает анимации по таймеру; true, если нужна перерисовка.
    pub fn on_timer(&mut self) -> bool {
        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f32();
        self.last_tick = now;
        let anim = self.tree.tick(dt);
        let spin = self.tree.has_spinner();
        if spin {
            self.tree.spin(dt);
        }
        anim || spin || self.toast.is_some() || self.tip_pending()
    }

    fn tip_pending(&self) -> bool {
        match self.hover_since {
            Some((id, since)) => {
                self.tree.tip(id).is_some() && since.elapsed().as_secs_f32() < 1.2
            }
            None => false,
        }
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

    /// Прокручивает таблицу под курсором колесом мыши.
    pub fn on_wheel(&mut self, delta: i32) -> bool {
        if let Some(mut id) = self.hot {
            let mut guard = 0;
            while !self.tree.is_scroll(id)
                && !self.tree.is_list(id)
                && !self.tree.is_table(id)
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

    fn splitter_bar_at(&self, x: f32, y: f32) -> Option<NodeId> {
        let mut found = None;
        self.tree.for_each(|id, node| {
            if !matches!(node.kind, NodeKind::Splitter { .. }) {
                return;
            }
            let r = node.rect;
            if r.x <= -100000.0 {
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
        let n = self.tree.menu_len();
        if n == 0 {
            return false;
        }
        self.close_dropdown();
        let mw = 220.0;
        let mh = n as f32 * MENU_ROW;
        let mx = x.min((self.width - mw).max(0.0));
        let my = y.min((self.height - mh).max(0.0));
        self.open_menu = Some((mx, my));
        self.menu_hover = None;
        true
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

    /// Тип курсора под текущим положением мыши.
    pub fn cursor_kind(&self) -> CursorKind {
        match self.hot {
            Some(id) if self.tree.is_interactive(id) => CursorKind::Hand,
            Some(id) if self.tree.is_dropdown(id) => CursorKind::Hand,
            Some(id) if self.tree.is_tabs(id) => CursorKind::Hand,
            Some(id) if self.tree.is_accordion(id) => CursorKind::Hand,
            Some(id) if self.tree.is_textbox(id) => CursorKind::IBeam,
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
        if self.dialog.is_some() {
            if let Some(i) = self.dialog_button_at(x, y) {
                self.dialog = None;
                self.tree.fire_dialog(i);
            }
            return true;
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
        let hit = self.tree.hit_test(x, y);
        let new_focus = hit.filter(|&id| self.tree.is_textbox(id) || self.tree.is_slider(id));
        self.focused = new_focus;

        if let Some(id) = self.splitter_bar_at(x, y) {
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
                    self.tree.set_list_selected(id, Some(ri));
                    self.tree.fire_change(id, ri as f32);
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
            if self.tree.is_table(id) {
                if let Some(ri) = self.table_row_at(id, y) {
                    self.tree.set_table_selected(id, Some(ri));
                    self.tree.fire_change(id, ri as f32);
                }
                self.focused = Some(id);
                return true;
            }
            if self.tree.is_textbox(id) {
                let idx = if self.tree.is_multiline(id) {
                    self.textarea_index_at(id, x, y)
                } else {
                    self.textbox_index_at(id, x)
                };
                if let Some(st) = self.tree.textbox_state_mut(id) {
                    st.set_caret(idx, false);
                }
                self.text_selecting = true;
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
        let was_selecting = self.text_selecting;
        self.text_selecting = false;
        if let Some(id) = click_id {
            self.dispatch(id);
        }
        was_pressed || was_dragging || was_scroll || was_split || was_selecting || click_id.is_some()
    }

    /// Обрабатывает символьный ввод. Возвращает true, если нужна перерисовка.
    pub fn on_char(&mut self, ch: u16) -> bool {
        const BACKSPACE: u16 = 0x08;
        if let Some(id) = self.focused {
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
                if let Some(i) = idx {
                    self.tree.fire_dialog(i);
                }
                return true;
            }
            return true;
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
            self.focused = None;
            return true;
        }

        if let Some(id) = self.focused {
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
                                    let sel: Vec<u16> = st.text[a..b].to_vec();
                                    set_clipboard_text(&sel);
                                }
                                handled = true;
                            }
                            KEY_X => {
                                let (a, b) = st.sel_range();
                                if a != b {
                                    let sel: Vec<u16> = st.text[a..b].to_vec();
                                    set_clipboard_text(&sel);
                                    st.backspace();
                                    changed = true;
                                }
                                handled = true;
                            }
                            KEY_V => {
                                let clip = get_clipboard_text();
                                let filtered: Vec<u16> =
                                    clip.into_iter().filter(|&c| c >= 0x20).collect();
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

        if vk == VK_SPACE && self.focused.is_none() {
            self.theme_index = (self.theme_index + 1) % 4;
            self.theme = theme_from_index(self.theme_index);
            return true;
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
        let wic = match &self.wic {
            Some(w) => w.clone(),
            None => return,
        };
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
        for p in paths {
            if self.img_cache.contains_key(&p) {
                continue;
            }
            let bmp = load_bitmap(&wic, &self.rt, &p);
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

    fn poll_dialog(&mut self) {
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
            });
        }
    }

    pub fn render(&mut self) {
        self.poll_dialog();
        self.poll_toast();
        self.tree.layout(Rect::new(0.0, 0.0, self.width, self.height));
        self.update_scroll();
        self.preload_images();
        let hovered = self.hovered;
        let pressed = self.pressed;
        let focused = self.focused;
        let hot = self.hot;
        let mouse = self.mouse;
        let theme = self.theme;
        unsafe {
            self.rt.BeginDraw();
        }
        {
            let canvas = Canvas::new(&self.rt);
            let format = &self.text_format;
            let format_left = &self.text_format_left;
            let format_wrap = &self.text_format_wrap;
            let img_cache = &self.img_cache;
            let dwrite = &self.dwrite;
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
            self.tree.for_each(|id, node| {
                if node.rect.x <= -100000.0 || node.rect.y <= -100000.0 {
                    return;
                }
                let mut style = node.style;
                if focused == Some(id) {
                    merge_style(&mut style, &node.style_focus);
                }
                let is_button = matches!(node.kind, NodeKind::Button { .. });
                if hot == Some(id) && !is_button {
                    merge_style(&mut style, &node.style_hover);
                }
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
                        if let Some(e) = style.elev {
                            if e > 0.0 {
                                draw_soft_shadow(&canvas, node.rect, rad, e);
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
                        let mut tr = node.rect;
                        if let Some(icon) = &node.icon {
                            if let Some(Some(bmp)) = img_cache.get(icon) {
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
                        canvas.draw_text(label, format, tr, text_color);
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
                        let knob_color = if focused == Some(id) { theme.accent } else { theme.content };
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
                        if focused == Some(id) {
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
                        for (i, item) in items.iter().enumerate() {
                            let iy = r.y - *scroll + LIST_ROW * i as f32;
                            if iy + LIST_ROW <= r.y || iy >= r.y + r.height {
                                continue;
                            }
                            let row_rect = Rect::new(r.x, iy, r.width, LIST_ROW);
                            if *selected == Some(i) {
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
                        if focused == Some(id) {
                            canvas.stroke_rect(r, 2.0, theme.accent);
                        }
                        canvas.pop_clip();
                    }
                    NodeKind::TextBox { state } => {
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
                        let border = if focused == Some(id) { theme.accent } else { theme.track };
                        let fill = style.fill.unwrap_or(theme.surface);
                        canvas.fill_rounded_rect(r, 8.0, fill);
                        canvas.stroke_rect(r, if focused == Some(id) { 2.0 } else { 1.0 }, border);
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
                        if focused == Some(id) {
                            canvas.stroke_rect(header, 2.0, theme.accent);
                        }
                    }
                    NodeKind::Table {
                        columns,
                        rows,
                        selected,
                        scroll,
                    } => {
                        let r = node.rect;
                        canvas.push_clip(r);
                        let bg = style.fill.unwrap_or(theme.surface);
                        canvas.fill_rounded_rect(r, 8.0, bg);
                        let ncol = columns.len().max(1);
                        let col_w = r.width / ncol as f32;
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
                        for (ri, row) in rows.iter().enumerate() {
                            let ry = top - *scroll + TABLE_ROW * ri as f32;
                            if ry + TABLE_ROW <= top || ry >= r.y + r.height {
                                continue;
                            }
                            let row_rect = Rect::new(r.x, ry, r.width, TABLE_ROW);
                            if *selected == Some(ri) {
                                canvas.fill_rounded_rect(row_rect, 0.0, theme.selection);
                            } else if hover_row == Some(ri) {
                                canvas.fill_rounded_rect(row_rect, 0.0, theme.track);
                            }
                            for (c, cell) in row.iter().enumerate() {
                                let cx = r.x + col_w * c as f32;
                                let cr = Rect::new(
                                    cx + 10.0,
                                    ry,
                                    (col_w - 20.0).max(0.0),
                                    TABLE_ROW,
                                );
                                canvas.draw_text(cell, format_left, cr, theme.content);
                            }
                        }
                        let header = Rect::new(r.x, r.y, r.width, TABLE_HEADER);
                        canvas.fill_rounded_rect(header, 8.0, theme.track);
                        for (c, col) in columns.iter().enumerate() {
                            let cx = r.x + col_w * c as f32;
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
                        if focused == Some(id) {
                            canvas.stroke_rect(r, 2.0, theme.accent);
                        }
                        canvas.pop_clip();
                    }
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
        if self.dialog.is_none() {
                if let Some((id, since)) = self.hover_since {
                    if since.elapsed().as_secs_f32() > 0.6 {
                        if let Some(tip) = self.tree.tip(id) {
                            let tw = text_width(&self.dwrite, format_left, tip);
                            let bw = tw + 24.0;
                            let bh = 30.0;
                            let bx = (self.mouse.0 + 14.0).min(self.width - bw - 6.0);
                            let by = (self.mouse.1 + 22.0).min(self.height - bh - 6.0);
                            let r = Rect::new(bx.max(4.0), by.max(4.0), bw, bh);
                            canvas.fill_rounded_rect(r, 6.0, theme.content);
                            canvas.draw_text(tip, format, r, theme.surface);
                        }
                    }
                }
            }

            if let Some((text, at, secs)) = &self.toast {
                let e = at.elapsed().as_secs_f32();
                if e <= *secs {
                    let fade = ((*secs - e) / 0.35).clamp(0.0, 1.0);
                    let tw = text_width(&self.dwrite, format_left, text);
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
                    canvas.draw_text(text, format, r, Color::rgba(fg.r, fg.g, fg.b, fade));
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
                    canvas.draw_text(&d.message, format_wrap, mr, theme.content);
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
    match kind {
        NodeKind::Container => "Container",
        NodeKind::Frame { .. } => "Frame",
        NodeKind::Label { .. } => "Label",
        NodeKind::Button { .. } => "Button",
        NodeKind::Slider { .. } => "Slider",
        NodeKind::Progress { .. } => "Progress",
        NodeKind::Checkbox { .. } => "Checkbox",
        NodeKind::TextBox { .. } => "TextBox",
        NodeKind::Dropdown { .. } => "Dropdown",
        NodeKind::Tabs { .. } => "Tabs",
        NodeKind::Table { .. } => "Table",
        NodeKind::Image { .. } => "Image",
        NodeKind::Switch { .. } => "Switch",
        NodeKind::Radio { .. } => "Radio",
        NodeKind::Toggle { .. } => "Toggle",
        NodeKind::Separator { .. } => "Separator",
        NodeKind::List { .. } => "List",
        NodeKind::Group { .. } => "Group",
        NodeKind::Link { .. } => "Link",
        NodeKind::Accordion { .. } => "Accordion",
        NodeKind::Scroll { .. } => "Scroll",
        NodeKind::Stack { .. } => "Stack",
        NodeKind::Splitter { .. } => "Splitter",
        NodeKind::Spinner { .. } => "Spinner",
        NodeKind::Gauge { .. } => "Gauge",
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

fn load_bitmap(
    wic: &IWICImagingFactory,
    rt: &ID2D1RenderTarget,
    path: &str,
) -> Option<ID2D1Bitmap> {
    unsafe {
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let decoder = wic
            .CreateDecoderFromFilename(
                PCWSTR(wide.as_ptr()),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnLoad,
            )
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

fn set_clipboard_text(text: &[u16]) {
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

fn get_clipboard_text() -> Vec<u16> {
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
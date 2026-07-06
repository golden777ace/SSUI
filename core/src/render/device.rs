use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Ole::CF_UNICODETEXT;

use super::canvas::Canvas;
use super::types::{Color, Rect};
use crate::theme::Theme;
use crate::tree::{Action, Axis, NodeId, NodeKind, Props, Style, Tree};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CursorKind {
    Arrow,
    Hand,
    IBeam,
}

pub struct Renderer {
    swap_chain: IDXGISwapChain1,
    context: ID2D1DeviceContext,
    rt: ID2D1RenderTarget,
    target: Option<ID2D1Bitmap1>,
    dwrite: IDWriteFactory,
    text_format: IDWriteTextFormat,
    text_format_left: IDWriteTextFormat,
    tree: Tree,
    width: f32,
    height: f32,
    hovered: Option<NodeId>,
    pressed: Option<NodeId>,
    dragging: Option<NodeId>,
    focused: Option<NodeId>,
    hot: Option<NodeId>,
    text_selecting: bool,
    counter: i32,
    count_label: NodeId,
    value_label: NodeId,
    theme: Theme,
    theme_index: usize,
}

impl Renderer {
    /// Создаёт рендерер Direct2D, привязанный к окну `hwnd`.
    pub fn new(hwnd: HWND) -> Result<Self> {
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
            let swap_chain: IDXGISwapChain1 =
                factory.CreateSwapChainForHwnd(&device, hwnd, &desc, None, None)?;
            let _ = factory.MakeWindowAssociation(hwnd, DXGI_MWA_NO_ALT_ENTER);

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

            let mut tree = Tree::new();
            let root = tree.root();
            tree.set_props(
                root,
                Props {
                    axis: Axis::Vertical,
                    padding: 24.0,
                    gap: 16.0,
                    width: None,
                    height: None,
                },
            );
            let panel = tree.add_child(
                root,
                NodeKind::Frame { radius: 16.0 },
                Props {
                    axis: Axis::Vertical,
                    padding: 20.0,
                    gap: 12.0,
                    width: None,
                    height: None,
                },
            );
            let count_label = tree.add_child(
                panel,
                NodeKind::Label {
                    text: "Clicks: 0".encode_utf16().collect(),
                },
                Props {
                    height: Some(40.0),
                    ..Default::default()
                },
            );

            let row = tree.add_child(
                panel,
                NodeKind::Container,
                Props {
                    axis: Axis::Horizontal,
                    gap: 12.0,
                    height: Some(48.0),
                    ..Default::default()
                },
            );
            let minus = tree.add_child(
                row,
                NodeKind::Button {
                    label: "-".encode_utf16().collect(),
                    radius: 10.0,
                },
                Props {
                    width: Some(64.0),
                    ..Default::default()
                },
            );
            tree.set_action(minus, Action::Decrement);
            tree.set_style(
                minus,
                Style {
                    fill: Some(Color::hex(0xE5484D)),
                    text: Some(Color::hex(0xFFFFFF)),
                },
            );
            let plus = tree.add_child(
                row,
                NodeKind::Button {
                    label: "+".encode_utf16().collect(),
                    radius: 10.0,
                },
                Props {
                    width: Some(64.0),
                    ..Default::default()
                },
            );
            tree.set_action(plus, Action::Increment);
            tree.set_style(
                plus,
                Style {
                    fill: Some(Color::hex(0x2FBF71)),
                    text: Some(Color::hex(0xFFFFFF)),
                },
            );

            let checkbox = tree.add_child(
                panel,
                NodeKind::Checkbox {
                    label: "Enable feature".encode_utf16().collect(),
                    checked: false,
                },
                Props {
                    height: Some(28.0),
                    ..Default::default()
                },
            );
            tree.set_action(checkbox, Action::Toggle);

            tree.add_child(
                panel,
                NodeKind::Label {
                    text: "Type in the box:".encode_utf16().collect(),
                },
                Props {
                    height: Some(24.0),
                    ..Default::default()
                },
            );
            tree.add_child(
                panel,
                NodeKind::TextBox {
                    state: crate::tree::TextState::new(),
                },
                Props {
                    height: Some(44.0),
                    width: Some(280.0),
                    ..Default::default()
                },
            );

            tree.add_child(
                panel,
                NodeKind::Label {
                    text: "Press Space to cycle themes".encode_utf16().collect(),
                },
                Props {
                    height: Some(28.0),
                    ..Default::default()
                },
            );
            let value_label = tree.add_child(
                panel,
                NodeKind::Label {
                    text: "Value: 50%".encode_utf16().collect(),
                },
                Props {
                    height: Some(28.0),
                    ..Default::default()
                },
            );
            tree.add_child(
                panel,
                NodeKind::Slider { value: 0.5 },
                Props {
                    height: Some(40.0),
                    width: Some(240.0),
                    ..Default::default()
                },
            );

            let mut renderer = Renderer {
                swap_chain,
                context,
                rt,
                target: None,
                dwrite,
                text_format,
                text_format_left,
                tree,
                width: 1280.0,
                height: 720.0,
                hovered: None,
                pressed: None,
                dragging: None,
                focused: None,
                hot: None,
                text_selecting: false,
                counter: 0,
                count_label,
                value_label,
                theme: Theme::dark(),
                theme_index: 2,
            };
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
                    alphaMode: D2D1_ALPHA_MODE_IGNORE,
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

    /// Тип курсора под текущим положением мыши.
    pub fn cursor_kind(&self) -> CursorKind {
        match self.hot {
            Some(id) if self.tree.is_interactive(id) => CursorKind::Hand,
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

    /// Обрабатывает движение мыши. Возвращает true, если нужна перерисовка.
    pub fn on_mouse_move(&mut self, x: f32, y: f32) -> bool {
        let mut dirty = false;
        let hit = self.tree.hit_test(x, y);
        self.hot = hit;
        let hover = hit.filter(|&id| self.tree.is_interactive(id));
        if hover != self.hovered {
            self.hovered = hover;
            dirty = true;
        }
        if let Some(id) = self.dragging {
            self.set_slider_from_x(id, x);
            dirty = true;
        }
        if self.text_selecting {
            if let Some(id) = self.focused {
                let idx = self.textbox_index_at(id, x);
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
        self.text_selecting = false;
        let hit = self.tree.hit_test(x, y);
        let new_focus = hit.filter(|&id| self.tree.is_textbox(id));
        self.focused = new_focus;

        if let Some(id) = hit {
            if self.tree.is_textbox(id) {
                let idx = self.textbox_index_at(id, x);
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
        let was_selecting = self.text_selecting;
        self.text_selecting = false;
        if let Some(id) = click_id {
            self.dispatch(id);
        }
        was_pressed || was_dragging || was_selecting || click_id.is_some()
    }

    /// Обрабатывает символьный ввод. Возвращает true, если нужна перерисовка.
    pub fn on_char(&mut self, ch: u16) -> bool {
        const BACKSPACE: u16 = 0x08;
        if let Some(id) = self.focused {
            if let Some(st) = self.tree.textbox_state_mut(id) {
                if ch == BACKSPACE {
                    st.backspace();
                    return true;
                }
                if ch >= 0x20 {
                    st.insert(&[ch]);
                    return true;
                }
            }
        }
        false
    }

    /// Обрабатывает нажатие клавиши. Возвращает true, если нужна перерисовка.
    pub fn on_key(&mut self, vk: u32) -> bool {
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

        if let Some(id) = self.focused {
            if self.tree.is_textbox(id) {
                let shift = key_down(0x10);
                let ctrl = key_down(0x11);
                if let Some(st) = self.tree.textbox_state_mut(id) {
                    match vk {
                        VK_LEFT => {
                            st.move_left(shift);
                            return true;
                        }
                        VK_RIGHT => {
                            st.move_right(shift);
                            return true;
                        }
                        VK_HOME => {
                            st.home(shift);
                            return true;
                        }
                        VK_END => {
                            st.end(shift);
                            return true;
                        }
                        VK_DELETE => {
                            st.delete_forward();
                            return true;
                        }
                        _ => {}
                    }
                    if ctrl {
                        match vk {
                            KEY_A => {
                                st.select_all();
                                return true;
                            }
                            KEY_Z => {
                                st.undo();
                                return true;
                            }
                            KEY_Y => {
                                st.redo();
                                return true;
                            }
                            KEY_C => {
                                let (a, b) = st.sel_range();
                                if a != b {
                                    let sel: Vec<u16> = st.text[a..b].to_vec();
                                    set_clipboard_text(&sel);
                                }
                                return true;
                            }
                            KEY_X => {
                                let (a, b) = st.sel_range();
                                if a != b {
                                    let sel: Vec<u16> = st.text[a..b].to_vec();
                                    set_clipboard_text(&sel);
                                    st.backspace();
                                }
                                return true;
                            }
                            KEY_V => {
                                let clip = get_clipboard_text();
                                let filtered: Vec<u16> =
                                    clip.into_iter().filter(|&c| c >= 0x20).collect();
                                if !filtered.is_empty() {
                                    st.insert(&filtered);
                                }
                                return true;
                            }
                            _ => {}
                        }
                    }
                }
                return false;
            }
        }

        if vk == VK_SPACE && self.focused.is_none() {
            self.theme_index = (self.theme_index + 1) % 4;
            self.theme = match self.theme_index {
                0 => Theme::white(),
                1 => Theme::light(),
                2 => Theme::dark(),
                _ => Theme::black(),
            };
            return true;
        }
        false
    }

    fn dispatch(&mut self, id: NodeId) {
        match self.tree.get_action(id) {
            Action::Increment => {
                self.counter += 1;
                self.update_count();
            }
            Action::Decrement => {
                self.counter -= 1;
                self.update_count();
            }
            Action::Toggle => self.tree.toggle_checkbox(id),
            Action::None => {}
        }
    }

    fn update_count(&mut self) {
        let text: Vec<u16> = format!("Clicks: {}", self.counter).encode_utf16().collect();
        self.tree.set_label_text(self.count_label, text);
    }

    fn set_slider_from_x(&mut self, id: NodeId, x: f32) {
        let rect = self.tree.get(id).rect;
        if rect.width <= 0.0 {
            return;
        }
        let value = ((x - rect.x) / rect.width).clamp(0.0, 1.0);
        self.tree.set_slider_value(id, value);
        let percent = (value * 100.0).round() as i32;
        let text: Vec<u16> = format!("Value: {}%", percent).encode_utf16().collect();
        self.tree.set_label_text(self.value_label, text);
    }

    fn update_scroll(&mut self) {
        if let Some(id) = self.focused {
            if self.tree.is_textbox(id) {
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
    pub fn render(&mut self) {
        self.tree.layout(Rect::new(0.0, 0.0, self.width, self.height));
        self.update_scroll();
        let hovered = self.hovered;
        let pressed = self.pressed;
        let focused = self.focused;
        let theme = self.theme;
        let dwrite = &self.dwrite;
        unsafe {
            self.rt.BeginDraw();
        }
        {
            let canvas = Canvas::new(&self.rt);
            let format = &self.text_format;
            let format_left = &self.text_format_left;
            canvas.clear(theme.background);
            self.tree.for_each(|id, node| {
                let style = node.style;
                match &node.kind {
                    NodeKind::Container => {}
                    NodeKind::Frame { radius } => {
                        let fill = style.fill.unwrap_or(theme.surface);
                        canvas.fill_rounded_rect(node.rect, *radius, fill);
                    }
                    NodeKind::Label { text } => {
                        let color = style.text.unwrap_or(theme.content);
                        canvas.draw_text(text, format, node.rect, color);
                    }
                    NodeKind::Button { label, radius } => {
                        let (base, hov, prs) = match style.fill {
                            Some(f) => (f, f.lighten(0.1), f.darken(0.1)),
                            None => (theme.accent, theme.accent_hover, theme.accent_pressed),
                        };
                        let fill = if pressed == Some(id) {
                            prs
                        } else if hovered == Some(id) {
                            hov
                        } else {
                            base
                        };
                        let text_color = style.text.unwrap_or(theme.on_accent);
                        canvas.fill_rounded_rect(node.rect, *radius, fill);
                        canvas.draw_text(label, format, node.rect, text_color);
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
                        let knob_x =
                            (r.x + filled_w - knob_d / 2.0).clamp(r.x, r.x + r.width - knob_d);
                        let knob = Rect::new(knob_x, cy - knob_d / 2.0, knob_d, knob_d);
                        canvas.fill_rounded_rect(knob, knob_d / 2.0, theme.content);
                    }
                    NodeKind::Checkbox { label, checked } => {
                        let r = node.rect;
                        let box_d = 22.0;
                        let bx = r.x;
                        let by = r.y + (r.height - box_d) / 2.0;
                        let box_rect = Rect::new(bx, by, box_d, box_d);
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
                    NodeKind::TextBox { state } => {
                        let r = node.rect;
                        let is_focused = focused == Some(id);
                        let border = if is_focused { theme.accent } else { theme.track };
                        canvas.fill_rounded_rect(r, 8.0, border);
                        let inner =
                            Rect::new(r.x + 2.0, r.y + 2.0, r.width - 4.0, r.height - 4.0);
                        canvas.fill_rounded_rect(inner, 6.0, theme.surface);

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
                }
            });
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
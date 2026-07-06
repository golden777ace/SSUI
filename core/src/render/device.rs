use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;

use super::canvas::Canvas;
use super::types::{Color, Rect};
use crate::tree::{Axis, NodeId, NodeKind, Props, Tree};

pub struct Renderer {
    swap_chain: IDXGISwapChain1,
    context: ID2D1DeviceContext,
    rt: ID2D1RenderTarget,
    target: Option<ID2D1Bitmap1>,
    text_format: IDWriteTextFormat,
    tree: Tree,
    width: f32,
    height: f32,
    hovered: Option<NodeId>,
    pressed: Option<NodeId>,
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
                NodeKind::Frame {
                    color: Color::rgb(0.14, 0.15, 0.18),
                    radius: 16.0,
                },
                Props {
                    axis: Axis::Vertical,
                    padding: 20.0,
                    gap: 12.0,
                    width: None,
                    height: None,
                },
            );
            tree.add_child(
                panel,
                NodeKind::Label {
                    text: "Hello, SSUI".encode_utf16().collect(),
                    color: Color::rgb(0.95, 0.96, 0.98),
                },
                Props {
                    height: Some(40.0),
                    ..Default::default()
                },
            );
            tree.add_child(
                panel,
                NodeKind::Button {
                    label: "Click me".encode_utf16().collect(),
                    base: Color::rgb(0.20, 0.45, 0.95),
                    hover: Color::rgb(0.28, 0.53, 1.0),
                    pressed: Color::rgb(0.15, 0.36, 0.80),
                    text: Color::rgb(1.0, 1.0, 1.0),
                    radius: 10.0,
                },
                Props {
                    height: Some(48.0),
                    width: Some(200.0),
                    ..Default::default()
                },
            );

            let mut renderer = Renderer {
                swap_chain,
                context,
                rt,
                target: None,
                text_format,
                tree,
                width: 1280.0,
                height: 720.0,
                hovered: None,
                pressed: None,
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

    /// Обрабатывает движение мыши. Возвращает true, если нужна перерисовка.
    pub fn on_mouse_move(&mut self, x: f32, y: f32) -> bool {
        let hit = self.tree.hit_test(x, y).filter(|&id| self.tree.is_button(id));
        if hit != self.hovered {
            self.hovered = hit;
            true
        } else {
            false
        }
    }

    /// Обрабатывает нажатие левой кнопки. Возвращает true, если нужна перерисовка.
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> bool {
        let hit = self.tree.hit_test(x, y).filter(|&id| self.tree.is_button(id));
        if hit.is_some() {
            self.pressed = hit;
            true
        } else {
            false
        }
    }

    /// Обрабатывает отпускание левой кнопки. Возвращает true, если нужна перерисовка.
    pub fn on_mouse_up(&mut self) -> bool {
        if self.pressed.is_some() {
            self.pressed = None;
            true
        } else {
            false
        }
    }

    /// Пересчитывает раскладку и перерисовывает окно из дерева элементов.
    pub fn render(&mut self) {
        self.tree.layout(Rect::new(0.0, 0.0, self.width, self.height));
        let hovered = self.hovered;
        let pressed = self.pressed;
        unsafe {
            self.rt.BeginDraw();
        }
        {
            let canvas = Canvas::new(&self.rt);
            let format = &self.text_format;
            canvas.clear(Color::rgb(0.06, 0.06, 0.07));
            self.tree.for_each(|id, node| match &node.kind {
                NodeKind::Container => {}
                NodeKind::Frame { color, radius } => {
                    canvas.fill_rounded_rect(node.rect, *radius, *color);
                }
                NodeKind::Label { text, color } => {
                    canvas.draw_text(text, format, node.rect, *color);
                }
                NodeKind::Button {
                    label,
                    base,
                    hover,
                    pressed: pressed_color,
                    text,
                    radius,
                } => {
                    let fill = if pressed == Some(id) {
                        *pressed_color
                    } else if hovered == Some(id) {
                        *hover
                    } else {
                        *base
                    };
                    canvas.fill_rounded_rect(node.rect, *radius, fill);
                    canvas.draw_text(label, format, node.rect, *text);
                }
            });
        }
        unsafe {
            let _ = self.rt.EndDraw(None, None);
            let _ = self.swap_chain.Present(1, DXGI_PRESENT(0));
        }
    }
}

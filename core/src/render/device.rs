use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;

pub struct Renderer {
    swap_chain: IDXGISwapChain1,
    context: ID2D1DeviceContext,
    rt: ID2D1RenderTarget,
    target: Option<ID2D1Bitmap1>,
    brush_panel: ID2D1SolidColorBrush,
    brush_text: ID2D1SolidColorBrush,
    text_format: IDWriteTextFormat,
    text: Vec<u16>,
}

fn color(r: f32, g: f32, b: f32, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F { r, g, b, a }
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
                28.0,
                w!("en-us"),
            )?;

            let brush_panel =
                rt.CreateSolidColorBrush(&color(0.14, 0.15, 0.18, 1.0), None)?;
            let brush_text =
                rt.CreateSolidColorBrush(&color(0.95, 0.96, 0.98, 1.0), None)?;

            let text: Vec<u16> = "Hello, SSUI".encode_utf16().collect();

            let mut renderer = Renderer {
                swap_chain,
                context,
                rt,
                target: None,
                brush_panel,
                brush_text,
                text_format,
                text,
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

    /// Перерисовывает содержимое окна.
    pub fn render(&mut self) {
        unsafe {
            self.rt.BeginDraw();
            self.rt.Clear(Some(&color(0.06, 0.06, 0.07, 1.0)));

            let rrect = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: 40.0,
                    top: 40.0,
                    right: 380.0,
                    bottom: 200.0,
                },
                radiusX: 16.0,
                radiusY: 16.0,
            };
            self.rt.FillRoundedRectangle(&rrect, &self.brush_panel);

            let layout = D2D_RECT_F {
                left: 64.0,
                top: 96.0,
                right: 360.0,
                bottom: 170.0,
            };
            self.rt.DrawText(
                &self.text,
                &self.text_format,
                &layout,
                &self.brush_text,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            let _ = self.rt.EndDraw(None, None);
            let _ = self.swap_chain.Present(1, DXGI_PRESENT(0));
        }
    }
}
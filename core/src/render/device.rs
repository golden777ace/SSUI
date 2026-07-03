use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;


pub struct Renderer {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    swap_chain: IDXGISwapChain1,
    rtv: Option<ID3D11RenderTargetView>,
}

impl Renderer {
    pub fn new(hwnd: HWND) -> Result<Self> {
        unsafe {
            let feature_levels = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0];
            let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
            let mut device: Option<ID3D11Device> = None;
            let mut context: Option<ID3D11DeviceContext> = None;
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
                Some(&mut context),
            )?;

            let device = device.expect("D3D11CreateDevice не вернул устройство");
            let context = context.expect("D3D11CreateDevice не вернул контекст");
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
            let mut renderer = Renderer {
                device,
                context,
                swap_chain,
                rtv: None,
            };
            renderer.create_rtv()?;
            Ok(renderer)
        }
    }

    fn create_rtv(&mut self) -> Result<()> {
        unsafe {
            let back_buffer: ID3D11Texture2D = self.swap_chain.GetBuffer(0)?;
            let mut rtv: Option<ID3D11RenderTargetView> = None;
            self.device
                .CreateRenderTargetView(&back_buffer, None, Some(&mut rtv))?;
            self.rtv = rtv;
            Ok(())
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        unsafe {
            self.rtv = None;
            if self
                .swap_chain
                .ResizeBuffers(0, width, height, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(0))
                .is_err()
            {
                return;
            }
            let _ = self.create_rtv();
        }
    }

    pub fn render(&mut self) {
        unsafe {
            if let Some(rtv) = &self.rtv {
                let clear_color: [f32; 4] = [0.06, 0.06, 0.07, 1.0];
                self.context.ClearRenderTargetView(rtv, &clear_color);
            }
            let _ = self.swap_chain.Present(1, DXGI_PRESENT(0));
        }
    }
}

pub mod painter;
pub mod render;
pub mod text;

use std::ffi::CString;
use std::num::NonZeroU32;

use glutin::config::{Config, ConfigTemplateBuilder};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext,
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{
    GlSurface, Surface as GlutinSurface, SurfaceAttributesBuilder, SwapInterval, WindowSurface,
};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
use skia_safe::gpu::gl::{Format, FramebufferInfo, Interface};
use skia_safe::gpu::{
    backend_render_targets, direct_contexts, surfaces, DirectContext, SurfaceOrigin,
};
use skia_safe::{ColorType, Surface};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window as WinitWindow, WindowAttributes, WindowId};

use self::painter::{SkiaFormats, SkiaPainter};
use self::render::{draw_tree, Images, Input};
use self::text::SharedText;
use crate::backend::{Painter, PlatformWindow};
use crate::render::types::Color;
use crate::theme::Theme;
use crate::tree::Tree;

/// Параметры создания окна; поля повторяют Windows-слой.
pub struct WindowOpts {
    pub glass: bool,
    pub tint: f32,
    pub blur: bool,
    pub frameless: bool,
    pub topmost: bool,
    pub center: u8,
    pub pos: Option<(i32, i32)>,
    pub resizable: bool,
    pub minbox: bool,
    pub maxbox: bool,
    pub closebox: bool,
    pub owner: Option<isize>,
    pub modal: bool,
    pub icon: Option<String>,
    pub caption: Option<u32>,
    pub caption_text: Option<u32>,
    pub border: Option<u32>,
    pub dark: Option<bool>,
    pub on_close: Option<Box<dyn FnMut()>>,
}

impl Default for WindowOpts {
    fn default() -> Self {
        Self {
            glass: false,
            tint: 0.0,
            blur: false,
            frameless: false,
            topmost: false,
            center: 0,
            pos: None,
            resizable: true,
            minbox: true,
            maxbox: true,
            closebox: true,
            owner: None,
            modal: false,
            icon: None,
            caption: None,
            caption_text: None,
            border: None,
            dark: None,
            on_close: None,
        }
    }
}

/// Живая графика окна: GL-контекст, поверхность окна и Skia.
struct Gpu {
    window: WinitWindow,
    surface: GlutinSurface<WindowSurface>,
    context: PossiblyCurrentContext,
    skia: DirectContext,
    target: Surface,
    images: Images,
    text: SharedText,
    fb: FramebufferInfo,
    samples: usize,
    stencil: usize,
}

impl Gpu {
    /// Пересоздаёт Skia-поверхность под новый размер клиентской области.
    fn resize(&mut self, w: u32, h: u32) {
        let (w, h) = (w.max(1), h.max(1));
        self.surface.resize(
            &self.context,
            NonZeroU32::new(w).unwrap(),
            NonZeroU32::new(h).unwrap(),
        );
        let rt = backend_render_targets::make_gl(
            (w as i32, h as i32),
            self.samples,
            self.stencil,
            self.fb,
        );
        if let Some(s) = surfaces::wrap_backend_render_target(
            &mut self.skia,
            &rt,
            SurfaceOrigin::BottomLeft,
            ColorType::RGBA8888,
            None,
            None,
        ) {
            self.target = s;
        }
    }

    /// Рисует кадр и меняет буферы местами.
    fn draw(&mut self, clear: Color, tree: Option<&Tree>, theme: Theme) {
        {
            let Gpu {
                target,
                images,
                text,
                ..
            } = self;
            let canvas = target.canvas();
            let mut painter = SkiaPainter::new(canvas, text.clone());
            painter.clear(clear);
            if let Some(tree) = tree {
                let formats = SkiaFormats::from_tree();
                let mut engine = text.clone();
                draw_tree(
                    &mut painter,
                    &mut engine,
                    &formats,
                    images,
                    tree,
                    theme,
                    Input::default(),
                );
            }
        }
        self.skia.flush_and_submit();
        let _ = self.surface.swap_buffers(&self.context);
    }
}

/// Окно Linux; создаётся отложенно в `show`, как требует winit 0.30.
pub struct Window {
    title: String,
    width: i32,
    height: i32,
    tree: Option<Tree>,
    opts: WindowOpts,
}

impl Window {
    /// Описание окна со стандартными параметрами.
    pub fn new(
        title: &str,
        width: i32,
        height: i32,
        tree: Tree,
        glass: bool,
        tint: f32,
        blur: bool,
    ) -> Result<Self, String> {
        Self::with_opts(
            title,
            width,
            height,
            tree,
            WindowOpts {
                glass,
                tint,
                blur,
                ..Default::default()
            },
        )
    }

    /// Описание окна по расширенным параметрам.
    pub fn with_opts(
        title: &str,
        width: i32,
        height: i32,
        tree: Tree,
        opts: WindowOpts,
    ) -> Result<Self, String> {
        Ok(Self {
            title: title.to_string(),
            width,
            height,
            tree: Some(tree),
            opts,
        })
    }

    /// Запускает цикл событий; блокирует поток до закрытия окна.
    pub fn show(mut self) -> Result<(), String> {
        let events = EventLoop::new().map_err(|e| e.to_string())?;
        events.set_control_flow(ControlFlow::Wait);
        let mut app = App {
            title: std::mem::take(&mut self.title),
            width: self.width,
            height: self.height,
            resizable: self.opts.resizable,
            frameless: self.opts.frameless,
            transparent: self.opts.glass,
            tree: self.tree.take(),
            gpu: None,
            on_close: self.opts.on_close.take(),
        };
        events.run_app(&mut app).map_err(|e| e.to_string())
    }
}

struct App {
    title: String,
    width: i32,
    height: i32,
    resizable: bool,
    frameless: bool,
    transparent: bool,
    tree: Option<Tree>,
    gpu: Option<Gpu>,
    on_close: Option<Box<dyn FnMut()>>,
}

impl App {
    /// Фон окна из текущей темы дерева.
    fn clear_color(&self) -> Color {
        let index = self.tree.as_ref().map(|t| t.theme()).unwrap_or(2);
        theme_of(index).background
    }

    /// Создаёт окно, GL-контекст и Skia-поверхность.
    fn build(&mut self, target: &ActiveEventLoop) -> Result<Gpu, String> {
        let attrs = WindowAttributes::default()
            .with_title(self.title.clone())
            .with_resizable(self.resizable)
            .with_decorations(!self.frameless)
            .with_transparent(self.transparent)
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.width.max(1) as f64,
                self.height.max(1) as f64,
            ));

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(self.transparent);
        let (window, config) = DisplayBuilder::new()
            .with_window_attributes(Some(attrs))
            .build(target, template, pick_config)
            .map_err(|e| e.to_string())?;
        let window = window.ok_or_else(|| "окно не создано".to_string())?;

        let handle = window.window_handle().ok().map(|h| h.as_raw());
        let ctx_attrs = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(None))
            .build(handle);
        let display = config.display();
        let not_current = unsafe {
            display
                .create_context(&config, &ctx_attrs)
                .map_err(|e| e.to_string())?
        };

        let builder: SurfaceAttributesBuilder<WindowSurface> = SurfaceAttributesBuilder::new();
        let surf_attrs = window.build_surface_attributes(builder).map_err(|e| e.to_string())?;
        let surface = unsafe {
            display
                .create_window_surface(&config, &surf_attrs)
                .map_err(|e| e.to_string())?
        };
        let context = not_current.make_current(&surface).map_err(|e| e.to_string())?;
        let _ = surface.set_swap_interval(
            &context,
            SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
        );

        let interface = Interface::new_load_with(|name| {
            let Ok(c) = CString::new(name) else {
                return std::ptr::null();
            };
            display.get_proc_address(c.as_c_str())
        })
        .ok_or_else(|| "GL-интерфейс Skia не создан".to_string())?;
        let mut skia = direct_contexts::make_gl(interface, None)
            .ok_or_else(|| "контекст Skia не создан".to_string())?;

        let fb = FramebufferInfo {
            fboid: 0,
            format: Format::RGBA8.into(),
            ..Default::default()
        };
        let samples = config.num_samples() as usize;
        let stencil = config.stencil_size() as usize;
        let size = window.inner_size();
        let rt = backend_render_targets::make_gl(
            (size.width.max(1) as i32, size.height.max(1) as i32),
            samples,
            stencil,
            fb,
        );
        let target = surfaces::wrap_backend_render_target(
            &mut skia,
            &rt,
            SurfaceOrigin::BottomLeft,
            ColorType::RGBA8888,
            None,
            None,
        )
        .ok_or_else(|| "поверхность Skia не создана".to_string())?;

        Ok(Gpu {
            window,
            surface,
            context,
            skia,
            target,
            images: Images::new(),
            text: SharedText::new(),
            fb,
            samples,
            stencil,
        })
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, target: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        match self.build(target) {
            Ok(gpu) => {
                gpu.window.request_redraw();
                self.gpu = Some(gpu);
            }
            Err(e) => {
                eprintln!("SSUI: окно не создано: {e}");
                target.exit();
            }
        }
    }

    fn window_event(&mut self, target: &ActiveEventLoop, _id: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => {
                if let Some(cb) = self.on_close.as_mut() {
                    cb();
                }
                target.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.resize(size.width, size.height);
                    gpu.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let clear = self.clear_color();
                let theme = theme_of(self.tree.as_ref().map(|t| t.theme()).unwrap_or(2));
                let tree = self.tree.take();
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.draw(clear, tree.as_ref(), theme);
                }
                self.tree = tree;
            }
            _ => {}
        }
    }
}

impl PlatformWindow for Window {
    fn run(&self) {}

    fn request_redraw(&self) {}

    fn set_title(&self, _title: &str) {}

    fn client_size(&self) -> (f32, f32) {
        (self.width as f32, self.height as f32)
    }

    fn scale(&self) -> f32 {
        1.0
    }

    fn raise(&self) {}

    fn close(&self) {}
}

/// Выбирает GL-конфигурацию с наибольшим числом сэмплов.
fn pick_config(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
    configs
        .reduce(|best, c| {
            if c.num_samples() > best.num_samples() {
                c
            } else {
                best
            }
        })
        .expect("нет подходящей GL-конфигурации")
}

/// Тема по индексу; повторяет выбор Windows-рендерера.
fn theme_of(index: usize) -> Theme {
    match index {
        0 => Theme::white(),
        1 => Theme::light(),
        2 => Theme::dark(),
        _ => Theme::black(),
    }
}
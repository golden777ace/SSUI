pub mod dpi;
pub mod input;
pub mod painter;
pub mod render;
pub mod system;
pub mod text;

use std::ffi::CString;
use std::num::NonZeroU32;

use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
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
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, NamedKey};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{CursorIcon, Window as WinitWindow, WindowAttributes, WindowId};

use self::painter::{SkiaFormats, SkiaPainter};
use self::render::Input;
use self::input::InputState;
use self::render::{draw_tree, Images};
use self::text::SharedText;
use crate::backend::{Painter, PlatformWindow};
use crate::render::types::{Color, Rect};
use crate::render::CursorKind;
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

    /// Рисует кадр и меняет буферы местами; дерево уже разложено.
    fn draw(&mut self, clear: Color, tree: Option<&Tree>, theme: Theme, input: Input) {
        {
            let Gpu {
                target,
                images,
                text,
                ..
            } = self;
            if let Some(tree) = tree {
                images.preload(tree);
            }
            let canvas = target.canvas();
            let mut painter = SkiaPainter::new(canvas, text.clone());
            painter.clear(clear);
            if let Some(tree) = tree {
                let formats = SkiaFormats::from_tree();
                let mut engine = text.clone();
                draw_tree(&mut painter, &mut engine, &formats, images, tree, theme, input);
            }
        }
        self.skia.flush_and_submit();
        let _ = self.surface.swap_buffers(&self.context);
    }
}

thread_local! {
    /// Окна, созданные до запуска цикла событий.
    static PENDING: std::cell::RefCell<Vec<Pending>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Счётчик описателей окон.
    static NEXT_ID: std::cell::Cell<isize> = const { std::cell::Cell::new(1) };
}

/// Описание окна, ждущее запуска цикла событий.
struct Pending {
    id: isize,
    title: String,
    width: i32,
    height: i32,
    tree: Tree,
    opts: WindowOpts,
}

/// Окно Linux; создаётся отложенно в цикле событий, как требует winit.
pub struct Window {
    id: isize,
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
        let id = NEXT_ID.with(|n| {
            let v = n.get();
            n.set(v + 1);
            v
        });
        PENDING.with(|p| {
            p.borrow_mut().push(Pending {
                id,
                title: title.to_string(),
                width,
                height,
                tree,
                opts,
            })
        });
        Ok(Self { id })
    }

    /// Описатель окна; на Linux — внутренний номер, не HWND.
    pub fn handle(&self) -> isize {
        self.id
    }

    /// Помечает окно главным: его закрытие завершает цикл событий.
    pub fn mark_main(&self) {
        MAIN.with(|m| m.set(self.id));
    }

    /// Поднимает окно поверх остальных и передаёт ему фокус.
    pub fn raise(&self) {
        RAISE.with(|r| r.borrow_mut().push(self.id));
    }

    /// Просит закрыть окно.
    pub fn close(&self) {
        CLOSE.with(|c| c.borrow_mut().push(self.id));
    }

    /// Меняет заголовок окна.
    pub fn set_title(&self, title: &str) {
        TITLES.with(|t| t.borrow_mut().push((self.id, title.to_string())));
    }

    /// Будит цикл событий окна по описателю; безопасно из любого потока.
    pub fn wake(handle: isize) {
        if handle != 0 {
            WAKE.with(|w| w.borrow_mut().push(handle));
        }
    }

    /// Просит окно обработать очередь файловых диалогов.
    pub fn post_files(handle: isize) {
        if handle != 0 {
            FILES.with(|f| f.borrow_mut().push(handle));
        }
    }

    /// Размер основного экрана в пикселях.
    pub fn screen() -> (f32, f32) {
        SCREEN.with(|s| s.get())
    }

    /// Размер клиентской области окна по описателю.
    pub fn client_size(handle: isize) -> (f32, f32) {
        SIZES.with(|s| s.borrow().get(&handle).copied().unwrap_or((0.0, 0.0)))
    }

    /// Перемещает окно в точку экрана.
    pub fn move_to(handle: isize, x: f32, y: f32) {
        if handle != 0 {
            MOVES.with(|m| m.borrow_mut().push((handle, x, y)));
        }
    }

    /// Запускает цикл событий; блокирует поток до закрытия главного окна.
    pub fn loop_messages() {
        if let Err(e) = run_loop() {
            eprintln!("SSUI: цикл событий завершился с ошибкой: {e}");
        }
    }

    /// Запускает цикл событий для этого окна.
    pub fn show(self) -> Result<(), String> {
        run_loop()
    }
}

thread_local! {
    /// Главное окно: его закрытие завершает цикл.
    static MAIN: std::cell::Cell<isize> = const { std::cell::Cell::new(0) };
    /// Окна, которые просят поднять себя поверх остальных.
    static RAISE: std::cell::RefCell<Vec<isize>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Окна, которые просят закрыться.
    static CLOSE: std::cell::RefCell<Vec<isize>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Окна, которым нужно разбудить цикл событий.
    static WAKE: std::cell::RefCell<Vec<isize>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Окна, которым нужно показать файловый диалог.
    static FILES: std::cell::RefCell<Vec<isize>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Окна, которые нужно переместить.
    static MOVES: std::cell::RefCell<Vec<(isize, f32, f32)>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Размеры клиентских областей живых окон.
    static SIZES: std::cell::RefCell<std::collections::HashMap<isize, (f32, f32)>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    /// Размер основного экрана; уточняется при создании окна.
    static SCREEN: std::cell::Cell<(f32, f32)> =
        const { std::cell::Cell::new((1920.0, 1080.0)) };
    /// Новые заголовки окон.
    static TITLES: std::cell::RefCell<Vec<(isize, String)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Курсор winit по виду курсора SSUI.
fn cursor_icon(kind: CursorKind) -> CursorIcon {
    match kind {
        CursorKind::Hand => CursorIcon::Pointer,
        CursorKind::IBeam => CursorIcon::Text,
        CursorKind::Arrow => CursorIcon::Default,
    }
}

/// Забирает накопленные описания и крутит цикл событий.
///
/// Ограничение текущего шага: окна создаются пачкой на старте цикла,
/// новые окна во время работы цикла ещё не поддержаны.
fn run_loop() -> Result<(), String> {
    let mut queue = PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()));
    if queue.is_empty() {
        return Ok(());
    }
    let first = queue.remove(0);
    let events = EventLoop::new().map_err(|e| e.to_string())?;
    events.set_control_flow(ControlFlow::Wait);
    let mut opts = first.opts;
    let mut app = App {
        title: first.title,
        width: first.width,
        height: first.height,
        resizable: opts.resizable,
        frameless: opts.frameless,
        transparent: opts.glass,
        tree: Some(first.tree),
        gpu: None,
        input: InputState::default(),
        shift: false,
        id: first.id,
        last_tick: std::time::Instant::now(),
        on_close: opts.on_close.take(),
    };
    events.run_app(&mut app).map_err(|e| e.to_string())
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
    input: InputState,
    shift: bool,
    id: isize,
    last_tick: std::time::Instant,
    on_close: Option<Box<dyn FnMut()>>,
}

impl App {
    /// Продвигает кадр: очереди, анимации, таймеры, спиннеры.
    /// Возвращает `true`, если нужна перерисовка.
    fn pump(&mut self) -> bool {
        let now = std::time::Instant::now();
        let dt = (now - self.last_tick).as_secs_f32();
        self.last_tick = now;
        let tree = match self.tree.as_mut() {
            Some(t) => t,
            None => return false,
        };
        let posted = tree.fire_frame();
        let built = tree.apply_build_queue()
            | tree.apply_css_queue()
            | tree.apply_text_queue();
        let moved = tree.apply_canvas_queue() | tree.apply_tree_queue();
        tree.sync_tree_geom();
        let anim = tree.tick(dt);
        let fired = tree.has_timers() && tree.tick_timers();
        let spin = tree.has_spinner();
        if spin {
            tree.spin(dt);
        }
        posted || built | moved || anim || fired || spin
    }

    /// Нужен ли непрерывный цикл кадров.
    fn busy(&self) -> bool {
        self.tree
            .as_ref()
            .map(|t| t.has_timers() || t.has_spinner())
            .unwrap_or(false)
    }

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
                if let Some(mon) = gpu.window.current_monitor() {
                    let s = mon.size();
                    SCREEN.with(|v| v.set((s.width as f32, s.height as f32)));
                }
                let size = gpu.window.inner_size();
                let id = self.id;
                SIZES.with(|m| {
                    m.borrow_mut()
                        .insert(id, (size.width as f32, size.height as f32))
                });
                gpu.window.request_redraw();
                self.gpu = Some(gpu);
            }
            Err(e) => {
                eprintln!("SSUI: окно не создано: {e}");
                target.exit();
            }
        }
    }

    fn about_to_wait(&mut self, target: &ActiveEventLoop) {
        if self.pump() {
            if let Some(gpu) = self.gpu.as_ref() {
                gpu.window.request_redraw();
            }
        }
        target.set_control_flow(if self.busy() {
            ControlFlow::Poll
        } else {
            ControlFlow::Wait
        });
        let id = self.id;
        let woken = WAKE.with(|w| {
            let mut q = w.borrow_mut();
            let hit = q.iter().any(|&h| h == id);
            q.retain(|&h| h != id);
            hit
        });
        let moved = MOVES.with(|m| {
            let mut q = m.borrow_mut();
            let found = q.iter().find(|(h, _, _)| *h == id).map(|(_, x, y)| (*x, *y));
            q.retain(|(h, _, _)| *h != id);
            found
        });
        let closing = CLOSE.with(|c| {
            let mut q = c.borrow_mut();
            let hit = q.iter().any(|&h| h == id);
            q.retain(|&h| h != id);
            hit
        });
        let raising = RAISE.with(|r| {
            let mut q = r.borrow_mut();
            let hit = q.iter().any(|&h| h == id);
            q.retain(|&h| h != id);
            hit
        });
        let title = TITLES.with(|t| {
            let mut q = t.borrow_mut();
            let found = q
                .iter()
                .rev()
                .find(|(h, _)| *h == id)
                .map(|(_, s)| s.clone());
            q.retain(|(h, _)| *h != id);
            found
        });
        if let Some(gpu) = self.gpu.as_ref() {
            if let Some(t) = title {
                gpu.window.set_title(&t);
            }
            if let Some((x, y)) = moved {
                gpu.window
                    .set_outer_position(winit::dpi::PhysicalPosition::new(x as f64, y as f64));
            }
            if raising {
                gpu.window.focus_window();
            }
            if woken || moved.is_some() || raising {
                gpu.window.request_redraw();
            }
        }
        if closing {
            if let Some(cb) = self.on_close.as_mut() {
                cb();
            }
            target.exit();
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
                let id = self.id;
                SIZES.with(|m| {
                    m.borrow_mut()
                        .insert(id, (size.width as f32, size.height as f32))
                });
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.resize(size.width, size.height);
                    gpu.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let clear = self.clear_color();
                let theme = theme_of(self.tree.as_ref().map(|t| t.theme()).unwrap_or(2));
                let size = self
                    .gpu
                    .as_ref()
                    .map(|g| g.window.inner_size())
                    .unwrap_or_default();
                let mut tree = self.tree.take();
                if let Some(t) = tree.as_mut() {
                    t.layout(Rect::new(
                        0.0,
                        0.0,
                        size.width.max(1) as f32,
                        size.height.max(1) as f32,
                    ));
                }
                let input = self.input.snapshot();
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.draw(clear, tree.as_ref(), theme, input);
                }
                self.tree = tree;
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x as f32, position.y as f32);
                let mut tree = self.tree.take();
                let dirty = match tree.as_mut() {
                    Some(t) => self.input.mouse_move(t, x, y),
                    None => false,
                };
                let kind = tree
                    .as_ref()
                    .map(|t| self.input.cursor(t))
                    .unwrap_or(CursorKind::Arrow);
                self.tree = tree;
                if let Some(gpu) = self.gpu.as_ref() {
                    gpu.window.set_cursor(cursor_icon(kind));
                    if dirty {
                        gpu.window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let d = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                let mut tree = self.tree.take();
                let dirty = match tree.as_mut() {
                    Some(t) => self.input.wheel(t, d),
                    None => false,
                };
                self.tree = tree;
                if dirty {
                    if let Some(gpu) = self.gpu.as_ref() {
                        gpu.window.request_redraw();
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let shift = self.shift;
                let mut tree = self.tree.take();
                let dirty = match (tree.as_mut(), event.logical_key) {
                    (Some(t), Key::Named(NamedKey::Tab)) => self.input.focus_step(t, shift),
                    (Some(t), Key::Named(NamedKey::Enter))
                    | (Some(t), Key::Named(NamedKey::Space)) => self.input.activate_focused(t),
                    _ => false,
                };
                self.tree = tree;
                if dirty {
                    if let Some(gpu) = self.gpu.as_ref() {
                        gpu.window.request_redraw();
                    }
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.shift = mods.state().shift_key();
            }
            WindowEvent::CursorLeft { .. } => {
                if self.input.mouse_leave() {
                    if let Some(gpu) = self.gpu.as_ref() {
                        gpu.window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button != MouseButton::Left {
                    return;
                }
                let (x, y) = self.input.mouse;
                let mut tree = self.tree.take();
                let dirty = match tree.as_mut() {
                    Some(t) => match state {
                        ElementState::Pressed => self.input.mouse_down(t, x, y),
                        ElementState::Released => self.input.mouse_up(t),
                    },
                    None => false,
                };
                self.tree = tree;
                if dirty {
                    if let Some(gpu) = self.gpu.as_ref() {
                        gpu.window.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }
}

impl PlatformWindow for Window {
    fn run(&self) {
        Window::loop_messages();
    }

    fn request_redraw(&self) {
        RAISE.with(|r| r.borrow_mut().push(self.id));
    }

    fn set_title(&self, title: &str) {
        Window::set_title(self, title);
    }

    fn client_size(&self) -> (f32, f32) {
        PENDING.with(|p| {
            p.borrow()
                .iter()
                .find(|w| w.id == self.id)
                .map(|w| (w.width as f32, w.height as f32))
                .unwrap_or((0.0, 0.0))
        })
    }

    fn scale(&self) -> f32 {
        1.0
    }

    fn raise(&self) {
        Window::raise(self);
    }

    fn close(&self) {
        Window::close(self);
    }
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
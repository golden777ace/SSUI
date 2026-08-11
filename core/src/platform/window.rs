use core::ffi::c_void;
use std::cell::Cell;
use std::sync::Once;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, FillRect, GetMonitorInfoW, InvalidateRect, MonitorFromWindow,
    UpdateWindow, ValidateRect, HDC, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::Ime::{
    ImmGetCompositionStringW, ImmGetContext, ImmReleaseContext, ImmSetCompositionWindow, CFS_POINT,
    COMPOSITIONFORM, GCS_RESULTSTR,
};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
    DWMWA_USE_IMMERSIVE_DARK_MODE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, ReleaseCapture, SetCapture};
use windows::Win32::UI::Shell::{
    DragAcceptFiles, DragFinish, DragQueryFileW, DragQueryPoint, FileOpenDialog, FileSaveDialog,
    IFileOpenDialog, IFileSaveDialog, IShellItem, FOS_PICKFOLDERS, HDROP, SIGDN_FILESYSPATH,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::backend::PlatformWindow;
use crate::render::{CursorKind, Renderer};
use crate::tree::Tree;

struct WindowState {
    renderer: Renderer,
    blur_on: bool,
    moving: bool,
    applied_mode: u32,
    applied_tint: u32,
    ticking: bool,
    idle: u32,
    owner: HWND,
    modal: bool,
    on_close: Option<Box<dyn FnMut()>>,
}

pub struct Window {
    hwnd: HWND,
}

/// Параметры создания окна.
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

thread_local! {
    static MAIN: Cell<isize> = const { Cell::new(0) };
}

static REGISTER_CLASS: Once = Once::new();

const WM_APP_FILES: u32 = WM_APP + 3;

impl Window {
    /// Создаёт окно со стандартными параметрами.
    pub fn new(
        title: &str,
        width: i32,
        height: i32,
        tree: Tree,
        glass: bool,
        tint: f32,
        blur: bool,
    ) -> Result<Self> {
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

    /// Создаёт окно по расширенным параметрам `opts`.
    pub fn with_opts(
        title: &str,
        width: i32,
        height: i32,
        tree: Tree,
        opts: WindowOpts,
    ) -> Result<Self> {
        let WindowOpts {
            glass,
            tint,
            blur,
            frameless,
            topmost,
            center,
            pos,
            resizable,
            minbox,
            maxbox,
            closebox,
            owner,
            modal,
            icon,
            caption,
            caption_text,
            border,
            dark,
            on_close,
        } = opts;
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let hinstance: HINSTANCE = instance.into();
            let class_name = w!("SSUI.Window");

            REGISTER_CLASS.call_once(|| {
                let wc = WNDCLASSW {
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: Some(wndproc),
                    hInstance: hinstance,
                    hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                    lpszClassName: class_name,
                    ..Default::default()
                };
                let atom = RegisterClassW(&wc);
                debug_assert!(atom != 0, "RegisterClassW завершился ошибкой");
            });

            let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();

            let mut ex_style = if glass {
                WS_EX_NOREDIRECTIONBITMAP
            } else {
                WINDOW_EX_STYLE::default()
            };
            if topmost {
                ex_style |= WS_EX_TOPMOST;
            }
            if frameless {
                ex_style |= WS_EX_TOOLWINDOW;
            }

            let style = if frameless {
                WS_POPUP
            } else {
                let mut s = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
                if resizable {
                    s |= WS_THICKFRAME;
                }
                if minbox {
                    s |= WS_MINIMIZEBOX;
                }
                if maxbox {
                    s |= WS_MAXIMIZEBOX;
                }
                if !closebox && !minbox && !maxbox {
                    s &= !WS_SYSMENU;
                }
                s
            };

            let owner_hwnd = match owner {
                Some(h) if h != 0 => HWND(h as *mut c_void),
                _ => HWND::default(),
            };

            let (px, py) = if let Some((x, y)) = pos {
                (x, y)
            } else if center > 0 {
                let has_owner = !owner_hwnd.0.is_null();
                let mut area = RECT::default();
                if center == 1 && has_owner {
                    let _ = GetWindowRect(owner_hwnd, &mut area);
                } else {
                    let flag = if has_owner {
                        MONITOR_DEFAULTTONEAREST
                    } else {
                        MONITOR_DEFAULTTOPRIMARY
                    };
                    let mon = MonitorFromWindow(owner_hwnd, flag);
                    let mut mi = MONITORINFO {
                        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                        ..Default::default()
                    };
                    if GetMonitorInfoW(mon, &mut mi).as_bool() {
                        area = mi.rcWork;
                    } else {
                        area.right = GetSystemMetrics(SM_CXSCREEN);
                        area.bottom = GetSystemMetrics(SM_CYSCREEN);
                    }
                }
                (
                    area.left + (area.right - area.left - width) / 2,
                    area.top + (area.bottom - area.top - height) / 2,
                )
            } else {
                (CW_USEDEFAULT, CW_USEDEFAULT)
            };

            let hwnd = CreateWindowExW(
                ex_style,
                class_name,
                PCWSTR(title_w.as_ptr()),
                style,
                px,
                py,
                width,
                height,
                if modal && !owner_hwnd.0.is_null() {
                    Some(owner_hwnd)
                } else {
                    None
                },
                None,
                Some(hinstance),
                None,
            )?;

            if modal && !owner_hwnd.0.is_null() {
                let _ = EnableWindow(owner_hwnd, false);
            }

            if let Some(d) = dark {
                let v: i32 = if d { 1 } else { 0 };
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE,
                    &v as *const i32 as *const c_void,
                    4,
                );
            }
            for (attr, val) in [
                (DWMWA_CAPTION_COLOR, caption),
                (DWMWA_TEXT_COLOR, caption_text),
                (DWMWA_BORDER_COLOR, border),
            ] {
                if let Some(c) = val {
                    let _ =
                        DwmSetWindowAttribute(hwnd, attr, &c as *const u32 as *const c_void, 4);
                }
            }
            if let Some(path) = &icon {
                let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
                for (msg, cx, cy) in [
                    (ICON_BIG, SM_CXICON, SM_CYICON),
                    (ICON_SMALL, SM_CXSMICON, SM_CYSMICON),
                ] {
                    if let Ok(h) = LoadImageW(
                        None,
                        PCWSTR(wide.as_ptr()),
                        IMAGE_ICON,
                        GetSystemMetrics(cx),
                        GetSystemMetrics(cy),
                        LR_LOADFROMFILE,
                    ) {
                        SendMessageW(
                            hwnd,
                            WM_SETICON,
                            Some(WPARAM(msg as usize)),
                            Some(LPARAM(h.0 as isize)),
                        );
                    }
                }
            }

            let _ = ShowWindow(hwnd, SW_SHOW);
            let init_mode: u32 = if blur { 3 } else { 0 };
            let init_tint: u32 = 0x40101418;
            let mut renderer = Renderer::new(hwnd, tree, glass, tint, width, height)?;
            let mut rc = RECT::default();
            let _ = GetClientRect(hwnd, &mut rc);
            renderer.resize((rc.right - rc.left) as u32, (rc.bottom - rc.top) as u32);
            renderer.set_blur(init_mode, init_tint);
            let state = Box::new(WindowState {
                renderer,
                blur_on: blur,
                moving: false,
                applied_mode: init_mode,
                applied_tint: init_tint,
                ticking: true,
                idle: 0,
                owner: owner_hwnd,
                modal,
                on_close,
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

            let _ = SetTimer(Some(hwnd), 1, 10, None);
            DragAcceptFiles(hwnd, true);
            let _ = InvalidateRect(Some(hwnd), None, false);
            let _ = UpdateWindow(hwnd);
            if blur {
                set_accent(hwnd, init_mode, init_tint);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }

            Ok(Window { hwnd })
        }
    }

    /// HWND окна.
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// HWND окна как целое; для передачи в Python-слой.
    pub fn handle(&self) -> isize {
        self.hwnd.0 as isize
    }

    /// Помечает окно главным: его закрытие завершает приложение.
    pub fn mark_main(&self) {
        MAIN.with(|c| c.set(self.hwnd.0 as isize));
    }

    /// Поднимает окно поверх остальных и передаёт ему фокус.
    pub fn raise(&self) {
        unsafe {
            let _ = BringWindowToTop(self.hwnd);
            let _ = SetForegroundWindow(self.hwnd);
        }
    }

    /// Закрывает окно программно; разрушение отложено до конца обработки.
    pub fn close(&self) {
        unsafe {
            let _ = PostMessageW(Some(self.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }

    /// Меняет заголовок окна.
    pub fn set_title(&self, title: &str) {
        let w = wide0(title);
        unsafe {
            let _ = SetWindowTextW(self.hwnd, PCWSTR(w.as_ptr()));
        }
    }

    /// Помечает окно грязным и просит перерисовку.
    pub fn request_redraw(&self) {
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    /// Масштаб окна: 1.0 при 96 DPI.
    pub fn scale(&self) -> f32 {
        let dpi = unsafe { GetDpiForWindow(self.hwnd) };
        if dpi == 0 {
            1.0
        } else {
            dpi as f32 / 96.0
        }
    }

    /// Размер клиентской области этого окна в пикселях.
    pub fn size(&self) -> (f32, f32) {
        Window::client_size(self.hwnd.0 as isize)
    }

    /// Будит цикл сообщений окна по HWND; безопасно из любого потока.
    pub fn wake(handle: isize) {
        if handle == 0 {
            return;
        }
        unsafe {
            let h = HWND(handle as *mut c_void);
            let _ = PostMessageW(Some(h), WM_NULL, WPARAM(0), LPARAM(0));
        }
    }

    /// Просит окно показать отложенные файловые диалоги.
    pub fn post_files(handle: isize) {
        if handle == 0 {
            return;
        }
        unsafe {
            let h = HWND(handle as *mut c_void);
            let _ = PostMessageW(Some(h), WM_APP_FILES, WPARAM(0), LPARAM(0));
        }
    }

    /// Размер основного экрана в пикселях.
    pub fn screen() -> (f32, f32) {
        unsafe {
            let w = GetSystemMetrics(SM_CXSCREEN);
            let h = GetSystemMetrics(SM_CYSCREEN);
            (w as f32, h as f32)
        }
    }

    /// Размер клиентской области окна по HWND.
    pub fn client_size(handle: isize) -> (f32, f32) {
        if handle == 0 {
            return (0.0, 0.0);
        }
        unsafe {
            let h = HWND(handle as *mut c_void);
            let mut rc = RECT::default();
            if GetClientRect(h, &mut rc).is_ok() {
                ((rc.right - rc.left) as f32, (rc.bottom - rc.top) as f32)
            } else {
                (0.0, 0.0)
            }
        }
    }

    /// Перемещает окно; `(x, y)` — левый верхний угол на экране.
    pub fn move_to(handle: isize, x: f32, y: f32) {
        if handle == 0 {
            return;
        }
        unsafe {
            let h = HWND(handle as *mut c_void);
            let _ = SetWindowPos(
                h,
                None,
                x as i32,
                y as i32,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    /// Прокачивает накопленные сообщения без блокировки.
    pub fn pump() -> bool {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    return false;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            true
        }
    }

    /// Запускает блокирующий цикл сообщений до закрытия окна.
    pub fn run(&self) {
        Window::loop_messages();
    }

    /// Крутит цикл сообщений потока до `WM_QUIT`; не требует окна.
    pub fn loop_messages() {
        unsafe {
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

impl PlatformWindow for Window {
    fn run(&self) {
        Window::run(self);
    }

    fn request_redraw(&self) {
        Window::request_redraw(self);
    }

    fn set_title(&self, title: &str) {
        Window::set_title(self, title);
    }

    fn client_size(&self) -> (f32, f32) {
        Window::size(self)
    }

    fn scale(&self) -> f32 {
        Window::scale(self)
    }

    fn raise(&self) {
        Window::raise(self);
    }

    fn close(&self) {
        Window::close(self);
    }
}

unsafe fn state_ptr(hwnd: HWND) -> *mut WindowState {
    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState
}

unsafe fn ensure_timer(hwnd: HWND, state: &mut WindowState) {
    state.idle = 0;
    if !state.ticking {
        let _ = SetTimer(Some(hwnd), 1, 10, None);
        state.ticking = true;
    }
}

fn wide0(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn read_item(item: &IShellItem) -> windows::core::Result<String> {
    let pw = item.GetDisplayName(SIGDN_FILESYSPATH)?;
    let text = pw.to_string().unwrap_or_default();
    CoTaskMemFree(Some(pw.0 as *const c_void));
    Ok(text)
}

/// Показывает нативный файловый диалог; `None` при отмене или ошибке.
fn show_file_dialog(
    owner: HWND,
    mode: u8,
    title: &str,
    name: &str,
    patterns: &[(String, String)],
) -> Option<String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let title_w = wide0(title);
        let name_w = wide0(name);
        let pat_w: Vec<(Vec<u16>, Vec<u16>)> =
            patterns.iter().map(|(d, m)| (wide0(d), wide0(m))).collect();
        let specs: Vec<COMDLG_FILTERSPEC> = pat_w
            .iter()
            .map(|(d, m)| COMDLG_FILTERSPEC {
                pszName: PCWSTR(d.as_ptr()),
                pszSpec: PCWSTR(m.as_ptr()),
            })
            .collect();
        let run = || -> windows::core::Result<String> {
            if mode == 1 {
                let dlg: IFileSaveDialog =
                    CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)?;
                if !title.is_empty() {
                    dlg.SetTitle(PCWSTR(title_w.as_ptr()))?;
                }
                if !specs.is_empty() {
                    dlg.SetFileTypes(&specs)?;
                }
                if !name.is_empty() {
                    dlg.SetFileName(PCWSTR(name_w.as_ptr()))?;
                }
                dlg.Show(Some(owner))?;
                read_item(&dlg.GetResult()?)
            } else {
                let dlg: IFileOpenDialog =
                    CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
                if !title.is_empty() {
                    dlg.SetTitle(PCWSTR(title_w.as_ptr()))?;
                }
                if mode == 2 {
                    let opts = dlg.GetOptions()?;
                    dlg.SetOptions(opts | FOS_PICKFOLDERS)?;
                } else if !specs.is_empty() {
                    dlg.SetFileTypes(&specs)?;
                }
                dlg.Show(Some(owner))?;
                read_item(&dlg.GetResult()?)
            }
        };
        run().ok()
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    state.renderer.render();
                    ensure_timer(hwnd, state);
                }
                let _ = ValidateRect(Some(hwnd), None);
                LRESULT(0)
            }

            WM_TIMER => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    if state.renderer.on_timer() {
                        let _ = InvalidateRect(Some(hwnd), None, false);
                        state.idle = 0;
                    } else {
                        state.idle = state.idle.saturating_add(1);
                        if state.idle > 8 && !state.renderer.busy() {
                            let _ = KillTimer(Some(hwnd), 1);
                            state.ticking = false;
                        }
                    }
                    if state.blur_on && !state.moving {
                        let mode = state.renderer.blur_mode();
                        let tint = state.renderer.blur_tint();
                        if mode != state.applied_mode || tint != state.applied_tint {
                            set_accent(hwnd, mode, tint);
                            state.applied_mode = mode;
                            state.applied_tint = tint;
                        }
                    }
                }
                LRESULT(0)
            }

            WM_ENTERSIZEMOVE => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    if state.blur_on && state.renderer.drag_smooth() {
                        set_accent(hwnd, 0, state.applied_tint);
                        state.moving = true;
                    }
                }
                LRESULT(0)
            }

            WM_EXITSIZEMOVE => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    if state.blur_on {
                        let mode = state.renderer.blur_mode();
                        let tint = state.renderer.blur_tint();
                        set_accent(hwnd, mode, tint);
                        state.applied_mode = mode;
                        state.applied_tint = tint;
                        state.moving = false;
                    }
                }
                LRESULT(0)
            }

            WM_SIZE => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    let width = loword(lparam.0 as u32);
                    let height = hiword(lparam.0 as u32);
                    state.renderer.resize(width, height);
                    state.renderer.fire_resize(width as f32, height as f32);
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                LRESULT(0)
            }

            WM_MOUSEMOVE => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    let (x, y) = mouse_xy(lparam);
                    if state.renderer.on_mouse_move(x, y) {
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                }
                LRESULT(0)
            }

            WM_MOUSEWHEEL => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    let delta = ((wparam.0 >> 16) & 0xFFFF) as i16 as i32;
                    if state.renderer.on_wheel(delta) {
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                }
                LRESULT(0)
            }

            WM_RBUTTONDOWN => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    let (x, y) = mouse_xy(lparam);
                    if state.renderer.on_right_down(x, y) {
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                }
                LRESULT(0)
            }

            WM_LBUTTONDOWN => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    let (x, y) = mouse_xy(lparam);
                    let _ = SetCapture(hwnd);
                    if state.renderer.on_mouse_down(x, y) {
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                }
                LRESULT(0)
            }

            WM_LBUTTONUP => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    let _ = ReleaseCapture();
                    if state.renderer.on_mouse_up() {
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                }
                LRESULT(0)
            }

            WM_KEYDOWN => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    if state.renderer.on_key(wparam.0 as u32) {
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                }
                LRESULT(0)
            }

            WM_APP_FILES => {
                let reqs = match state_ptr(hwnd).as_mut() {
                    Some(state) => state.renderer.take_files(),
                    None => Vec::new(),
                };
                for req in reqs {
                    let path =
                        show_file_dialog(hwnd, req.mode, &req.title, &req.name, &req.patterns)
                            .unwrap_or_default();
                    if let Some(state) = state_ptr(hwnd).as_mut() {
                        state.renderer.deliver_file(req, path);
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                }
                LRESULT(0)
            }

            WM_CHAR => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    if state.renderer.on_char(wparam.0 as u16) {
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                }
                LRESULT(0)
            }

            WM_IME_STARTCOMPOSITION => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    if let Some((cx, cy)) = state.renderer.ime_caret() {
                        let himc = ImmGetContext(hwnd);
                        if !himc.0.is_null() {
                            let form = COMPOSITIONFORM {
                                dwStyle: CFS_POINT,
                                ptCurrentPos: POINT {
                                    x: cx as i32,
                                    y: cy as i32,
                                },
                                rcArea: RECT::default(),
                            };
                            let _ = ImmSetCompositionWindow(himc, &form);
                            let _ = ImmReleaseContext(hwnd, himc);
                        }
                    }
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            WM_IME_COMPOSITION => {
                if (lparam.0 as u32) & GCS_RESULTSTR.0 != 0 {
                    if let Some(state) = state_ptr(hwnd).as_mut() {
                        let text = ime_result_string(hwnd);
                        if !text.is_empty() && state.renderer.on_ime_text(&text) {
                            let _ = InvalidateRect(Some(hwnd), None, false);
                        }
                    }
                    LRESULT(0)
                } else {
                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
            }

            WM_MOUSEACTIVATE => {
                let _ = BringWindowToTop(hwnd);
                LRESULT(MA_ACTIVATE as isize)
            }

            WM_SETCURSOR => {
                let ht = (lparam.0 & 0xFFFF) as i32;
                if ht == HTCLIENT as i32 {
                    if let Some(state) = state_ptr(hwnd).as_mut() {
                        let id = match state.renderer.cursor_kind() {
                            CursorKind::Hand => IDC_HAND,
                            CursorKind::IBeam => IDC_IBEAM,
                            CursorKind::Arrow => IDC_ARROW,
                        };
                        if let Ok(cur) = LoadCursorW(None, id) {
                            let _ = SetCursor(Some(cur));
                        }
                    }
                    return LRESULT(1);
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            WM_DROPFILES => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    let hdrop = HDROP(wparam.0 as *mut c_void);
                    let count = DragQueryFileW(hdrop, 0xFFFF_FFFF, None);
                    let mut paths: Vec<String> = Vec::new();
                    for i in 0..count {
                        let len = DragQueryFileW(hdrop, i, None) as usize;
                        let mut buf = vec![0u16; len + 1];
                        DragQueryFileW(hdrop, i, Some(&mut buf));
                        buf.truncate(len);
                        paths.push(String::from_utf16_lossy(&buf));
                    }
                    let mut pt = POINT::default();
                    let _ = DragQueryPoint(hdrop, &mut pt);
                    DragFinish(hdrop);
                    if state
                        .renderer
                        .on_drop(pt.x as f32, pt.y as f32, &paths.join("\n"))
                    {
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                }
                LRESULT(0)
            }

            WM_ERASEBKGND => {
                if state_ptr(hwnd).is_null() {
                    let hdc = HDC(wparam.0 as *mut c_void);
                    let mut rc = RECT::default();
                    let _ = GetClientRect(hwnd, &mut rc);
                    let brush = CreateSolidBrush(COLORREF(0x00181410));
                    let _ = FillRect(hdc, &rc, brush);
                    let _ = DeleteObject(brush.into());
                }
                LRESULT(1)
            }

            WM_DPICHANGED => {
                let suggested = &*(lparam.0 as *const RECT);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                LRESULT(0)
            }

            WM_DESTROY => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    if state.modal && !state.owner.0.is_null() {
                        let _ = EnableWindow(state.owner, true);
                        let _ = SetForegroundWindow(state.owner);
                        state.modal = false;
                    }
                    if let Some(cb) = state.on_close.as_mut() {
                        cb();
                    }
                    state.on_close = None;
                }
                let is_main = MAIN.with(|c| c.get()) == hwnd.0 as isize;
                if is_main {
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }

            WM_NCDESTROY => {
                let ptr = state_ptr(hwnd);
                if !ptr.is_null() {
                    drop(Box::from_raw(ptr));
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            WM_NULL => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    ensure_timer(hwnd, state);
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                LRESULT(0)
            }

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

unsafe fn ime_result_string(hwnd: HWND) -> Vec<u16> {
    let himc = ImmGetContext(hwnd);
    if himc.0.is_null() {
        return Vec::new();
    }
    let bytes = ImmGetCompositionStringW(himc, GCS_RESULTSTR, None, 0);
    let mut out = Vec::new();
    if bytes > 0 {
        let count = bytes as usize / 2;
        let mut buf = vec![0u16; count];
        ImmGetCompositionStringW(
            himc,
            GCS_RESULTSTR,
            Some(buf.as_mut_ptr() as *mut c_void),
            bytes as u32,
        );
        out = buf;
    }
    let _ = ImmReleaseContext(hwnd, himc);
    out
}

fn mouse_xy(lparam: LPARAM) -> (f32, f32) {
    let x = (lparam.0 & 0xFFFF) as u16 as i16 as f32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as u16 as i16 as f32;
    (x, y)
}

#[inline]
fn loword(v: u32) -> u32 {
    v & 0xFFFF
}

#[inline]
fn hiword(v: u32) -> u32 {
    (v >> 16) & 0xFFFF
}

#[repr(C)]
struct AccentPolicy {
    state: u32,
    flags: u32,
    gradient_color: u32,
    animation_id: u32,
}

#[repr(C)]
struct WindowCompositionAttribData {
    attrib: u32,
    data: *mut core::ffi::c_void,
    size: usize,
}

type SetWca = unsafe extern "system" fn(HWND, *mut WindowCompositionAttribData) -> BOOL;

/// Применяет политику фона: 0 — выкл, 3 — размытие, 4 — акрил (Windows 10/11).
unsafe fn set_accent(hwnd: HWND, state: u32, gradient_color: u32) {
    let lib = match LoadLibraryW(w!("user32.dll")) {
        Ok(h) => h,
        Err(_) => return,
    };
    let proc = match GetProcAddress(lib, s!("SetWindowCompositionAttribute")) {
        Some(p) => p,
        None => return,
    };
    let set_wca: SetWca = core::mem::transmute(proc);
    let mut accent = AccentPolicy {
        state,
        flags: 2,
        gradient_color,
        animation_id: 0,
    };
    let mut data = WindowCompositionAttribData {
        attrib: 19,
        data: &mut accent as *mut _ as *mut core::ffi::c_void,
        size: core::mem::size_of::<AccentPolicy>(),
    };
    let _ = set_wca(hwnd, &mut data);
}
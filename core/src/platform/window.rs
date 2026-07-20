use core::ffi::c_void;
use std::cell::Cell;
use std::sync::Once;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, FillRect, InvalidateRect, UpdateWindow, ValidateRect, HDC,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
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
    DragAcceptFiles, DragFinish, DragQueryFileW, DragQueryPoint, HDROP,
};
use windows::Win32::UI::WindowsAndMessaging::*;

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
    pub center: bool,
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
            center: false,
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

            let (px, py) = if center {
                let (sw, sh) = if owner_hwnd.0.is_null() {
                    (
                        GetSystemMetrics(SM_CXSCREEN),
                        GetSystemMetrics(SM_CYSCREEN),
                    )
                } else {
                    let mut rc = RECT::default();
                    let _ = GetWindowRect(owner_hwnd, &mut rc);
                    (rc.right - rc.left, rc.bottom - rc.top)
                };
                let mut ox = 0;
                let mut oy = 0;
                if !owner_hwnd.0.is_null() {
                    let mut rc = RECT::default();
                    let _ = GetWindowRect(owner_hwnd, &mut rc);
                    ox = rc.left;
                    oy = rc.top;
                }
                (ox + (sw - width) / 2, oy + (sh - height) / 2)
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
        unsafe {
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
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
                        if state.idle > 8 {
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
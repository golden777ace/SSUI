use core::ffi::c_void;
use std::sync::Once;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::{InvalidateRect, UpdateWindow, ValidateRect};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::Ime::{
    ImmGetCompositionStringW, ImmGetContext, ImmReleaseContext, ImmSetCompositionWindow, CFS_POINT,
    COMPOSITIONFORM, GCS_RESULTSTR,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::render::{CursorKind, Renderer};
use crate::tree::Tree;

struct WindowState {
    renderer: Renderer,
}

pub struct Window {
    hwnd: HWND,
}

static REGISTER_CLASS: Once = Once::new();

impl Window {
    /// Создаёт окно, инициализирует рендерер деревом `tree` и показывает его.
    pub fn new(title: &str, width: i32, height: i32, tree: Tree) -> Result<Self> {
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

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                PCWSTR(title_w.as_ptr()),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width,
                height,
                None,
                None,
                Some(hinstance),
                None,
            )?;

            let renderer = Renderer::new(hwnd, tree)?;
            let state = Box::new(WindowState { renderer });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

            let _ = SetTimer(Some(hwnd), 1, 16, None);
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);

            Ok(Window { hwnd })
        }
    }

    /// HWND окна.
    pub fn hwnd(&self) -> HWND {
        self.hwnd
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

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    state.renderer.render();
                }
                let _ = ValidateRect(Some(hwnd), None);
                LRESULT(0)
            }

            WM_TIMER => {
                if let Some(state) = state_ptr(hwnd).as_mut() {
                    if state.renderer.on_timer() {
                        let _ = InvalidateRect(Some(hwnd), None, false);
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

            WM_ERASEBKGND => LRESULT(1),

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
                PostQuitMessage(0);
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
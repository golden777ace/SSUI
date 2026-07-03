use std::sync::Once;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::{UpdateWindow, ValidateRect};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use crate::render::Renderer;

struct WindowState {
    renderer: Renderer,
}

pub struct Window {
    hwnd: HWND,
}

static REGISTER_CLASS: Once = Once::new();

impl Window {
    pub fn new(title: &str, width: i32, height: i32) -> Result<Self> {
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
                hinstance,
                None,
            )?;

            let renderer = Renderer::new(hwnd)?;
            let state = Box::new(WindowState { renderer });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);

            Ok(Window { hwnd })
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

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
                let _ = ValidateRect(hwnd, None);
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

#[inline]
fn loword(v: u32) -> u32 {
    v & 0xFFFF
}

#[inline]
fn hiword(v: u32) -> u32 {
    (v >> 16) & 0xFFFF
}

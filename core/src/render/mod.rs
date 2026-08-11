#[cfg(windows)]
pub mod canvas;
#[cfg(windows)]
pub mod device;
pub mod paint;
pub mod types;

#[cfg(windows)]
pub use canvas::Canvas;
#[cfg(windows)]
pub use device::{CursorKind, Renderer};
pub use types::{parse_hex, Color, Rect};

#[cfg(windows)]
use std::cell::RefCell;
#[cfg(windows)]
use windows::core::w;
#[cfg(windows)]
use windows::Win32::Graphics::DirectWrite::*;

#[cfg(windows)]
thread_local! {
    static MEASURE_DW: RefCell<Option<IDWriteFactory>> = const { RefCell::new(None) };
}

/// Заглушки вне Windows: заменяются бэкендом Skia на фазе L2/L3.
#[cfg(not(windows))]
mod stub {
    /// Число кадров в файле изображения.
    pub fn frame_count(_path: &str) -> u32 {
        1
    }

    /// Кладёт текст в системный буфер обмена.
    pub fn clipboard_set(_text: &str) {}

    /// Возвращает текст из системного буфера.
    pub fn clipboard_get() -> String {
        String::new()
    }

    /// Ширина и высота строки в пикселях.
    pub fn measure_text(_text: &str, _family: &str, size: f32) -> (f32, f32) {
        (0.0, size.max(1.0))
    }
}

#[cfg(not(windows))]
pub use stub::{clipboard_get, clipboard_set, frame_count, measure_text};

/// Число кадров в файле изображения; для GIF — длина анимации.
#[cfg(windows)]
pub fn frame_count(path: &str) -> u32 {
    use windows::Win32::Foundation::GENERIC_READ;
    use windows::Win32::Graphics::Imaging::*;
    use windows::Win32::System::Com::*;
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let wic: IWICImagingFactory =
            match CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) {
                Ok(f) => f,
                Err(_) => return 1,
            };
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let decoder = wic.CreateDecoderFromFilename(
            windows::core::PCWSTR(wide.as_ptr()),
            None,
            GENERIC_READ,
            WICDecodeMetadataCacheOnLoad,
        );
        match decoder {
            Ok(d) => d.GetFrameCount().unwrap_or(1).max(1),
            Err(_) => 1,
        }
    }
}

/// Кладёт текст в системный буфер обмена.
#[cfg(windows)]
pub fn clipboard_set(text: &str) {
    let wide: Vec<u16> = text.encode_utf16().collect();
    device::set_clipboard_text(&wide);
}

/// Возвращает текст из системного буфера; переводы строк — `\n`.
#[cfg(windows)]
pub fn clipboard_get() -> String {
    let raw = device::get_clipboard_text();
    let s = String::from_utf16_lossy(&raw);
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Ширина и высота строки в пикселях для семейства и размера шрифта.
#[cfg(windows)]
pub fn measure_text(text: &str, family: &str, size: f32) -> (f32, f32) {
    MEASURE_DW.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).ok() };
        }
        let Some(dw) = slot.as_ref() else {
            return (0.0, 0.0);
        };
        let fam: Vec<u16> = family.encode_utf16().chain(std::iter::once(0)).collect();
        let body: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            let fmt = dw.CreateTextFormat(
                windows::core::PCWSTR(fam.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size.max(1.0),
                w!("en-us"),
            );
            let Ok(fmt) = fmt else {
                return (0.0, 0.0);
            };
            let layout = dw.CreateTextLayout(&body, &fmt, 1.0e6, 1.0e6);
            let Ok(layout) = layout else {
                return (0.0, 0.0);
            };
            let mut m = DWRITE_TEXT_METRICS::default();
            if layout.GetMetrics(&mut m).is_err() {
                return (0.0, 0.0);
            }
            (m.widthIncludingTrailingWhitespace, m.height)
        }
    })
}
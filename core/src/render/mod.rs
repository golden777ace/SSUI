pub mod canvas;
pub mod device;
pub mod types;

pub use canvas::Canvas;
pub use device::{CursorKind, Renderer};
pub use types::{parse_hex, Color, Rect};

use std::cell::RefCell;
use windows::core::w;
use windows::Win32::Graphics::DirectWrite::*;

thread_local! {
    static MEASURE_DW: RefCell<Option<IDWriteFactory>> = const { RefCell::new(None) };
}

/// Ширина и высота строки в пикселях для семейства и размера шрифта.
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
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use skia_safe::textlayout::{
    FontCollection, Paragraph, ParagraphBuilder, ParagraphStyle, RectHeightStyle, RectWidthStyle,
    TextAlign, TextStyle,
};
use skia_safe::{Canvas as SkCanvas, Color4f, FontMgr, FontStyle, Paint, Point as SkPoint};

use super::painter::TextFormat;
use crate::backend::{Point, TextEngine};
use crate::render::types::{Color, Rect};

const CACHE_LIMIT: usize = 4096;

#[derive(Clone, PartialEq, Eq, Hash)]
struct ParaKey {
    text: Vec<u16>,
    family: String,
    size_bits: u32,
    bold: bool,
    align: u8,
    wrap: bool,
    width_bits: u32,
    color: u32,
}

/// Раскладчик текста поверх Skia.
pub struct SkiaText {
    fonts: FontCollection,
    cache: HashMap<ParaKey, Paragraph>,
}

impl SkiaText {
    /// Создаёт движок с системным менеджером шрифтов.
    pub fn new() -> Self {
        let mut fonts = FontCollection::new();
        fonts.set_default_font_manager(FontMgr::new(), None);
        Self {
            fonts,
            cache: HashMap::new(),
        }
    }

    /// Сбрасывает кэш; вызывается при смене базового шрифта.
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    fn key(text: &[u16], fmt: &TextFormat, width: f32, color: Color) -> ParaKey {
        ParaKey {
            text: text.to_vec(),
            family: fmt.family.clone(),
            size_bits: fmt.size.max(1.0).to_bits(),
            bold: fmt.bold,
            align: fmt.align,
            wrap: fmt.wrap,
            width_bits: width.max(1.0).to_bits(),
            color: color.pack(),
        }
    }

    fn build(&mut self, text: &[u16], fmt: &TextFormat, width: f32, color: Color) -> Paragraph {
        let mut style = ParagraphStyle::new();
        style.set_text_align(if fmt.align == 1 {
            TextAlign::Left
        } else {
            TextAlign::Center
        });
        if !fmt.wrap {
            style.set_max_lines(1);
        }
        let mut ts = TextStyle::new();
        ts.set_font_families(&[fmt.family.as_str()]);
        ts.set_font_size(fmt.size.max(1.0));
        ts.set_font_style(if fmt.bold {
            FontStyle::bold()
        } else {
            FontStyle::normal()
        });
        let paint = Paint::new(Color4f::new(color.r, color.g, color.b, color.a), None);
        ts.set_foreground_paint(&paint);
        style.set_text_style(&ts);

        let mut builder = ParagraphBuilder::new(&style, self.fonts.clone());
        builder.add_text(&String::from_utf16_lossy(text));
        let mut para = builder.build();
        para.layout(width.max(1.0));
        para
    }

    /// Готовый параграф из кэша; при промахе строит и запоминает.
    fn paragraph(
        &mut self,
        text: &[u16],
        fmt: &TextFormat,
        width: f32,
        color: Color,
    ) -> &mut Paragraph {
        let key = Self::key(text, fmt, width, color);
        if !self.cache.contains_key(&key) {
            if self.cache.len() >= CACHE_LIMIT {
                self.cache.clear();
            }
            let para = self.build(text, fmt, width, color);
            self.cache.insert(key.clone(), para);
        }
        self.cache.get_mut(&key).expect("параграф только что вставлен")
    }

    /// Рисует текст в прямоугольнике; вертикально центрирует по флагу.
    pub fn draw(&mut self, canvas: &SkCanvas, text: &[u16], fmt: &TextFormat, rect: Rect, color: Color) {
        if text.is_empty() {
            return;
        }
        let middle = fmt.middle;
        let para = self.paragraph(text, fmt, rect.width.max(1.0), color);
        let y = if middle {
            rect.y + (rect.height - para.height()) / 2.0
        } else {
            rect.y
        };
        para.paint(canvas, SkPoint::new(rect.x, y));
    }
}

impl Default for SkiaText {
    fn default() -> Self {
        Self::new()
    }
}

impl TextEngine for SkiaText {
    type Format = TextFormat;

    fn width(&mut self, text: &[u16], format: &TextFormat) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let para = self.paragraph(text, format, 100_000.0, Color::rgb(0.0, 0.0, 0.0));
        para.max_intrinsic_width()
    }

    fn height(&mut self, text: &[u16], format: &TextFormat, width: f32) -> f32 {
        if text.is_empty() {
            return format.size.max(1.0);
        }
        let para = self.paragraph(text, format, width, Color::rgb(0.0, 0.0, 0.0));
        para.height()
    }

    fn caret(&mut self, text: &[u16], format: &TextFormat, width: f32, pos: usize) -> Point {
        let bytes = utf16_to_utf8(text, pos);
        if bytes == 0 {
            return Point::new(0.0, 0.0);
        }
        let para = self.paragraph(text, format, width, Color::rgb(0.0, 0.0, 0.0));
        let boxes = para.get_rects_for_range(0..bytes, RectHeightStyle::Max, RectWidthStyle::Tight);
        match boxes.last() {
            Some(b) => Point::new(b.rect.right, b.rect.top),
            None => Point::new(0.0, 0.0),
        }
    }

    fn hit(&mut self, text: &[u16], format: &TextFormat, width: f32, p: Point) -> usize {
        if text.is_empty() {
            return 0;
        }
        let para = self.paragraph(text, format, width, Color::rgb(0.0, 0.0, 0.0));
        let hit = para.get_glyph_position_at_coordinate((p.x, p.y));
        utf8_to_utf16(text, hit.position.max(0) as usize)
    }

    fn ranges(
        &mut self,
        text: &[u16],
        format: &TextFormat,
        width: f32,
        a: usize,
        b: usize,
    ) -> Vec<Rect> {
        if text.is_empty() || b <= a {
            return Vec::new();
        }
        let from = utf16_to_utf8(text, a);
        let to = utf16_to_utf8(text, b);
        let para = self.paragraph(text, format, width, Color::rgb(0.0, 0.0, 0.0));
        para.get_rects_for_range(from..to, RectHeightStyle::Max, RectWidthStyle::Tight)
            .iter()
            .map(|b| {
                Rect::new(
                    b.rect.left,
                    b.rect.top,
                    b.rect.right - b.rect.left,
                    b.rect.bottom - b.rect.top,
                )
            })
            .collect()
    }
}

/// Общий кэш текста: один движок на рисовальщик и измерения.
#[derive(Clone)]
pub struct SharedText(pub Rc<RefCell<SkiaText>>);

impl SharedText {
    /// Создаёт общий движок.
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(SkiaText::new())))
    }
}

impl Default for SharedText {
    fn default() -> Self {
        Self::new()
    }
}

impl TextEngine for SharedText {
    type Format = TextFormat;

    fn width(&mut self, text: &[u16], format: &TextFormat) -> f32 {
        self.0.borrow_mut().width(text, format)
    }

    fn height(&mut self, text: &[u16], format: &TextFormat, width: f32) -> f32 {
        self.0.borrow_mut().height(text, format, width)
    }

    fn caret(&mut self, text: &[u16], format: &TextFormat, width: f32, pos: usize) -> Point {
        self.0.borrow_mut().caret(text, format, width, pos)
    }

    fn hit(&mut self, text: &[u16], format: &TextFormat, width: f32, p: Point) -> usize {
        self.0.borrow_mut().hit(text, format, width, p)
    }

    fn ranges(
        &mut self,
        text: &[u16],
        format: &TextFormat,
        width: f32,
        a: usize,
        b: usize,
    ) -> Vec<Rect> {
        self.0.borrow_mut().ranges(text, format, width, a, b)
    }
}

/// Смещение в байтах UTF-8 для позиции `pos` в кодовых единицах UTF-16.
fn utf16_to_utf8(text: &[u16], pos: usize) -> usize {
    let cut = pos.min(text.len());
    String::from_utf16_lossy(&text[..cut]).len()
}

/// Позиция в кодовых единицах UTF-16 для смещения `bytes` в UTF-8.
fn utf8_to_utf16(text: &[u16], bytes: usize) -> usize {
    let full = String::from_utf16_lossy(text);
    let cut = bytes.min(full.len());
    let mut edge = cut;
    while edge > 0 && !full.is_char_boundary(edge) {
        edge -= 1;
    }
    full[..edge].encode_utf16().count()
}
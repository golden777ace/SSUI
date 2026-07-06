use crate::render::types::Color;

#[derive(Clone, Copy)]
pub struct Theme {
    pub background: Color,
    pub surface: Color,
    pub content: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub accent_pressed: Color,
    pub on_accent: Color,
}

impl Theme {
    /// Белая минималистичная тема (3 цвета).
    pub fn white() -> Self {
        Self {
            background: Color::hex(0xFFFFFF),
            surface: Color::hex(0xF5F5F5),
            content: Color::hex(0xAAAAAA),
            accent: Color::hex(0xF5F5F5),
            accent_hover: Color::hex(0xEAEAEA),
            accent_pressed: Color::hex(0xDDDDDD),
            on_accent: Color::hex(0xAAAAAA),
        }
    }

    /// Светлая тема.
    pub fn light() -> Self {
        Self {
            background: Color::hex(0xF7F8FA),
            surface: Color::hex(0xFFFFFF),
            content: Color::hex(0x1B1F24),
            accent: Color::hex(0x2F6FEB),
            accent_hover: Color::hex(0x3B7BF5),
            accent_pressed: Color::hex(0x2559C4),
            on_accent: Color::hex(0xFFFFFF),
        }
    }

    /// Тёмная тема.
    pub fn dark() -> Self {
        Self {
            background: Color::hex(0x0E1116),
            surface: Color::hex(0x171A21),
            content: Color::hex(0xE6E8EB),
            accent: Color::hex(0x3B82F6),
            accent_hover: Color::hex(0x4B8FF7),
            accent_pressed: Color::hex(0x2E6AD1),
            on_accent: Color::hex(0xFFFFFF),
        }
    }

    /// Чёрная минималистичная тема (3 цвета).
    pub fn black() -> Self {
        Self {
            background: Color::hex(0x000000),
            surface: Color::hex(0x1A1A1A),
            content: Color::hex(0x424242),
            accent: Color::hex(0x1A1A1A),
            accent_hover: Color::hex(0x242424),
            accent_pressed: Color::hex(0x101010),
            on_accent: Color::hex(0x424242),
        }
    }
}

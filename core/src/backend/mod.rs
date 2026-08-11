#[cfg(all(target_os = "linux", feature = "linux-skia"))]
pub mod linux;

use crate::render::types::{Color, Rect};

/// Точка в пикселях поверхности.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    /// Создаёт точку из координат.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Направление линейного градиента: 0 — вниз, 1 — вправо, 2 — вниз-вправо,
/// 3 — вверх-вправо.
pub type GradDir = u8;

/// Способ вписывания картинки: 0 — contain, 1 — cover, 2 — fill, 3 — none.
pub type ImageFit = u8;

/// Идентификатор загруженного изображения в кэше бэкенда.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageId(pub u32);

/// Начертание текста: обычное или полужирное.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weight {
    Normal,
    Bold,
}

/// Горизонтальное выравнивание строки.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Ключ layout'а текста: описывает строку целиком и служит ключом кэша
/// обоих бэкендов (IDWriteTextLayout / skia Paragraph).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextKey {
    /// Текст в UTF-16 без завершающего нуля.
    pub text: Vec<u16>,
    /// Индекс семейства шрифта в таблице `tree::intern_font`.
    pub font: u16,
    /// Размер шрифта в пикселях, биты `f32`.
    pub size_bits: u32,
    pub weight: Weight,
    pub align: Align,
    /// Перенос строк по словам.
    pub wrap: bool,
    /// Ширина области раскладки в пикселях, биты `f32`.
    pub width_bits: u32,
    /// Высота области раскладки в пикселях, биты `f32`.
    pub height_bits: u32,
}

impl TextKey {
    /// Собирает ключ из текста, шрифта и размеров области.
    pub fn new(
        text: &[u16],
        font: u16,
        size: f32,
        weight: Weight,
        align: Align,
        wrap: bool,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            text: text.to_vec(),
            font,
            size_bits: size.max(1.0).to_bits(),
            weight,
            align,
            wrap,
            width_bits: width.max(1.0).to_bits(),
            height_bits: height.max(1.0).to_bits(),
        }
    }

    /// Размер шрифта в пикселях.
    pub fn size(&self) -> f32 {
        f32::from_bits(self.size_bits)
    }

    /// Ширина области раскладки в пикселях.
    pub fn width(&self) -> f32 {
        f32::from_bits(self.width_bits)
    }

    /// Высота области раскладки в пикселях.
    pub fn height(&self) -> f32 {
        f32::from_bits(self.height_bits)
    }
}

/// Рисование примитивов на поверхности кадра. Набор методов повторяет
/// фактический Windows-`Canvas`, чтобы бэкенды давали полный паритет.
pub trait Painter {
    /// Формат текста бэкенда: `IDWriteTextFormat` или стиль Skia.
    type Format;
    /// Загруженное изображение бэкенда.
    type Image;

    /// Заливает всю поверхность цветом.
    fn clear(&mut self, color: Color);

    /// Заливает прямоугольник со скруглением `radius`.
    fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color);

    /// Рисует контур прямоугольника толщиной `width`.
    fn stroke_rect(&mut self, rect: Rect, width: f32, color: Color);

    /// Заливает эллипс, вписанный в прямоугольник.
    fn fill_ellipse(&mut self, rect: Rect, color: Color);

    /// Рисует контур эллипса толщиной `width`.
    fn stroke_ellipse(&mut self, rect: Rect, width: f32, color: Color);

    /// Рисует отрезок толщиной `width`.
    fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: Color);

    /// Заливает скруглённый прямоугольник линейным градиентом.
    fn fill_rounded_gradient(
        &mut self,
        rect: Rect,
        radius: f32,
        from: Color,
        to: Color,
        dir: GradDir,
    );

    /// Заливает многоугольник по списку вершин.
    fn fill_polygon(&mut self, pts: &[(f32, f32)], color: Color);

    /// Рисует незамкнутую ломаную заданной толщины.
    fn stroke_polyline(&mut self, pts: &[(f32, f32)], width: f32, color: Color);

    /// Рисует замкнутый контур многоугольника.
    fn stroke_polygon(&mut self, pts: &[(f32, f32)], width: f32, color: Color);

    /// Рисует стрелку; `head` — длина наконечника.
    fn draw_arrow(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        width: f32,
        head: f32,
        color: Color,
    );

    /// Рисует дугу: центр, радиус, начальный угол и разворот в градусах.
    fn stroke_arc(
        &mut self,
        cx: f32,
        cy: f32,
        r: f32,
        start: f32,
        sweep: f32,
        width: f32,
        color: Color,
    );

    /// Заливает сектор: центр, радиус, начальный угол и разворот.
    fn fill_sector(&mut self, cx: f32, cy: f32, r: f32, start: f32, sweep: f32, color: Color);

    /// Рисует строку текста внутри прямоугольника.
    fn draw_text(&mut self, text: &[u16], format: &Self::Format, rect: Rect, color: Color);

    /// Ограничивает дальнейшую отрисовку прямоугольником.
    fn push_clip(&mut self, rect: Rect);

    /// Снимает последнее ограничение отрисовки.
    fn pop_clip(&mut self);

    /// Рисует изображение в прямоугольник по режиму `fit`.
    fn draw_bitmap(&mut self, image: &Self::Image, rect: Rect, fit: ImageFit);
}

/// Слот формата: 0 — по центру, 1 — слева с вертикальным центром,
/// 2 — слева сверху с переносом по словам.
pub type FormatSlot = u8;

/// Источник форматов текста: превращает стиль узла в формат бэкенда.
pub trait FormatSource {
    /// Формат текста бэкенда.
    type Format;

    /// Формат для стиля узла; `bold` просит полужирное начертание,
    /// `default_size` действует, если размер в стиле не задан.
    fn format(
        &self,
        style: crate::tree::Style,
        slot: FormatSlot,
        bold: bool,
        default_size: f32,
    ) -> Self::Format;
}

/// Измерение и попадание по тексту.
pub trait TextEngine {
    /// Формат текста бэкенда.
    type Format;

    /// Ширина строки в пикселях.
    fn width(&mut self, text: &[u16], format: &Self::Format) -> f32;

    /// Высота раскладки строки в пикселях при ширине `width`.
    fn height(&mut self, text: &[u16], format: &Self::Format, width: f32) -> f32;

    /// Позиция каретки перед символом `pos`; координаты от начала области.
    fn caret(&mut self, text: &[u16], format: &Self::Format, width: f32, pos: usize) -> Point;

    /// Индекс символа под точкой.
    fn hit(&mut self, text: &[u16], format: &Self::Format, width: f32, p: Point) -> usize;

    /// Прямоугольники выделения символов `[a, b)`.
    fn ranges(
        &mut self,
        text: &[u16],
        format: &Self::Format,
        width: f32,
        a: usize,
        b: usize,
    ) -> Vec<Rect>;
}

/// Кнопка мыши: 0 — левая, 1 — правая, 2 — средняя.
pub type MouseButton = u8;

/// Событие ввода или жизненного цикла окна.
#[derive(Clone, Debug)]
pub enum Event {
    /// Движение курсора в клиентских координатах.
    MouseMove { x: f32, y: f32 },
    /// Нажатие кнопки мыши.
    MouseDown { x: f32, y: f32, btn: MouseButton },
    /// Отпускание кнопки мыши.
    MouseUp { x: f32, y: f32, btn: MouseButton },
    /// Двойной клик.
    MouseDouble { x: f32, y: f32, btn: MouseButton },
    /// Прокрутка колесом; `dy` в шагах.
    Wheel { x: f32, y: f32, dx: f32, dy: f32 },
    /// Курсор покинул окно.
    MouseLeave,
    /// Нажатие клавиши; `code` — виртуальный код, `mods` — битовая маска.
    KeyDown { code: u32, mods: u32 },
    /// Отпускание клавиши.
    KeyUp { code: u32, mods: u32 },
    /// Введённый символ.
    Char(char),
    /// Изменение размера клиентской области в пикселях.
    Resize { w: f32, h: f32 },
    /// Изменение масштаба экрана.
    Scale(f32),
    /// Окно получило или потеряло фокус.
    Focus(bool),
    /// Тик таймера кадра.
    Timer,
    /// Файлы, брошенные в окно.
    Dropped(Vec<std::path::PathBuf>),
    /// Запрос закрытия окна.
    Close,
}

/// Форма курсора.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    Arrow,
    Hand,
    Text,
    SizeWe,
    SizeNs,
    SizeNwse,
    SizeNesw,
    Wait,
}

/// Окно платформы и его цикл событий.
pub trait PlatformWindow {
    /// Запускает блокирующий цикл сообщений до закрытия окна.
    fn run(&self);

    /// Просит перерисовать окно.
    fn request_redraw(&self);

    /// Меняет заголовок окна.
    fn set_title(&self, title: &str);

    /// Размер клиентской области в пикселях.
    fn client_size(&self) -> (f32, f32);

    /// Масштаб окна: 1.0 при 96 DPI.
    fn scale(&self) -> f32;

    /// Поднимает окно поверх остальных и передаёт ему фокус.
    fn raise(&self);

    /// Закрывает окно.
    fn close(&self);
}
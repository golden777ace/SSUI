use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::render::types::{Color, Rect};
use std::cell::Cell;

mod css;

fn utf16z(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

thread_local! {
    static FONTS: RefCell<Vec<Vec<u16>>> = RefCell::new(vec![utf16z("Segoe UI")]);
    static BASE_SIZE: Cell<f32> = Cell::new(20.0);
}

/// Задаёт базовый шрифт приложения: имя и размер.
pub fn set_base_font(name: &str, size: f32) {
    FONTS.with(|f| f.borrow_mut()[0] = utf16z(name));
    BASE_SIZE.with(|s| s.set(size.max(1.0)));
}

/// Регистрирует семейство; возвращает его индекс.
pub fn intern_font(name: &str) -> u16 {
    let z = utf16z(name);
    FONTS.with(|f| {
        let mut v = f.borrow_mut();
        if let Some(i) = v.iter().position(|e| *e == z) {
            return i as u16;
        }
        v.push(z);
        (v.len() - 1) as u16
    })
}

/// UTF-16 имя семейства по индексу (с нулём).
pub fn font_utf16(idx: u16) -> Vec<u16> {
    FONTS.with(|f| {
        let v = f.borrow();
        v.get(idx as usize).cloned().unwrap_or_else(|| v[0].clone())
    })
}

/// Базовое семейство приложения (UTF-16, с нулём).
pub fn base_font() -> Vec<u16> {
    FONTS.with(|f| f.borrow()[0].clone())
}

/// Базовый размер шрифта приложения.
pub fn base_size() -> f32 {
    BASE_SIZE.with(|s| s.get())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeId(usize);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Axis {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy)]
pub struct Props {
    pub axis: Axis,
    pub padding: f32,
    pub gap: f32,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub grow: f32,
    pub justify: u8,
    pub cross: u8,
    pub abs: bool,
    pub l: Option<f32>,
    pub t: Option<f32>,
    pub r: Option<f32>,
    pub b: Option<f32>,
    pub mode: u8,
    pub row: u8,
    pub col: u8,
    pub rspan: u8,
    pub cspan: u8,
    pub side: u8,
    pub fill: u8,
    pub expand: bool,
    pub z: i32,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            axis: Axis::Vertical,
            padding: 0.0,
            gap: 0.0,
            width: None,
            height: None,
            grow: 0.0,
            justify: 0,
            cross: 0,
            abs: false,
            l: None,
            t: None,
            r: None,
            b: None,
            mode: 0,
            row: 0,
            col: 0,
            rspan: 1,
            cspan: 1,
            side: 0,
            fill: 0,
            expand: false,
            z: 0,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Style {
    pub fill: Option<Color>,
    pub text: Option<Color>,
    pub radius: Option<f32>,
    pub wrap: Option<bool>,
    pub elev: Option<f32>,
    pub grad: Option<(Color, Color)>,
    pub grad_dir: u8,
    pub font: Option<u16>,
    pub size: Option<f32>,
}

#[derive(Clone)]
pub struct TextState {
    pub text: Vec<u16>,
    pub caret: usize,
    pub anchor: usize,
    pub scroll: f32,
    undo: Vec<(Vec<u16>, usize)>,
    redo: Vec<(Vec<u16>, usize)>,
}

impl TextState {
    pub fn new() -> Self {
        Self {
            text: Vec::new(),
            caret: 0,
            anchor: 0,
            scroll: 0.0,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Возвращает границы выделения `(начало, конец)`.
    pub fn sel_range(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    fn snapshot(&mut self) {
        self.undo.push((self.text.clone(), self.caret));
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn delete_range(&mut self) -> bool {
        let (a, b) = self.sel_range();
        if a != b {
            self.text.drain(a..b);
            self.caret = a;
            self.anchor = a;
            true
        } else {
            false
        }
    }

    /// Вставляет символы в позицию каретки, заменяя выделение.
    pub fn insert(&mut self, chars: &[u16]) {
        self.snapshot();
        self.delete_range();
        for (i, &c) in chars.iter().enumerate() {
            self.text.insert(self.caret + i, c);
        }
        self.caret += chars.len();
        self.anchor = self.caret;
    }

    /// Удаляет символ перед кареткой или выделение.
    pub fn backspace(&mut self) {
        self.snapshot();
        if !self.delete_range() && self.caret > 0 {
            self.text.remove(self.caret - 1);
            self.caret -= 1;
        }
        self.anchor = self.caret;
    }

    /// Удаляет символ после каретки или выделение.
    pub fn delete_forward(&mut self) {
        self.snapshot();
        if !self.delete_range() && self.caret < self.text.len() {
            self.text.remove(self.caret);
        }
        self.anchor = self.caret;
    }

    /// Двигает каретку влево (с расширением выделения при `extend`).
    pub fn move_left(&mut self, extend: bool) {
        if !extend && self.caret != self.anchor {
            self.caret = self.sel_range().0;
        } else if self.caret > 0 {
            self.caret -= 1;
        }
        if !extend {
            self.anchor = self.caret;
        }
    }

    /// Двигает каретку вправо (с расширением выделения при `extend`).
    pub fn move_right(&mut self, extend: bool) {
        if !extend && self.caret != self.anchor {
            self.caret = self.sel_range().1;
        } else if self.caret < self.text.len() {
            self.caret += 1;
        }
        if !extend {
            self.anchor = self.caret;
        }
    }

    /// Переносит каретку в начало.
    pub fn home(&mut self, extend: bool) {
        self.caret = 0;
        if !extend {
            self.anchor = 0;
        }
    }

    /// Переносит каретку в конец.
    pub fn end(&mut self, extend: bool) {
        self.caret = self.text.len();
        if !extend {
            self.anchor = self.caret;
        }
    }

    /// Выделяет весь текст.
    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.caret = self.text.len();
    }

    /// Устанавливает каретку по индексу.
    pub fn set_caret(&mut self, index: usize, extend: bool) {
        self.caret = index.min(self.text.len());
        if !extend {
            self.anchor = self.caret;
        }
    }

    /// Выделяет слово вокруг позиции `index`.
    pub fn select_word(&mut self, index: usize) {
        let n = self.text.len();
        if n == 0 {
            return;
        }
        let i = index.min(n.saturating_sub(1));
        let is_word = |c: u16| {
            let ch = char::from_u32(c as u32).unwrap_or(' ');
            ch.is_alphanumeric() || ch == '_'
        };
        if !is_word(self.text[i]) {
            self.anchor = i;
            self.caret = (i + 1).min(n);
            return;
        }
        let mut a = i;
        while a > 0 && is_word(self.text[a - 1]) {
            a -= 1;
        }
        let mut b = i + 1;
        while b < n && is_word(self.text[b]) {
            b += 1;
        }
        self.anchor = a;
        self.caret = b;
    }

    /// Отменяет последнее изменение.
    pub fn undo(&mut self) {
        if let Some((t, c)) = self.undo.pop() {
            self.redo.push((self.text.clone(), self.caret));
            self.text = t;
            self.caret = c.min(self.text.len());
            self.anchor = self.caret;
        }
    }

    /// Повторяет отменённое изменение.
    pub fn redo(&mut self) {
        if let Some((t, c)) = self.redo.pop() {
            self.undo.push((self.text.clone(), self.caret));
            self.text = t;
            self.caret = c.min(self.text.len());
            self.anchor = self.caret;
        }
    }
}

impl Default for TextState {
    fn default() -> Self {
        Self::new()
    }
}

pub enum NodeKind {
    Container,
    Frame { radius: f32 },
    Label { text: Vec<u16> },
    Button { label: Vec<u16>, radius: f32 },
    Slider { value: f32 },
    Progress { value: f32 },
    Checkbox { label: Vec<u16>, checked: bool },
    TextBox { state: TextState },
    Dropdown {
        options: Vec<Vec<u16>>,
        selected: usize,
        open: bool,
    },
    Tabs {
        labels: Vec<Vec<u16>>,
        selected: usize,
    },
    Table {
        columns: Vec<Vec<u16>>,
        rows: Vec<Vec<Vec<u16>>>,
        selected: Option<usize>,
        scroll: f32,
        hline: f32,
        vline: f32,
        cbg: Vec<((usize, usize), u32)>,
    },
    Image {
        path: String,
        fit: u8,
    },
    Switch {
        label: Vec<u16>,
        on: bool,
    },
    Radio {
        label: Vec<u16>,
        on: bool,
        group: u32,
    },
    Toggle {
        label: Vec<u16>,
        on: bool,
    },
    Separator {
        vertical: bool,
    },
    List {
        items: Vec<Vec<u16>>,
        selected: Option<usize>,
        scroll: f32,
        multi: Vec<usize>,
        msel: bool,
    },
    Group {
        title: Vec<u16>,
        radius: f32,
    },
    Link {
        label: Vec<u16>,
    },
    Accordion {
        title: Vec<u16>,
        open: bool,
        radius: f32,
    },
    Scroll {
        offset: f32,
        content: f32,
    },
    Stack {
        page: usize,
    },
    Splitter {
        ratio: f32,
        vertical: bool,
    },
    Spinner {
        phase: f32,
    },
    Gauge {
        value: f32,
        label: Vec<u16>,
    },
    Meter {
        value: f32,
        segments: usize,
    },
    Chart {
        values: Vec<f32>,
    },
    Range {
        lo: f32,
        hi: f32,
    },
    Status {
        text: Vec<u16>,
    },
    Split {
        label: Vec<u16>,
        options: Vec<Vec<u16>>,
        radius: f32,
    },
    MenuBar {
        titles: Vec<Vec<u16>>,
        items: Vec<Vec<Vec<u16>>>,
    },
    Dial {
        value: f32,
        label: Vec<u16>,
    },
    TreeView {
        items: Vec<TreeItem>,
        selected: Option<usize>,
        scroll: f32,
        cols: Vec<Vec<u16>>,
        widths: Vec<f32>,
        multi: Vec<usize>,
        msel: bool,
    },
    Calendar {
        year: i32,
        month: u32,
        day: u32,
    },
    Color {
        hue: f32,
        sat: f32,
        val: f32,
    },
    Time {
        hour: u32,
        minute: u32,
    },
    PropGrid {
        rows: Vec<(Vec<u16>, Vec<u16>)>,
        selected: Option<usize>,
        scroll: f32,
    },
    Badge {
        text: Vec<u16>,
        dot: bool,
    },
    Crumbs {
        items: Vec<Vec<u16>>,
    },
    Pager {
        page: usize,
        total: usize,
    },
    Rating {
        value: usize,
        max: usize,
    },
    Canvas {
        shapes: Vec<Shape>,
        ox: f32,
        oy: f32,
        rx: f32,
        ry: f32,
        rw: f32,
        rh: f32,
        scroll: bool,
    },
    Term {
        lines: Vec<Vec<u16>>,
        input: TextState,
        prompt: Vec<u16>,
        scroll: f32,
    },
    Dock {
        title: Vec<u16>,
        side: u8,
        size: f32,
        open: bool,
    },
    Drop {
        label: Vec<u16>,
    },
}

#[derive(Clone)]
pub struct TreeItem {
    pub depth: usize,
    pub label: Vec<u16>,
    pub open: bool,
    pub leaf: bool,
    pub values: Vec<Vec<u16>>,
    pub bg: Option<u32>,
    pub fg: Option<u32>,
    pub icon: Option<String>,
    pub cbg: Vec<Option<u32>>,
    pub cfg: Vec<Option<u32>>,
}

impl TreeItem {
    /// Создаёт строку без колонок, цветов и иконки.
    pub fn new(depth: usize, label: Vec<u16>, open: bool, leaf: bool) -> Self {
        Self {
            depth,
            label,
            open,
            leaf,
            values: Vec::new(),
            bg: None,
            fg: None,
            icon: None,
            cbg: Vec::new(),
            cfg: Vec::new(),
        }
    }
}

/// Высота заголовка колонок дерева в пикселях.
pub const TREE_HEADER: f32 = 34.0;

/// Границы колонок дерева: `(x, ширина)` в оконных координатах.
pub fn column_bounds(r: Rect, ncol: usize, widths: &[f32]) -> Vec<(f32, f32)> {
    let n = ncol.max(1);
    let avail = (r.width - SCROLLBAR_W - 4.0).max(1.0);
    let mut fixed = 0.0;
    let mut free = 0usize;
    for i in 0..n {
        let w = widths.get(i).copied().unwrap_or(0.0);
        if w > 0.0 {
            fixed += w;
        } else {
            free += 1;
        }
    }
    let rest = if free > 0 {
        ((avail - fixed) / free as f32).max(40.0)
    } else {
        0.0
    };
    let mut out = Vec::with_capacity(n);
    let mut x = r.x;
    for i in 0..n {
        let w = widths.get(i).copied().unwrap_or(0.0);
        let w = if w > 0.0 { w } else { rest };
        out.push((x, w));
        x += w;
    }
    out
}

#[derive(Clone)]
pub struct Shape {
    pub kind: u8,
    pub args: [f32; 6],
    pub color: u32,
    pub text: Vec<u16>,
    pub pts: Vec<f32>,
}

fn point_in_poly(px: f32, py: f32, pts: &[f32]) -> bool {
    let n = pts.len() / 2;
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (pts[i * 2], pts[i * 2 + 1]);
        let (xj, yj) = (pts[j * 2], pts[j * 2 + 1]);
        if (yi > py) != (yj > py) {
            let t = (py - yi) / (yj - yi);
            if px < xi + t * (xj - xi) {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

fn near_segment(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len2 = dx * dx + dy * dy;
    let t = if len2 <= 0.0 {
        0.0
    } else {
        (((px - x1) * dx + (py - y1) * dy) / len2).clamp(0.0, 1.0)
    };
    let cx = x1 + dx * t;
    let cy = y1 + dy * t;
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

/// Толщина разделителя в пикселях.
pub const SPLIT_W: f32 = 8.0;

/// Ширина зоны стрелки у кнопки с меню.
pub const SPLIT_ARROW: f32 = 34.0;

/// Высота строки всплывающего списка.
pub const POPUP_ROW: f32 = 34.0;

/// Ширина раздела строки меню.
pub const BAR_ITEM: f32 = 120.0;

/// Высота заголовка календаря и строки дней недели.
pub const CAL_HEADER: f32 = 40.0;
pub const CAL_WEEK: f32 = 24.0;

/// Высота заголовка группы в пикселях.
pub const GROUP_HEADER: f32 = 30.0;

/// Высота заголовка секции аккордеона в пикселях.
pub const ACC_HEADER: f32 = 40.0;

/// Высота заголовка док-панели в пикселях.
pub const DOCK_HEADER: f32 = 32.0;

/// Высота строки списка и ширина полосы прокрутки.
pub const LIST_ROW: f32 = 32.0;
pub const SCROLLBAR_W: f32 = 10.0;

/// Высота строки терминала и его поля ввода.
pub const TERM_ROW: f32 = 22.0;
pub const TERM_INPUT: f32 = 30.0;

/// Высота полосы вкладок в пикселях.
pub const TAB_HEADER: f32 = 40.0;

/// Высота заголовка таблицы и строки в пикселях.
pub const TABLE_HEADER: f32 = 34.0;
pub const TABLE_ROW: f32 = 30.0;

/// Координата укладки скрытых узлов.
pub const OFF_COORD: f32 = -1.0e6;
/// Порог: узлы левее/выше считаются скрытыми.
pub const OFF_LIMIT: f32 = -100000.0;
const OFF_RECT: Rect = Rect::new(OFF_COORD, OFF_COORD, 0.0, 0.0);

#[derive(Clone, Copy)]
pub enum Ease {
    Linear,
    In,
    Out,
    InOut,
}

pub struct Anim {
    from: f32,
    to: f32,
    elapsed: f32,
    dur: f32,
    ease: Ease,
    cb: Box<dyn FnMut(&mut Tree, f32)>,
}

impl Anim {
    /// Создаёт анимацию значения от `from` к `to` за `dur` секунд.
    pub fn new<F: FnMut(&mut Tree, f32) + 'static>(
        from: f32,
        to: f32,
        dur: f32,
        ease: Ease,
        cb: F,
    ) -> Self {
        Self {
            from,
            to,
            elapsed: 0.0,
            dur,
            ease,
            cb: Box::new(cb),
        }
    }
}

pub type AnimQueue = Rc<RefCell<Vec<Anim>>>;

pub struct DialogData {
    pub title: Vec<u16>,
    pub message: Vec<u16>,
    pub buttons: Vec<Vec<u16>>,
    pub cb: Box<dyn FnMut(&mut Tree, usize)>,
}

pub type DialogQueue = Rc<RefCell<Option<DialogData>>>;

pub struct NoteData {
    pub title: Vec<u16>,
    pub text: Vec<u16>,
    pub action: Vec<u16>,
    pub secs: f32,
    pub kind: u8,
    pub cb: Option<Box<dyn FnMut(&mut Tree)>>,
}

pub type NoteQueue = Rc<RefCell<Vec<NoteData>>>;

/// Запрос фокуса: узел, новый текст, выделить ли всё.
pub type FocusQueue = Rc<RefCell<Option<(Option<NodeId>, Option<Vec<u16>>, bool)>>>;

/// Прямоугольники узлов после раскладки, в координатах окна.
pub type RectTable = Rc<RefCell<Vec<Rect>>>;

/// Хук начала кадра; true — работа была, нужна перерисовка.
pub type FrameHook = Box<dyn FnMut(&mut Tree) -> bool>;

/// Колбэк указателя: индекс фигуры, x, y в координатах содержимого.
pub type PointerCb = Box<dyn FnMut(&mut Tree, i32, f32, f32)>;

/// Колбэк колеса: дельта в щелчках, x, y в координатах содержимого.
pub type WheelCb = Box<dyn FnMut(&mut Tree, f32, f32, f32)>;

/// Заявка на таймер: идентификатор, период в мс, одноразовость.
pub struct TimerReq {
    pub id: u64,
    pub ms: f32,
    pub once: bool,
    pub cb: Box<dyn FnMut(&mut Tree)>,
}

pub type TimerQueue = Rc<RefCell<Vec<TimerReq>>>;
pub type TimerKill = Rc<RefCell<Vec<u64>>>;

/// Заявка на прокрутку канвы: узел, операция, четыре числа.
/// Операция: 0 — область прокрутки, 1 — переход к точке.
pub type CanvasQueue = Rc<RefCell<Vec<(NodeId, u8, f32, f32, f32, f32)>>>;

/// Заявка дереву: узел, операция, список индексов.
/// Операция: 0 — выделение, 1 — показать строку, 2 — раскрыть, 3 — свернуть.
pub type TreeQueue = Rc<RefCell<Vec<(NodeId, u8, Vec<usize>)>>>;

/// Геометрия дерева: прямоугольник, заголовок, прокрутка,
/// границы колонок, порядок видимых строк.
pub type TreeGeomRow = (Rect, f32, f32, Vec<(f32, f32)>, Vec<usize>);
pub type TreeGeom = Rc<RefCell<HashMap<usize, TreeGeomRow>>>;

use std::time::{Duration, Instant};

struct Timer {
    id: u64,
    every: Duration,
    last: Instant,
    once: bool,
    dead: bool,
    cb: Box<dyn FnMut(&mut Tree)>,
}

impl NodeId {
    /// Порядковый номер узла в дереве.
    pub fn index(self) -> usize {
        self.0
    }
}

pub struct Node {
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub rect: Rect,
    pub kind: NodeKind,
    pub props: Props,
    pub style: Style,
    pub style_focus: Style,
    pub style_hover: Style,
    pub class_name: Option<String>,
    pub icon: Option<String>,
    pub tip: Option<Vec<u16>>,
    pub multiline: bool,
    on_click: Option<Box<dyn FnMut(&mut Tree)>>,
    on_change: Option<Box<dyn FnMut(&mut Tree, f32)>>,
    on_input: Option<Box<dyn FnMut(&mut Tree, &str)>>,
}

pub struct Tree {
    nodes: Vec<Node>,
    root: NodeId,
    theme: usize,
    anims: Vec<Anim>,
    pending: AnimQueue,
    menu_items: Vec<Vec<u16>>,
    pending_dialog: DialogQueue,
    pending_notes: NoteQueue,
    pending_theme: Rc<RefCell<Option<usize>>>,
    pending_font: Rc<RefCell<Option<(String, f32)>>>,
    pending_focus: FocusQueue,
    rects: RectTable,
    timers: Vec<Timer>,
    pending_timers: TimerQueue,
    kill_timers: TimerKill,
    pending_canvas: CanvasQueue,
    pending_tree: TreeQueue,
    geom_tree: TreeGeom,
    popup_layer: Option<NodeId>,
    on_point: HashMap<(usize, u8), PointerCb>,
    on_wheel: HashMap<usize, WheelCb>,
    on_frame: Option<FrameHook>,
    insp: bool,
    img_dirty: bool,
    ghosts: HashSet<usize>,
    fronts: HashSet<usize>,
    placeholders: HashMap<usize, Vec<u16>>,
    on_dialog: Option<Box<dyn FnMut(&mut Tree, usize)>>,
    hotkeys: Vec<(u8, u32, Box<dyn FnMut(&mut Tree)>)>,
    tint: f32,
    blur_mode: u32,
    blur_tint: u32,
    drag_smooth: bool,
    dirty: bool,
    last_root: Option<Rect>,
    toast: Option<(Vec<u16>, f32)>,
}

impl Tree {
    /// Создаёт дерево с пустым корневым контейнером.
    pub fn new() -> Self {
        let root = Node {
            parent: None,
            children: Vec::new(),
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            kind: NodeKind::Container,
            props: Props::default(),
            style: Style::default(),
            style_focus: Style::default(),
            style_hover: Style::default(),
            class_name: None,
            icon: None,
            tip: None,
            multiline: false,
            on_click: None,
            on_change: None,
            on_input: None,
        };
        Self {
            nodes: vec![root],
            root: NodeId(0),
            theme: 2,
            anims: Vec::new(),
            pending: Rc::new(RefCell::new(Vec::new())),
            menu_items: Vec::new(),
            pending_dialog: Rc::new(RefCell::new(None)),
            pending_notes: Rc::new(RefCell::new(Vec::new())),
            pending_theme: Rc::new(RefCell::new(None)),
            pending_font: Rc::new(RefCell::new(None)),
            pending_focus: Rc::new(RefCell::new(None)),
            rects: Rc::new(RefCell::new(Vec::new())),
            timers: Vec::new(),
            pending_timers: Rc::new(RefCell::new(Vec::new())),
            kill_timers: Rc::new(RefCell::new(Vec::new())),
            pending_canvas: Rc::new(RefCell::new(Vec::new())),
            pending_tree: Rc::new(RefCell::new(Vec::new())),
            geom_tree: Rc::new(RefCell::new(HashMap::new())),
            popup_layer: None,
            on_point: HashMap::new(),
            on_wheel: HashMap::new(),
            on_frame: None,
            insp: false,
            img_dirty: false,
            ghosts: HashSet::new(),
            fronts: HashSet::new(),
            placeholders: HashMap::new(),
            on_dialog: None,
            hotkeys: Vec::new(),
            tint: 0.0,
            blur_mode: 0,
            blur_tint: 0x40101418,
            drag_smooth: true,
            dirty: true,
            last_root: None,
            toast: None,
        }
    }

    /// Задаёт стартовую тему: 0=white, 1=light, 2=dark, 3=black.
    pub fn set_theme(&mut self, index: usize) {
        self.theme = index.min(3);
    }

    /// Возвращает индекс стартовой темы.
    pub fn theme(&self) -> usize {
        self.theme
    }

    /// Возвращает очередь смены темы для внешнего управления.
    pub fn theme_queue(&self) -> Rc<RefCell<Option<usize>>> {
        self.pending_theme.clone()
    }

    /// Забирает запрошенную тему.
    pub fn take_pending_theme(&mut self) -> Option<usize> {
        let t = self.pending_theme.borrow_mut().take();
        if let Some(i) = t {
            self.theme = i.min(3);
        }
        t
    }

    /// Возвращает очередь смены шрифта для внешнего управления.
    pub fn font_queue(&self) -> Rc<RefCell<Option<(String, f32)>>> {
        self.pending_font.clone()
    }

    /// Возвращает очередь фокуса для внешнего управления.
    pub fn focus_queue(&self) -> FocusQueue {
        self.pending_focus.clone()
    }

    /// Возвращает таблицу прямоугольников узлов.
    pub fn rect_table(&self) -> RectTable {
        self.rects.clone()
    }

    /// Обновляет таблицу прямоугольников после раскладки.
    pub fn publish_rects(&self) {
        let mut out = self.rects.borrow_mut();
        out.clear();
        out.extend(self.nodes.iter().map(|n| n.rect));
    }

    /// Забирает отложенный запрос фокуса.
    pub fn take_pending_focus(&mut self) -> Option<(Option<NodeId>, Option<Vec<u16>>, bool)> {
        self.pending_focus.borrow_mut().take()
    }

    /// Забирает запрошенный шрифт и применяет как базовый; true — если менялся.
    pub fn take_pending_font(&mut self) -> bool {
        let f = self.pending_font.borrow_mut().take();
        if let Some((name, size)) = f {
            set_base_font(&name, size);
            true
        } else {
            false
        }
    }

    /// Возвращает значение ползунка (0..1).
    pub fn slider_value(&self, id: NodeId) -> f32 {
        if let NodeKind::Slider { value } = &self.nodes[id.0].kind {
            *value
        } else {
            0.0
        }
    }

    /// Задаёт альфа-канал фона окна (0..1).
    pub fn set_tint(&mut self, value: f32) {
        self.tint = value.clamp(0.0, 1.0);
    }

    /// Возвращает альфа-канал фона окна.
    pub fn tint(&self) -> f32 {
        self.tint
    }

    /// Возвращает режим фонового размытия.
    pub fn blur_mode(&self) -> u32 {
        self.blur_mode
    }

    /// Задаёт тон фонового размытия (0xAABBGGRR).
    pub fn set_blur_tint(&mut self, tint: u32) {
        self.blur_tint = tint;
    }

    /// Возвращает тонировку размытия `0xAARRGGBB`.
    pub fn blur_tint(&self) -> u32 {
        self.blur_tint
    }

    /// Задаёт режим фонового размытия: 0 — нет, 3 — размытие.
    pub fn set_blur_mode(&mut self, mode: u32) {
        self.blur_mode = mode;
    }

    /// Гасить ли размытие при перемещении/ресайзе окна.
    pub fn set_drag_smooth(&mut self, value: bool) {
        self.drag_smooth = value;
    }

    /// Возвращает флаг гашения размытия при перемещении.
    pub fn drag_smooth(&self) -> bool {
        self.drag_smooth
    }

    /// Возвращает очередь анимаций для внешнего добавления.
    pub fn anim_queue(&self) -> AnimQueue {
        self.pending.clone()
    }

    /// Есть ли активные анимации или таймеры.
    pub fn has_anims(&self) -> bool {
        !self.anims.is_empty() || !self.pending.borrow().is_empty() || !self.timers.is_empty()
    }

    /// Есть ли в дереве вращающийся спиннер.
    pub fn has_spinner(&self) -> bool {
        self.nodes
            .iter()
            .any(|n| matches!(n.kind, NodeKind::Spinner { .. }))
    }

    /// Продвигает фазу всех спиннеров.
    pub fn spin(&mut self, dt: f32) {
        for n in self.nodes.iter_mut() {
            if let NodeKind::Spinner { phase } = &mut n.kind {
                *phase = (*phase + dt * 1.6) % 1.0;
            }
        }
    }

    /// Задаёт значение кругового индикатора (0..1).
    pub fn set_gauge_value(&mut self, id: NodeId, v: f32) {
        if let NodeKind::Gauge { value, .. } = &mut self.nodes[id.0].kind {
            *value = v.clamp(0.0, 1.0);
        }
    }

    /// Задаёт значение сегментной шкалы (0..1).
    pub fn set_meter_value(&mut self, id: NodeId, v: f32) {
        if let NodeKind::Meter { value, .. } = &mut self.nodes[id.0].kind {
            *value = v.clamp(0.0, 1.0);
        }
    }

    /// Задаёт данные столбчатой диаграммы.
    pub fn set_chart_values(&mut self, id: NodeId, data: Vec<f32>) {
        if let NodeKind::Chart { values } = &mut self.nodes[id.0].kind {
            *values = data;
        }
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        {
            let mut p = self.pending.borrow_mut();
            if !p.is_empty() {
                self.anims.append(&mut p);
            }
        }
        if self.anims.is_empty() {
            return false;
        }
        let mut anims = std::mem::take(&mut self.anims);
        for a in anims.iter_mut() {
            a.elapsed = (a.elapsed + dt).min(a.dur);
            let t = if a.dur > 0.0 { a.elapsed / a.dur } else { 1.0 };
            let v = a.from + (a.to - a.from) * ease_value(a.ease, t);
            (a.cb)(self, v);
        }
        anims.retain(|a| a.elapsed < a.dur);
        self.anims.append(&mut anims);
        true
    }

    /// Идентификатор корневого узла.
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// Задаёт свойства раскладки для узла.
    pub fn set_props(&mut self, id: NodeId, props: Props) {
        self.nodes[id.0].props = props;
    }

    /// Переопределяет цвета элемента поверх темы.
    pub fn set_style(&mut self, id: NodeId, style: Style) {
        self.nodes[id.0].style = style;
    }

    /// Задаёт CSS-класс элемента.
    pub fn set_class(&mut self, id: NodeId, name: Option<String>) {
        self.nodes[id.0].class_name = name;
    }

    /// Включает перенос строк для метки.
    pub fn set_wrap(&mut self, id: NodeId, on: bool) {
        self.nodes[id.0].style.wrap = Some(on);
    }

    /// Базовый шрифт приложения: семейство и размер.
    pub fn set_font(&self, name: &str, size: f32) {
        set_base_font(name, size);
    }

    /// Задаёт высоту тени (elevation) элемента.
    pub fn set_elev(&mut self, id: NodeId, elev: f32) {
        self.nodes[id.0].style.elev = Some(elev);
    }

    /// Размещает узел абсолютно внутри родителя.
    pub fn set_place(
        &mut self,
        id: NodeId,
        l: Option<f32>,
        t: Option<f32>,
        r: Option<f32>,
        b: Option<f32>,
        w: Option<f32>,
        h: Option<f32>,
    ) {
        let p = &mut self.nodes[id.0].props;
        p.abs = true;
        p.l = l;
        p.t = t;
        p.r = r;
        p.b = b;
        if w.is_some() {
            p.width = w;
        }
        if h.is_some() {
            p.height = h;
        }
        self.dirty = true;
    }

    /// Ставит узел в ячейку сетки родителя.
    pub fn set_grid(&mut self, id: NodeId, row: u8, col: u8, rspan: u8, cspan: u8) {
        {
            let p = &mut self.nodes[id.0].props;
            p.abs = false;
            p.row = row;
            p.col = col;
            p.rspan = rspan.max(1);
            p.cspan = cspan.max(1);
        }
        if let Some(parent) = self.nodes[id.0].parent {
            self.nodes[parent.0].props.mode = 1;
        }
        self.dirty = true;
    }

    /// Прижимает узел к стороне родителя (упаковка).
    pub fn set_pack(&mut self, id: NodeId, side: u8, fill: u8, expand: bool) {
        {
            let p = &mut self.nodes[id.0].props;
            p.abs = false;
            p.side = side;
            p.fill = fill;
            p.expand = expand;
        }
        if let Some(parent) = self.nodes[id.0].parent {
            self.nodes[parent.0].props.mode = 2;
        }
        self.dirty = true;
    }

    /// Задаёт отступ и зазор контейнера.
    pub fn set_box(&mut self, id: NodeId, pad: f32, gap: f32) {
        let p = &mut self.nodes[id.0].props;
        p.padding = pad;
        p.gap = gap;
        self.dirty = true;
    }

    /// Задаёт глубину узла: больше `z` — ближе к зрителю.
    pub fn set_depth(&mut self, id: NodeId, z: i32) {
        self.nodes[id.0].props.z = z;
    }

    /// Разбирает CSS с каскадом и применяет стили ко всем узлам.
    pub fn apply_css(&mut self, css: &str) {
        css::add_source(self, css);
        self.dirty = true;
    }

    /// Включает слежение за CSS-файлом и горячую перезагрузку.
    pub fn css_watch(&mut self, path: &str) {
        css::watch(self, path);
        self.dirty = true;
    }

    /// Перечитывает CSS-файл, если он изменился на диске.
    pub fn poll_css(&mut self) {
        if css::poll(self) {
            self.dirty = true;
        }
    }

    /// Меняет текст у узла-метки.
    pub fn set_label_text(&mut self, id: NodeId, text: Vec<u16>) {
        match &mut self.nodes[id.0].kind {
            NodeKind::Label { text: t } => *t = text,
            NodeKind::Status { text: t } => *t = text,
            NodeKind::Badge { text: t, .. } => *t = text,
            _ => {}
        }
    }

    /// Является ли узел хлебными крошками.
    pub fn is_crumbs(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Crumbs { .. })
    }

    /// Возвращает элементы хлебных крошек.
    pub fn crumb_items(&self, id: NodeId) -> Vec<Vec<u16>> {
        if let NodeKind::Crumbs { items } = &self.nodes[id.0].kind {
            items.clone()
        } else {
            Vec::new()
        }
    }

    /// Обрезает хлебные крошки до элемента `index` включительно.
    pub fn crumb_truncate(&mut self, id: NodeId, index: usize) {
        if let NodeKind::Crumbs { items } = &mut self.nodes[id.0].kind {
            items.truncate(index + 1);
        }
    }

    /// Задаёт элементы хлебных крошек.
    pub fn set_crumb_items(&mut self, id: NodeId, data: Vec<Vec<u16>>) {
        if let NodeKind::Crumbs { items } = &mut self.nodes[id.0].kind {
            *items = data;
        }
    }

    /// Является ли узел постраничной навигацией.
    pub fn is_pager(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Pager { .. })
    }

    /// Возвращает `(страница, всего)`.
    pub fn pager_state(&self, id: NodeId) -> (usize, usize) {
        if let NodeKind::Pager { page, total } = &self.nodes[id.0].kind {
            (*page, *total)
        } else {
            (0, 0)
        }
    }

    /// Задаёт текущую страницу.
    pub fn set_pager_page(&mut self, id: NodeId, index: usize) {
        if let NodeKind::Pager { page, total } = &mut self.nodes[id.0].kind {
            if *total > 0 {
                *page = index.min(*total - 1);
            }
        }
    }

    /// Является ли узел оценкой звёздами.
    pub fn is_rating(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Rating { .. })
    }

    /// Возвращает `(оценка, максимум)`.
    pub fn rating_state(&self, id: NodeId) -> (usize, usize) {
        if let NodeKind::Rating { value, max } = &self.nodes[id.0].kind {
            (*value, *max)
        } else {
            (0, 0)
        }
    }

    /// Задаёт оценку.
    pub fn set_rating_value(&mut self, id: NodeId, v: usize) {
        if let NodeKind::Rating { value, max } = &mut self.nodes[id.0].kind {
            *value = v.min(*max);
        }
    }

    /// Является ли узел областью рисования.
    pub fn is_canvas(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Canvas { .. })
    }

    /// Задаёт фигуры области рисования.
    pub fn set_canvas_shapes(&mut self, id: NodeId, data: Vec<Shape>) {
        if let NodeKind::Canvas { shapes, .. } = &mut self.nodes[id.0].kind {
            *shapes = data;
        }
    }

    /// Прокручивается ли область рисования.
    pub fn is_canvas_scroll(&self, id: NodeId) -> bool {
        matches!(&self.nodes[id.0].kind, NodeKind::Canvas { scroll, .. } if *scroll)
    }

    /// Возвращает смещение прокрутки области рисования.
    pub fn canvas_offset(&self, id: NodeId) -> (f32, f32) {
        if let NodeKind::Canvas { ox, oy, .. } = &self.nodes[id.0].kind {
            (*ox, *oy)
        } else {
            (0.0, 0.0)
        }
    }

    /// Возвращает очередь заявок на прокрутку канвы.
    pub fn canvas_queue(&self) -> CanvasQueue {
        self.pending_canvas.clone()
    }

    /// Применяет накопленные заявки; true — что-то изменилось.
    pub fn apply_canvas_queue(&mut self) -> bool {
        let list = std::mem::take(&mut *self.pending_canvas.borrow_mut());
        if list.is_empty() {
            return false;
        }
        for (id, op, a, b, c, d) in list {
            if id.0 >= self.nodes.len() {
                continue;
            }
            match op {
                0 => self.set_canvas_region(id, a, b, c, d),
                1 => self.set_canvas_view(id, a, b),
                2 => self.show_popup(id, a, b, c, d),
                _ => self.hide_popup(id),
            }
        }
        true
    }

    /// Задаёт виртуальную область прокрутки в координатах содержимого.
    pub fn set_canvas_region(&mut self, id: NodeId, x1: f32, y1: f32, x2: f32, y2: f32) {
        if let NodeKind::Canvas { rx, ry, rw, rh, .. } = &mut self.nodes[id.0].kind {
            *rx = x1.min(x2);
            *ry = y1.min(y2);
            *rw = (x2 - x1).abs();
            *rh = (y2 - y1).abs();
        }
        self.clamp_canvas(id);
    }

    /// Прокручивает область к точке содержимого.
    pub fn set_canvas_view(&mut self, id: NodeId, x: f32, y: f32) {
        if let NodeKind::Canvas { ox, oy, .. } = &mut self.nodes[id.0].kind {
            *ox = x;
            *oy = y;
        }
        self.clamp_canvas(id);
    }

    /// Сдвигает область прокрутки на `dx`, `dy` пикселей.
    pub fn canvas_scroll_by(&mut self, id: NodeId, dx: f32, dy: f32) {
        if let NodeKind::Canvas { ox, oy, .. } = &mut self.nodes[id.0].kind {
            *ox += dx;
            *oy += dy;
        }
        self.clamp_canvas(id);
    }

    fn clamp_canvas(&mut self, id: NodeId) {
        let view = self.nodes[id.0].rect;
        if let NodeKind::Canvas {
            ox,
            oy,
            rx,
            ry,
            rw,
            rh,
            ..
        } = &mut self.nodes[id.0].kind
        {
            let mx = (*rw - view.width).max(0.0);
            let my = (*rh - view.height).max(0.0);
            *ox = ox.clamp(*rx, *rx + mx);
            *oy = oy.clamp(*ry, *ry + my);
        }
        self.dirty = true;
    }

    /// Задаёт колбэк указателя: 0 — нажатие, 1 — движение, 2 — отпускание, 3 — двойной.
    pub fn set_on_point(&mut self, id: NodeId, phase: u8, cb: PointerCb) {
        self.on_point.insert((id.0, phase), cb);
    }

    /// Задан ли колбэк указателя для фазы.
    pub fn has_point(&self, id: NodeId, phase: u8) -> bool {
        self.on_point.contains_key(&(id.0, phase))
    }

    /// Вызывает колбэк указателя; true — колбэк был задан.
    pub fn fire_point(&mut self, id: NodeId, phase: u8, i: i32, x: f32, y: f32) -> bool {
        match self.on_point.remove(&(id.0, phase)) {
            Some(mut cb) => {
                cb(self, i, x, y);
                self.on_point.insert((id.0, phase), cb);
                true
            }
            None => false,
        }
    }

    /// Задаёт колбэк колеса мыши над узлом.
    pub fn set_on_wheel(&mut self, id: NodeId, cb: WheelCb) {
        self.on_wheel.insert(id.0, cb);
    }

    /// Задан ли колбэк колеса для узла.
    pub fn has_wheel(&self, id: NodeId) -> bool {
        self.on_wheel.contains_key(&id.0)
    }

    /// Вызывает колбэк колеса; true — колбэк был задан.
    pub fn fire_wheel(&mut self, id: NodeId, d: f32, x: f32, y: f32) -> bool {
        match self.on_wheel.remove(&id.0) {
            Some(mut cb) => {
                cb(self, d, x, y);
                self.on_wheel.insert(id.0, cb);
                true
            }
            None => false,
        }
    }

    /// Переводит точку окна в систему координат содержимого узла.
    pub fn canvas_local(&self, id: NodeId, x: f32, y: f32) -> (f32, f32) {
        let r = self.nodes[id.0].rect;
        let (ox, oy) = self.canvas_offset(id);
        (x - r.x + ox, y - r.y + oy)
    }

    /// Индекс верхней фигуры под точкой; координаты — экранные.
    pub fn canvas_hit(&self, id: NodeId, x: f32, y: f32) -> Option<usize> {
        let node = &self.nodes[id.0];
        let NodeKind::Canvas {
            shapes, ox, oy, ..
        } = &node.kind
        else {
            return None;
        };
        let lx = x - node.rect.x + ox;
        let ly = y - node.rect.y + oy;
        for (i, s) in shapes.iter().enumerate().rev() {
            let a = s.args;
            let hit = match s.kind {
                0 | 3 => {
                    lx >= a[0] && ly >= a[1] && lx <= a[0] + a[2] && ly <= a[1] + a[3]
                }
                1 => {
                    let dx = lx - a[0];
                    let dy = ly - a[1];
                    (dx * dx + dy * dy).sqrt() <= a[2].max(1.0)
                }
                2 | 4 => {
                    let tol = a[4].max(4.0);
                    near_segment(lx, ly, a[0], a[1], a[2], a[3]) <= tol
                }
                7 => point_in_poly(lx, ly, &s.pts),
                _ => {
                    let dx = lx - a[0];
                    let dy = ly - a[1];
                    let d = (dx * dx + dy * dy).sqrt();
                    if d > a[2].max(1.0) {
                        false
                    } else {
                        let mut ang = dy.atan2(dx).to_degrees() - a[3];
                        while ang < 0.0 {
                            ang += 360.0;
                        }
                        while ang >= 360.0 {
                            ang -= 360.0;
                        }
                        ang <= a[4].abs()
                    }
                }
            };
            if hit {
                return Some(i);
            }
        }
        None
    }

    /// Является ли узел терминалом.
    pub fn is_term(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Term { .. })
    }

    /// Число строк вывода терминала.
    pub fn term_len(&self, id: NodeId) -> usize {
        if let NodeKind::Term { lines, .. } = &self.nodes[id.0].kind {
            lines.len()
        } else {
            0
        }
    }

    /// Добавляет строку в вывод терминала.
    pub fn term_push(&mut self, id: NodeId, line: Vec<u16>) {
        if let NodeKind::Term { lines, .. } = &mut self.nodes[id.0].kind {
            lines.push(line);
        }
    }

    /// Очищает вывод терминала.
    pub fn term_clear(&mut self, id: NodeId) {
        if let NodeKind::Term { lines, scroll, .. } = &mut self.nodes[id.0].kind {
            lines.clear();
            *scroll = 0.0;
        }
    }

    /// Возвращает состояние ввода терминала.
    pub fn term_input_mut(&mut self, id: NodeId) -> Option<&mut TextState> {
        if let NodeKind::Term { input, .. } = &mut self.nodes[id.0].kind {
            Some(input)
        } else {
            None
        }
    }

    /// Забирает введённую строку терминала.
    pub fn term_take_input(&mut self, id: NodeId) -> Option<String> {
        if let NodeKind::Term { input, .. } = &mut self.nodes[id.0].kind {
            let text = String::from_utf16_lossy(&input.text);
            *input = TextState::new();
            Some(text)
        } else {
            None
        }
    }

    /// Возвращает прокрутку терминала.
    pub fn term_scroll(&self, id: NodeId) -> f32 {
        if let NodeKind::Term { scroll, .. } = &self.nodes[id.0].kind {
            *scroll
        } else {
            0.0
        }
    }

    /// Задаёт прокрутку терминала.
    pub fn set_term_scroll(&mut self, id: NodeId, value: f32) {
        if let NodeKind::Term { scroll, .. } = &mut self.nodes[id.0].kind {
            *scroll = value;
        }
    }

    /// Является ли узел док-панелью.
    pub fn is_dock(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Dock { .. })
    }

    /// Раскрыта ли док-панель.
    pub fn dock_open(&self, id: NodeId) -> bool {
        matches!(&self.nodes[id.0].kind, NodeKind::Dock { open, .. } if *open)
    }

    /// Сворачивает или раскрывает док-панель.
    pub fn toggle_dock(&mut self, id: NodeId) {
        let (open, size, side) = match &mut self.nodes[id.0].kind {
            NodeKind::Dock {
                open, size, side, ..
            } => {
                *open = !*open;
                (*open, *size, *side)
            }
            _ => return,
        };
        let main = if open { size } else { DOCK_HEADER };
        if side == 0 || side == 1 {
            self.nodes[id.0].props.width = Some(main);
        } else {
            self.nodes[id.0].props.height = Some(main);
        }
        self.dirty = true;
    }

    /// Является ли узел зоной приёма файлов.
    pub fn is_drop(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Drop { .. })
    }

    /// Назначает обработчик клика для узла.
    pub fn set_on_click<F: FnMut(&mut Tree) + 'static>(&mut self, id: NodeId, f: F) {
        self.nodes[id.0].on_click = Some(Box::new(f));
    }

    /// Назначает обработчик изменения значения (ползунок).
    pub fn set_on_change<F: FnMut(&mut Tree, f32) + 'static>(&mut self, id: NodeId, f: F) {
        self.nodes[id.0].on_change = Some(Box::new(f));
    }

    /// Вызывает обработчик ввода с произвольным текстом.
    pub fn fire_input_text(&mut self, id: NodeId, text: &str) {
        if let Some(mut cb) = self.nodes[id.0].on_input.take() {
            cb(self, text);
            self.nodes[id.0].on_input = Some(cb);
        }
    }

    fn take_on_click(&mut self, id: NodeId) -> Option<Box<dyn FnMut(&mut Tree)>> {
        self.nodes[id.0].on_click.take()
    }

    fn put_on_click(&mut self, id: NodeId, cb: Box<dyn FnMut(&mut Tree)>) {
        self.nodes[id.0].on_click = Some(cb);
    }

    fn take_on_change(&mut self, id: NodeId) -> Option<Box<dyn FnMut(&mut Tree, f32)>> {
        self.nodes[id.0].on_change.take()
    }

    fn put_on_change(&mut self, id: NodeId, cb: Box<dyn FnMut(&mut Tree, f32)>) {
        self.nodes[id.0].on_change = Some(cb);
    }

    /// Вызывает обработчик клика узла, если он назначен.
    pub fn fire_click(&mut self, id: NodeId) {
        if let Some(mut cb) = self.take_on_click(id) {
            cb(self);
            self.put_on_click(id, cb);
        }
    }

    /// Вызывает обработчик изменения значения, если он назначен.
    pub fn fire_change(&mut self, id: NodeId, value: f32) {
        if let Some(mut cb) = self.take_on_change(id) {
            cb(self, value);
            self.put_on_change(id, cb);
        }
    }

    /// Назначает обработчик ввода текста (поле ввода).
    pub fn set_on_input<F: FnMut(&mut Tree, &str) + 'static>(&mut self, id: NodeId, f: F) {
        self.nodes[id.0].on_input = Some(Box::new(f));
    }

    fn take_on_input(&mut self, id: NodeId) -> Option<Box<dyn FnMut(&mut Tree, &str)>> {
        self.nodes[id.0].on_input.take()
    }

    fn put_on_input(&mut self, id: NodeId, cb: Box<dyn FnMut(&mut Tree, &str)>) {
        self.nodes[id.0].on_input = Some(cb);
    }

    /// Вызывает обработчик ввода текста, если он назначен.
    pub fn fire_text_input(&mut self, id: NodeId) {
        let text = match self.textbox_state(id) {
            Some(s) => String::from_utf16_lossy(&s.text),
            None => return,
        };
        if let Some(mut cb) = self.take_on_input(id) {
            cb(self, &text);
            self.put_on_input(id, cb);
        }
    }

    /// Возвращает текущий текст поля ввода.
    pub fn textbox_text(&self, id: NodeId) -> Option<String> {
        self.textbox_state(id)
            .map(|s| String::from_utf16_lossy(&s.text))
    }

    /// Добавляет узел ребёнком к `parent` и возвращает его идентификатор.
    pub fn add_child(&mut self, parent: NodeId, kind: NodeKind, props: Props) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(Node {
            parent: Some(parent),
            children: Vec::new(),
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            kind,
            props,
            style: Style::default(),
            style_focus: Style::default(),
            style_hover: Style::default(),
            class_name: None,
            icon: None,
            tip: None,
            multiline: false,
            on_click: None,
            on_change: None,
            on_input: None,
        });
        self.nodes[parent.0].children.push(id);
        id
    }

    /// Задаёт режим вписывания изображения (0..3).
    pub fn set_image_fit(&mut self, id: NodeId, fit: u8) {
        if let NodeKind::Image { fit: f, .. } = &mut self.nodes[id.0].kind {
            *f = fit;
        }
    }

    /// Задаёт файл изображения; помечает кэш для дозагрузки.
    pub fn set_image_path(&mut self, id: NodeId, src: &str) {
        if let NodeKind::Image { path, .. } = &mut self.nodes[id.0].kind {
            if path != src {
                *path = src.to_string();
                self.img_dirty = true;
            }
        }
    }

    /// Забирает флаг необходимости дозагрузки изображений.
    pub fn take_img_dirty(&mut self) -> bool {
        std::mem::take(&mut self.img_dirty)
    }

    /// Ставит таймер в очередь регистрации; `once` — одноразовый.
    pub fn add_timer<F: FnMut(&mut Tree) + 'static>(
        &mut self,
        id: u64,
        ms: f32,
        once: bool,
        f: F,
    ) {
        self.pending_timers.borrow_mut().push(TimerReq {
            id,
            ms,
            once,
            cb: Box::new(f),
        });
    }

    /// Возвращает очередь регистрации таймеров.
    pub fn timer_queue(&self) -> TimerQueue {
        self.pending_timers.clone()
    }

    /// Возвращает очередь отмены таймеров.
    pub fn kill_queue(&self) -> TimerKill {
        self.kill_timers.clone()
    }

    /// Есть ли активные или ожидающие регистрации таймеры.
    pub fn has_timers(&self) -> bool {
        !self.timers.is_empty() || !self.pending_timers.borrow().is_empty()
    }

    /// Задаёт хук начала кадра; вызывается до раскладки.
    pub fn set_on_frame(&mut self, f: FrameHook) {
        self.on_frame = Some(f);
    }

    /// Вызывает хук начала кадра; true — работа была.
    pub fn fire_frame(&mut self) -> bool {
        match self.on_frame.take() {
            Some(mut f) => {
                let done = f(self);
                self.on_frame = Some(f);
                done
            }
            None => false,
        }
    }

    /// Разрешает вызов инспектора по F12.
    pub fn set_inspect(&mut self, on: bool) {
        self.insp = on;
    }

    /// Разрешён ли инспектор.
    pub fn inspect(&self) -> bool {
        self.insp
    }

    /// Прогоняет созревшие таймеры; true — хотя бы один сработал.
    pub fn tick_timers(&mut self) -> bool {
        let fresh = std::mem::take(&mut *self.pending_timers.borrow_mut());
        for r in fresh {
            self.timers.push(Timer {
                id: r.id,
                every: Duration::from_secs_f32(r.ms.max(1.0) / 1000.0),
                last: Instant::now(),
                once: r.once,
                dead: false,
                cb: r.cb,
            });
        }
        let kills = std::mem::take(&mut *self.kill_timers.borrow_mut());
        if !kills.is_empty() {
            self.timers.retain(|t| !kills.contains(&t.id));
        }
        if self.timers.is_empty() {
            return false;
        }
        let mut fired = false;
        let mut list = std::mem::take(&mut self.timers);
        let now = Instant::now();
        for t in list.iter_mut() {
            if now.duration_since(t.last) >= t.every {
                t.last = now;
                if t.once {
                    t.dead = true;
                }
                fired = true;
                (t.cb)(self);
            }
        }
        list.retain(|t| !t.dead);
        list.append(&mut self.timers);
        self.timers = list;
        fired
    }

    /// Задаёт иконку узла из файла.
    pub fn set_icon(&mut self, id: NodeId, path: &str) {
        self.nodes[id.0].icon = Some(path.to_string());
    }

    /// Задаёт всплывающую подсказку узла.
    pub fn set_tip(&mut self, id: NodeId, text: Vec<u16>) {
        self.nodes[id.0].tip = Some(text);
    }

    /// Возвращает подсказку узла.
    pub fn tip(&self, id: NodeId) -> Option<&[u16]> {
        self.nodes[id.0].tip.as_deref()
    }

    /// Ставит всплывающее уведомление на `secs` секунд.
    pub fn push_toast(&mut self, text: Vec<u16>, secs: f32) {
        self.toast = Some((text, secs));
    }

    /// Забирает уведомление из очереди.
    pub fn take_toast(&mut self) -> Option<(Vec<u16>, f32)> {
        self.toast.take()
    }

    /// Задаёт flex-вес узла вдоль главной оси.
    pub fn set_grow(&mut self, id: NodeId, g: f32) {
        self.nodes[id.0].props.grow = g;
    }

    /// Задаёт выравнивание детей: `justify` вдоль оси, `cross` поперёк.
    pub fn set_align(&mut self, id: NodeId, justify: u8, cross: u8) {
        self.nodes[id.0].props.justify = justify;
        self.nodes[id.0].props.cross = cross;
    }

    /// Пинит узел к краям родителя абсолютными отступами.
    pub fn set_pin(
        &mut self,
        id: NodeId,
        l: Option<f32>,
        t: Option<f32>,
        r: Option<f32>,
        b: Option<f32>,
    ) {
        let p = &mut self.nodes[id.0].props;
        p.abs = true;
        p.l = l;
        p.t = t;
        p.r = r;
        p.b = b;
    }


    /// Возвращает родителя узла.
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        for (i, n) in self.nodes.iter().enumerate() {
            if n.children.contains(&id) {
                return Some(NodeId(i));
            }
        }
        None
    }

    /// Возвращает узел по идентификатору.
    pub fn get(&self, id: NodeId) -> &Node {
        &self.nodes[id.0]
    }

    /// Является ли узел кнопкой.
    pub fn is_button(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Button { .. })
    }

    /// Является ли узел чекбоксом.
    pub fn is_checkbox(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Checkbox { .. })
    }

    /// Является ли узел полем ввода.
    pub fn is_textbox(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::TextBox { .. })
    }

    /// Является ли узел выпадающим списком.
    pub fn is_dropdown(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Dropdown { .. })
    }

    /// Открыт ли выпадающий список.
    pub fn dropdown_is_open(&self, id: NodeId) -> bool {
        if let NodeKind::Dropdown { open, .. } = &self.nodes[id.0].kind {
            *open
        } else {
            false
        }
    }

    /// Открывает или закрывает выпадающий список.
    pub fn set_dropdown_open(&mut self, id: NodeId, value: bool) {
        if let NodeKind::Dropdown { open, .. } = &mut self.nodes[id.0].kind {
            *open = value;
        }
    }

    /// Число пунктов выпадающего списка.
    pub fn dropdown_len(&self, id: NodeId) -> usize {
        if let NodeKind::Dropdown { options, .. } = &self.nodes[id.0].kind {
            options.len()
        } else {
            0
        }
    }

    /// Задаёт выбранный пункт выпадающего списка.
    pub fn set_dropdown_selected(&mut self, id: NodeId, index: usize) {
        if let NodeKind::Dropdown { selected, .. } = &mut self.nodes[id.0].kind {
            *selected = index;
        }
    }

    /// Возвращает выбранный пункт выпадающего списка.
    pub fn dropdown_selected(&self, id: NodeId) -> usize {
        if let NodeKind::Dropdown { selected, .. } = &self.nodes[id.0].kind {
            *selected
        } else {
            0
        }
    }

    /// Является ли узел вкладками.
    pub fn is_tabs(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Tabs { .. })
    }

    /// Число вкладок.
    pub fn tabs_len(&self, id: NodeId) -> usize {
        if let NodeKind::Tabs { labels, .. } = &self.nodes[id.0].kind {
            labels.len()
        } else {
            0
        }
    }

    /// Возвращает активную вкладку.
    pub fn tabs_selected(&self, id: NodeId) -> usize {
        if let NodeKind::Tabs { selected, .. } = &self.nodes[id.0].kind {
            *selected
        } else {
            0
        }
    }

    /// Задаёт активную вкладку.
    pub fn set_tabs_selected(&mut self, id: NodeId, index: usize) {
        if let NodeKind::Tabs { selected, .. } = &mut self.nodes[id.0].kind {
            *selected = index;
        }
        self.dirty = true;
    }

    /// Является ли узел таблицей.
    pub fn is_table(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Table { .. })
    }

    /// Число строк таблицы.
    pub fn table_len(&self, id: NodeId) -> usize {
        if let NodeKind::Table { rows, .. } = &self.nodes[id.0].kind {
            rows.len()
        } else {
            0
        }
    }

    /// Возвращает выбранную строку таблицы.
    pub fn table_selected(&self, id: NodeId) -> Option<usize> {
        if let NodeKind::Table { selected, .. } = &self.nodes[id.0].kind {
            *selected
        } else {
            None
        }
    }

    /// Задаёт выбранную строку таблицы.
    pub fn set_table_selected(&mut self, id: NodeId, index: Option<usize>) {
        if let NodeKind::Table { selected, .. } = &mut self.nodes[id.0].kind {
            *selected = index;
        }
    }

    /// Заменяет строки таблицы, сохраняя выбор в пределах длины.
    pub fn set_table_rows(&mut self, id: NodeId, data: Vec<Vec<Vec<u16>>>) {
        if let NodeKind::Table {
            rows, selected, ..
        } = &mut self.nodes[id.0].kind
        {
            if let Some(s) = selected {
                if *s >= data.len() {
                    *selected = None;
                }
            }
            *rows = data;
            self.dirty = true;
        }
    }

    /// Возвращает вертикальную прокрутку таблицы в пикселях.
    pub fn table_scroll(&self, id: NodeId) -> f32 {
        if let NodeKind::Table { scroll, .. } = &self.nodes[id.0].kind {
            *scroll
        } else {
            0.0
        }
    }

    /// Задаёт вертикальную прокрутку таблицы в пикселях.
    pub fn set_table_scroll(&mut self, id: NodeId, value: f32) {
        if let NodeKind::Table { scroll, .. } = &mut self.nodes[id.0].kind {
            *scroll = value;
        }
    }

    /// Задаёт цвета ячеек таблицы: `((строка, колонка), цвет)`.
    pub fn set_table_cbg(&mut self, id: NodeId, data: Vec<((usize, usize), u32)>) {
        if let NodeKind::Table { cbg, .. } = &mut self.nodes[id.0].kind {
            *cbg = data;
        }
        self.dirty = true;
    }

    /// Число колонок таблицы.
    pub fn table_cols(&self, id: NodeId) -> usize {
        if let NodeKind::Table { columns, .. } = &self.nodes[id.0].kind {
            columns.len().max(1)
        } else {
            1
        }
    }

    /// Индекс колонки таблицы под точкой окна.
    pub fn table_col_at(&self, id: NodeId, x: f32) -> usize {
        let r = self.nodes[id.0].rect;
        let ncol = self.table_cols(id);
        let cw = r.width / ncol as f32;
        if cw <= 0.0 {
            return 0;
        }
        (((x - r.x) / cw).floor() as i32).clamp(0, ncol as i32 - 1) as usize
    }

    /// Прокручивает таблицу так, чтобы строка была видна.
    pub fn reveal_table(&mut self, id: NodeId, row: usize) {
        let r = self.nodes[id.0].rect;
        let view = (r.height - TABLE_HEADER).max(0.0);
        let top = row as f32 * TABLE_ROW;
        let mut s = self.table_scroll(id);
        if top < s {
            s = top;
        }
        if top + TABLE_ROW > s + view {
            s = top + TABLE_ROW - view;
        }
        let n = self.table_len(id);
        let max = (n as f32 * TABLE_ROW - view).max(0.0);
        self.set_table_scroll(id, s.clamp(0.0, max));
    }

    /// Задаёт пункты контекстного меню окна.
    pub fn set_menu(&mut self, items: Vec<Vec<u16>>) {
        self.menu_items = items;
    }

    /// Число пунктов контекстного меню.
    pub fn menu_len(&self) -> usize {
        self.menu_items.len()
    }

    /// Возвращает пункт контекстного меню по индексу.
    pub fn menu_item(&self, index: usize) -> Option<&Vec<u16>> {
        self.menu_items.get(index)
    }

    /// Возвращает очередь диалогов для внешнего показа.
    pub fn dialog_queue(&self) -> DialogQueue {
        self.pending_dialog.clone()
    }

    /// Возвращает очередь уведомлений для внешнего показа.
    pub fn note_queue(&self) -> NoteQueue {
        self.pending_notes.clone()
    }

    /// Забирает накопленные уведомления.
    pub fn take_notes(&mut self) -> Vec<NoteData> {
        let mut q = self.pending_notes.borrow_mut();
        std::mem::take(&mut *q)
    }

    /// Забирает отложенный запрос диалога.
    pub fn take_pending_dialog(&mut self) -> Option<DialogData> {
        self.pending_dialog.borrow_mut().take()
    }

    /// Устанавливает колбэк активного диалога.
    pub fn set_dialog_cb(&mut self, cb: Box<dyn FnMut(&mut Tree, usize)>) {
        self.on_dialog = Some(cb);
    }

    /// Вызывает колбэк диалога с выбранной кнопкой.
    pub fn fire_dialog(&mut self, index: usize) {
        if let Some(mut cb) = self.on_dialog.take() {
            cb(self, index);
            self.on_dialog = Some(cb);
        }
    }

    /// Регистрирует горячую клавишу окна: маска модификаторов и код.
    pub fn add_hotkey<F: FnMut(&mut Tree) + 'static>(&mut self, mods: u8, vk: u32, f: F) {
        self.hotkeys.push((mods, vk, Box::new(f)));
    }

    /// Вызывает первый подходящий хоткей; true — если сработал.
    pub fn fire_hotkey(&mut self, mods: u8, vk: u32) -> bool {
        let vk = match vk {
            0x6B => 0xBB,
            0x6D => 0xBD,
            _ => vk,
        };
        let mut list = std::mem::take(&mut self.hotkeys);
        let mut hit = false;
        for (m, k, cb) in list.iter_mut() {
            if *m == mods && *k == vk {
                cb(self);
                hit = true;
                break;
            }
        }
        list.append(&mut self.hotkeys);
        self.hotkeys = list;
        hit
    }

    /// Реагирует ли узел на клик.
    pub fn is_interactive(&self, id: NodeId) -> bool {
        self.is_button(id)
            || self.is_checkbox(id)
            || self.is_switch(id)
            || self.is_radio(id)
            || self.is_toggle(id)
            || self.is_link(id)
    }

    /// Является ли узел ссылкой.
    pub fn is_link(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Link { .. })
    }

    /// Является ли узел секцией аккордеона.
    pub fn is_accordion(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Accordion { .. })
    }

    /// Раскрыта ли секция аккордеона.
    pub fn acc_open(&self, id: NodeId) -> bool {
        matches!(&self.nodes[id.0].kind, NodeKind::Accordion { open, .. } if *open)
    }

    /// Переключает раскрытие секции аккордеона.
    pub fn toggle_acc(&mut self, id: NodeId) {
        if let NodeKind::Accordion { open, .. } = &mut self.nodes[id.0].kind {
            *open = !*open;
        }
        self.dirty = true;
    }

    /// Является ли узел областью прокрутки.
    pub fn is_scroll(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Scroll { .. })
    }

    /// Является ли узел стопкой страниц.
    pub fn is_stack(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Stack { .. })
    }

    /// Возвращает активную страницу стопки.
    pub fn stack_page(&self, id: NodeId) -> usize {
        if let NodeKind::Stack { page } = &self.nodes[id.0].kind {
            *page
        } else {
            0
        }
    }

    /// Задаёт активную страницу стопки.
    pub fn set_stack_page(&mut self, id: NodeId, index: usize) {
        if let NodeKind::Stack { page } = &mut self.nodes[id.0].kind {
            *page = index;
        }
        self.dirty = true;
    }

    /// Является ли узел разделителем областей.
    pub fn is_splitter(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Splitter { .. })
    }

    /// Вертикален ли разделитель (делит по горизонтали).
    pub fn split_vertical(&self, id: NodeId) -> bool {
        matches!(&self.nodes[id.0].kind, NodeKind::Splitter { vertical, .. } if *vertical)
    }

    /// Возвращает долю первой области разделителя.
    pub fn split_ratio(&self, id: NodeId) -> f32 {
        if let NodeKind::Splitter { ratio, .. } = &self.nodes[id.0].kind {
            *ratio
        } else {
            0.5
        }
    }

    /// Задаёт долю первой области разделителя.
    pub fn set_split_ratio(&mut self, id: NodeId, value: f32) {
        if let NodeKind::Splitter { ratio, .. } = &mut self.nodes[id.0].kind {
            *ratio = value.clamp(0.1, 0.9);
        }
        self.dirty = true;
    }

    /// Возвращает смещение прокрутки области.
    pub fn scroll_offset(&self, id: NodeId) -> f32 {
        if let NodeKind::Scroll { offset, .. } = &self.nodes[id.0].kind {
            *offset
        } else {
            0.0
        }
    }

    /// Задаёт смещение прокрутки области.
    pub fn set_scroll_offset(&mut self, id: NodeId, value: f32) {
        if let NodeKind::Scroll { offset, .. } = &mut self.nodes[id.0].kind {
            *offset = value;
        }
        self.dirty = true;
    }

    /// Возвращает высоту содержимого области прокрутки.
    pub fn scroll_content(&self, id: NodeId) -> f32 {
        if let NodeKind::Scroll { content, .. } = &self.nodes[id.0].kind {
            *content
        } else {
            0.0
        }
    }

    /// Является ли узел переключателем.
    pub fn is_switch(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Switch { .. })
    }

    /// Является ли узел радиокнопкой.
    pub fn is_radio(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Radio { .. })
    }

    /// Инвертирует состояние переключателя.
    pub fn toggle_switch(&mut self, id: NodeId) {
        if let NodeKind::Switch { on, .. } = &mut self.nodes[id.0].kind {
            *on = !*on;
        }
    }

    /// Возвращает состояние переключателя.
    pub fn switch_on(&self, id: NodeId) -> bool {
        matches!(&self.nodes[id.0].kind, NodeKind::Switch { on, .. } if *on)
    }

    /// Выбирает радиокнопку, снимая выбор с её группы.
    pub fn select_radio(&mut self, id: NodeId) {
        let group = match &self.nodes[id.0].kind {
            NodeKind::Radio { group, .. } => *group,
            _ => return,
        };
        for n in self.nodes.iter_mut() {
            if let NodeKind::Radio { on, group: g, .. } = &mut n.kind {
                if *g == group {
                    *on = false;
                }
            }
        }
        if let NodeKind::Radio { on, .. } = &mut self.nodes[id.0].kind {
            *on = true;
        }
    }

    /// Возвращает состояние радиокнопки.
    pub fn radio_on(&self, id: NodeId) -> bool {
        matches!(&self.nodes[id.0].kind, NodeKind::Radio { on, .. } if *on)
    }

    /// Является ли узел кнопкой-переключателем.
    pub fn is_toggle(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Toggle { .. })
    }

    /// Инвертирует состояние кнопки-переключателя.
    pub fn flip_toggle(&mut self, id: NodeId) {
        if let NodeKind::Toggle { on, .. } = &mut self.nodes[id.0].kind {
            *on = !*on;
        }
    }

    /// Возвращает состояние кнопки-переключателя.
    pub fn toggle_on(&self, id: NodeId) -> bool {
        matches!(&self.nodes[id.0].kind, NodeKind::Toggle { on, .. } if *on)
    }

    /// Является ли узел ползунком.
    pub fn is_slider(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Slider { .. })
    }

    /// Является ли узел диапазонным ползунком.
    pub fn is_range(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Range { .. })
    }

    /// Возвращает границы диапазона `(lo, hi)`.
    pub fn range_values(&self, id: NodeId) -> (f32, f32) {
        if let NodeKind::Range { lo, hi } = &self.nodes[id.0].kind {
            (*lo, *hi)
        } else {
            (0.0, 0.0)
        }
    }

    /// Задаёт границы диапазона (0..1).
    pub fn set_range(&mut self, id: NodeId, a: f32, b: f32) {
        if let NodeKind::Range { lo, hi } = &mut self.nodes[id.0].kind {
            let a = a.clamp(0.0, 1.0);
            let b = b.clamp(0.0, 1.0);
            *lo = a.min(b);
            *hi = a.max(b);
        }
    }

    /// Является ли узел кнопкой с меню.
    pub fn is_split(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Split { .. })
    }

    /// Пункты меню кнопки с меню.
    pub fn split_options(&self, id: NodeId) -> Vec<Vec<u16>> {
        if let NodeKind::Split { options, .. } = &self.nodes[id.0].kind {
            options.clone()
        } else {
            Vec::new()
        }
    }

    /// Является ли узел строкой меню.
    pub fn is_menubar(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::MenuBar { .. })
    }

    /// Является ли узел круговым регулятором.
    pub fn is_dial(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Dial { .. })
    }

    /// Возвращает значение регулятора (0..1).
    pub fn dial_value(&self, id: NodeId) -> f32 {
        if let NodeKind::Dial { value, .. } = &self.nodes[id.0].kind {
            *value
        } else {
            0.0
        }
    }

    /// Задаёт значение регулятора (0..1).
    pub fn set_dial_value(&mut self, id: NodeId, v: f32) {
        if let NodeKind::Dial { value, .. } = &mut self.nodes[id.0].kind {
            *value = v.clamp(0.0, 1.0);
        }
    }

    /// Является ли узел деревом.
    pub fn is_tree(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::TreeView { .. })
    }

    /// Индексы видимых строк дерева.
    pub fn tree_visible(&self, id: NodeId) -> Vec<usize> {
        let items = match &self.nodes[id.0].kind {
            NodeKind::TreeView { items, .. } => items,
            _ => return Vec::new(),
        };
        let mut out = Vec::new();
        let mut skip: Option<usize> = None;
        for (i, it) in items.iter().enumerate() {
            if let Some(d) = skip {
                if it.depth > d {
                    continue;
                }
                skip = None;
            }
            out.push(i);
            if !it.leaf && !it.open {
                skip = Some(it.depth);
            }
        }
        out
    }

    /// Данные строки дерева: глубина, метка, раскрыт, лист.
    pub fn tree_item(&self, id: NodeId, index: usize) -> Option<(usize, Vec<u16>, bool, bool)> {
        if let NodeKind::TreeView { items, .. } = &self.nodes[id.0].kind {
            items
                .get(index)
                .map(|it| (it.depth, it.label.clone(), it.open, it.leaf))
        } else {
            None
        }
    }

    /// Заменяет строки дерева, сохраняя выбор в пределах длины.
    pub fn set_tree_items(&mut self, id: NodeId, data: Vec<TreeItem>) {
        if let NodeKind::TreeView {
            items,
            selected,
            multi,
            ..
        } = &mut self.nodes[id.0].kind
        {
            if let Some(s) = selected {
                if *s >= data.len() {
                    *selected = None;
                }
            }
            multi.retain(|i| *i < data.len());
            *items = data;
            self.dirty = true;
        }
    }

    /// Число строк дерева.
    pub fn tree_len(&self, id: NodeId) -> usize {
        if let NodeKind::TreeView { items, .. } = &self.nodes[id.0].kind {
            items.len()
        } else {
            0
        }
    }

    /// Число колонок дерева; 0 — заголовка нет.
    pub fn tree_cols(&self, id: NodeId) -> usize {
        if let NodeKind::TreeView { cols, .. } = &self.nodes[id.0].kind {
            cols.len()
        } else {
            0
        }
    }

    /// Высота заголовка колонок конкретного дерева.
    pub fn tree_head(&self, id: NodeId) -> f32 {
        if self.tree_cols(id) == 0 {
            0.0
        } else {
            TREE_HEADER
        }
    }

    /// Границы колонок дерева в оконных координатах.
    pub fn tree_bounds(&self, id: NodeId) -> Vec<(f32, f32)> {
        if let NodeKind::TreeView { cols, widths, .. } = &self.nodes[id.0].kind {
            column_bounds(self.nodes[id.0].rect, cols.len().max(1), widths)
        } else {
            Vec::new()
        }
    }

    /// Возвращает очередь заявок дереву.
    pub fn tree_queue(&self) -> TreeQueue {
        self.pending_tree.clone()
    }

    /// Применяет накопленные заявки дереву; true — что-то изменилось.
    pub fn apply_tree_queue(&mut self) -> bool {
        let list = std::mem::take(&mut *self.pending_tree.borrow_mut());
        if list.is_empty() {
            return false;
        }
        for (id, op, arg) in list {
            if id.0 >= self.nodes.len() {
                continue;
            }
            match op {
                0 => {
                    let first = arg.first().copied();
                    self.set_tree_multi(id, arg);
                    self.set_tree_selected(id, first);
                }
                1 => {
                    if let Some(i) = arg.first() {
                        self.open_ancestors(id, *i);
                        self.tree_reveal(id, *i);
                    }
                }
                4 => {
                    if let Some(i) = arg.first() {
                        self.set_table_selected(id, Some(*i));
                        self.reveal_table(id, *i);
                    }
                }
                5 => {
                    let first = arg.first().copied();
                    self.set_list_multi(id, arg);
                    self.set_list_selected(id, first);
                    if let Some(i) = first {
                        self.reveal_list(id, i);
                    }
                }
                _ => {
                    let on = op == 2;
                    if arg.is_empty() {
                        let n = self.tree_len(id);
                        for i in 0..n {
                            self.set_tree_open(id, i, on);
                        }
                    } else {
                        for i in arg {
                            self.open_subtree(id, i, on);
                        }
                    }
                    self.clamp_tree_scroll(id);
                }
            }
        }
        self.dirty = true;
        true
    }

    /// Открытый слой поверх окна, если он есть.
    pub fn popup_layer(&self) -> Option<NodeId> {
        self.popup_layer
    }

    /// Показывает слой в заданном прямоугольнике окна.
    pub fn show_popup(&mut self, id: NodeId, x: f32, y: f32, w: f32, h: f32) {
        let (pw, ph) = (
            if w > 0.0 { Some(w) } else { None },
            if h > 0.0 { Some(h) } else { None },
        );
        self.set_place(id, Some(x), Some(y), None, None, pw, ph);
        self.popup_layer = Some(id);
        self.dirty = true;
    }

    /// Прячет слой; если он не открыт, вызов безвреден.
    pub fn hide_popup(&mut self, id: NodeId) {
        self.set_place(id, Some(OFF_COORD), Some(OFF_COORD), None, None, None, None);
        if self.popup_layer == Some(id) {
            self.popup_layer = None;
        }
        self.dirty = true;
    }

    /// Закрывает открытый слой и возвращает его идентификатор.
    pub fn close_popup(&mut self) -> Option<NodeId> {
        let id = self.popup_layer.take()?;
        self.set_place(id, Some(OFF_COORD), Some(OFF_COORD), None, None, None, None);
        self.dirty = true;
        Some(id)
    }

    /// Возвращает таблицу геометрии деревьев.
    pub fn tree_geom(&self) -> TreeGeom {
        self.geom_tree.clone()
    }

    /// Обновляет таблицу геометрии по текущей раскладке.
    pub fn sync_tree_geom(&self) {
        let mut map = self.geom_tree.borrow_mut();
        map.clear();
        for (i, n) in self.nodes.iter().enumerate() {
            if !matches!(n.kind, NodeKind::TreeView { .. }) {
                continue;
            }
            let id = NodeId(i);
            map.insert(
                i,
                (
                    n.rect,
                    self.tree_head(id),
                    self.tree_scroll(id),
                    self.tree_bounds(id),
                    self.tree_visible(id),
                ),
            );
        }
    }

    fn set_tree_open(&mut self, id: NodeId, index: usize, on: bool) {
        if let NodeKind::TreeView { items, .. } = &mut self.nodes[id.0].kind {
            if let Some(it) = items.get_mut(index) {
                if !it.leaf {
                    it.open = on;
                }
            }
        }
    }

    fn item_depth(&self, id: NodeId, index: usize) -> Option<usize> {
        if let NodeKind::TreeView { items, .. } = &self.nodes[id.0].kind {
            items.get(index).map(|it| it.depth)
        } else {
            None
        }
    }

    fn open_subtree(&mut self, id: NodeId, index: usize, on: bool) {
        let Some(base) = self.item_depth(id, index) else {
            return;
        };
        self.set_tree_open(id, index, on);
        let n = self.tree_len(id);
        for i in index + 1..n {
            match self.item_depth(id, i) {
                Some(d) if d > base => self.set_tree_open(id, i, on),
                _ => break,
            }
        }
    }

    fn open_ancestors(&mut self, id: NodeId, index: usize) {
        let Some(mut want) = self.item_depth(id, index) else {
            return;
        };
        let mut i = index;
        while i > 0 && want > 0 {
            i -= 1;
            if let Some(d) = self.item_depth(id, i) {
                if d < want {
                    self.set_tree_open(id, i, true);
                    want = d;
                }
            }
        }
    }

    fn tree_reveal(&mut self, id: NodeId, index: usize) {
        let vis = self.tree_visible(id);
        let Some(pos) = vis.iter().position(|v| *v == index) else {
            return;
        };
        let rect = self.nodes[id.0].rect;
        let view = (rect.height - self.tree_head(id)).max(0.0);
        let y = pos as f32 * LIST_ROW;
        let mut s = self.tree_scroll(id);
        if y < s {
            s = y;
        }
        if y + LIST_ROW > s + view {
            s = y + LIST_ROW - view;
        }
        let max = (vis.len() as f32 * LIST_ROW - view).max(0.0);
        self.set_tree_scroll(id, s.clamp(0.0, max));
    }

    /// Включён ли множественный выбор в дереве.
    pub fn is_tree_msel(&self, id: NodeId) -> bool {
        matches!(&self.nodes[id.0].kind, NodeKind::TreeView { msel, .. } if *msel)
    }

    /// Возвращает множественное выделение дерева.
    pub fn tree_multi(&self, id: NodeId) -> Vec<usize> {
        if let NodeKind::TreeView { multi, .. } = &self.nodes[id.0].kind {
            multi.clone()
        } else {
            Vec::new()
        }
    }

    /// Задаёт множественное выделение дерева.
    pub fn set_tree_multi(&mut self, id: NodeId, mut data: Vec<usize>) {
        let n = self.tree_len(id);
        data.retain(|i| *i < n);
        data.sort_unstable();
        data.dedup();
        if let NodeKind::TreeView { multi, .. } = &mut self.nodes[id.0].kind {
            *multi = data;
        }
        self.dirty = true;
    }

    /// Соседняя видимая строка: `step` −1 вверх, +1 вниз.
    pub fn tree_step(&self, id: NodeId, step: i32) -> Option<usize> {
        let vis = self.tree_visible(id);
        if vis.is_empty() {
            return None;
        }
        let cur = self
            .tree_selected(id)
            .and_then(|s| vis.iter().position(|v| *v == s));
        let next = match cur {
            Some(p) => {
                let n = p as i32 + step;
                n.clamp(0, vis.len() as i32 - 1) as usize
            }
            None => {
                if step < 0 {
                    vis.len() - 1
                } else {
                    0
                }
            }
        };
        Some(vis[next])
    }

    /// Первая или последняя видимая строка.
    pub fn tree_edge(&self, id: NodeId, last: bool) -> Option<usize> {
        let vis = self.tree_visible(id);
        if last {
            vis.last().copied()
        } else {
            vis.first().copied()
        }
    }

    /// Родитель строки по глубине; `None` у корневых.
    pub fn tree_parent(&self, id: NodeId, index: usize) -> Option<usize> {
        let want = self.item_depth(id, index)?;
        if want == 0 {
            return None;
        }
        let mut i = index;
        while i > 0 {
            i -= 1;
            if let Some(d) = self.item_depth(id, i) {
                if d < want {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Раскрыта ли строка; для листьев всегда false.
    pub fn tree_open(&self, id: NodeId, index: usize) -> bool {
        if let NodeKind::TreeView { items, .. } = &self.nodes[id.0].kind {
            items.get(index).map(|it| !it.leaf && it.open).unwrap_or(false)
        } else {
            false
        }
    }

    /// Лист ли строка.
    pub fn tree_leaf(&self, id: NodeId, index: usize) -> bool {
        if let NodeKind::TreeView { items, .. } = &self.nodes[id.0].kind {
            items.get(index).map(|it| it.leaf).unwrap_or(true)
        } else {
            true
        }
    }

    /// Раскрывает или сворачивает одну строку.
    pub fn open_row(&mut self, id: NodeId, index: usize, on: bool) {
        self.set_tree_open(id, index, on);
        self.clamp_tree_scroll(id);
        self.dirty = true;
    }

    /// Возвращает прокрутку дерева в допустимые пределы.
    pub fn clamp_tree_scroll(&mut self, id: NodeId) {
        let rect = self.nodes[id.0].rect;
        let view = (rect.height - self.tree_head(id)).max(0.0);
        let content = self.tree_visible(id).len() as f32 * LIST_ROW;
        let max = (content - view).max(0.0);
        let cur = self.tree_scroll(id);
        if cur > max {
            self.set_tree_scroll(id, max);
            self.dirty = true;
        }
    }

    /// Прокручивает дерево так, чтобы строка была видна.
    pub fn reveal_row(&mut self, id: NodeId, index: usize) {
        self.tree_reveal(id, index);
    }

    /// Индекс колонки под точкой; 0 при отсутствии колонок.
    pub fn tree_col_at(&self, id: NodeId, x: f32) -> usize {
        let b = self.tree_bounds(id);
        for (i, (cx, cw)) in b.iter().enumerate() {
            if x >= *cx && x < *cx + *cw {
                return i;
            }
        }
        b.len().saturating_sub(1)
    }

    /// Пути иконок строк дерева для предзагрузки.
    pub fn tree_icons(&self, out: &mut Vec<String>) {
        for n in self.nodes.iter() {
            if let NodeKind::TreeView { items, .. } = &n.kind {
                for it in items.iter() {
                    if let Some(p) = &it.icon {
                        if !p.is_empty() {
                            out.push(p.clone());
                        }
                    }
                }
            }
        }
    }

    /// Переключает раскрытие строки дерева.
    pub fn toggle_tree(&mut self, id: NodeId, index: usize) {
        if let NodeKind::TreeView { items, .. } = &mut self.nodes[id.0].kind {
            if let Some(it) = items.get_mut(index) {
                it.open = !it.open;
            }
        }
    }

    /// Возвращает выбранную строку дерева.
    pub fn tree_selected(&self, id: NodeId) -> Option<usize> {
        if let NodeKind::TreeView { selected, .. } = &self.nodes[id.0].kind {
            *selected
        } else {
            None
        }
    }

    /// Задаёт выбранную строку дерева.
    pub fn set_tree_selected(&mut self, id: NodeId, index: Option<usize>) {
        if let NodeKind::TreeView { selected, .. } = &mut self.nodes[id.0].kind {
            *selected = index;
        }
    }

    /// Возвращает прокрутку дерева в пикселях.
    pub fn tree_scroll(&self, id: NodeId) -> f32 {
        if let NodeKind::TreeView { scroll, .. } = &self.nodes[id.0].kind {
            *scroll
        } else {
            0.0
        }
    }

    /// Задаёт прокрутку дерева в пикселях.
    pub fn set_tree_scroll(&mut self, id: NodeId, value: f32) {
        if let NodeKind::TreeView { scroll, .. } = &mut self.nodes[id.0].kind {
            *scroll = value;
        }
    }

    /// Является ли узел календарём.
    pub fn is_calendar(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Calendar { .. })
    }

    /// Возвращает дату календаря `(год, месяц, день)`.
    pub fn cal_ymd(&self, id: NodeId) -> (i32, u32, u32) {
        if let NodeKind::Calendar { year, month, day } = &self.nodes[id.0].kind {
            (*year, *month, *day)
        } else {
            (2000, 1, 1)
        }
    }

    /// Задаёт день календаря.
    pub fn set_cal_day(&mut self, id: NodeId, d: u32) {
        if let NodeKind::Calendar { day, .. } = &mut self.nodes[id.0].kind {
            *day = d;
        }
    }

    /// Сдвигает месяц календаря на `delta`.
    pub fn cal_shift(&mut self, id: NodeId, delta: i32) {
        if let NodeKind::Calendar { year, month, day } = &mut self.nodes[id.0].kind {
            let m = *month as i32 - 1 + delta;
            *year += m.div_euclid(12);
            *month = m.rem_euclid(12) as u32 + 1;
            *day = 1;
        }
    }

    /// Код даты для колбэка: `(год-2000)*10000 + месяц*100 + день`.
    pub fn cal_code(&self, id: NodeId) -> f32 {
        let (y, m, d) = self.cal_ymd(id);
        ((y - 2000) * 10000 + m as i32 * 100 + d as i32) as f32
    }

    /// Является ли узел палитрой цвета.
    pub fn is_color(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Color { .. })
    }

    /// Возвращает цвет палитры в HSV.
    pub fn color_hsv(&self, id: NodeId) -> (f32, f32, f32) {
        if let NodeKind::Color { hue, sat, val } = &self.nodes[id.0].kind {
            (*hue, *sat, *val)
        } else {
            (0.0, 0.0, 0.0)
        }
    }

    /// Задаёт цвет палитры в HSV (0..1).
    pub fn set_color_hsv(&mut self, id: NodeId, h: f32, s: f32, v: f32) {
        if let NodeKind::Color { hue, sat, val } = &mut self.nodes[id.0].kind {
            *hue = h.clamp(0.0, 1.0);
            *sat = s.clamp(0.0, 1.0);
            *val = v.clamp(0.0, 1.0);
        }
    }

    /// Является ли узел выбором времени.
    pub fn is_time(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Time { .. })
    }

    /// Возвращает время `(часы, минуты)`.
    pub fn time_hm(&self, id: NodeId) -> (u32, u32) {
        if let NodeKind::Time { hour, minute } = &self.nodes[id.0].kind {
            (*hour, *minute)
        } else {
            (0, 0)
        }
    }

    /// Сдвигает часы или минуты на `delta`.
    pub fn time_shift(&mut self, id: NodeId, hours: i32, minutes: i32) {
        if let NodeKind::Time { hour, minute } = &mut self.nodes[id.0].kind {
            let h = (*hour as i32 + hours).rem_euclid(24);
            let m = (*minute as i32 + minutes).rem_euclid(60);
            *hour = h as u32;
            *minute = m as u32;
        }
    }

    /// Код времени для колбэка: `часы * 100 + минуты`.
    pub fn time_code(&self, id: NodeId) -> f32 {
        let (h, m) = self.time_hm(id);
        (h * 100 + m) as f32
    }

    /// Является ли узел таблицей свойств.
    pub fn is_propgrid(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::PropGrid { .. })
    }

    /// Число строк таблицы свойств.
    pub fn prop_len(&self, id: NodeId) -> usize {
        if let NodeKind::PropGrid { rows, .. } = &self.nodes[id.0].kind {
            rows.len()
        } else {
            0
        }
    }

    /// Задаёт строки таблицы свойств.
    pub fn set_prop_rows(&mut self, id: NodeId, data: Vec<(Vec<u16>, Vec<u16>)>) {
        if let NodeKind::PropGrid { rows, .. } = &mut self.nodes[id.0].kind {
            *rows = data;
        }
    }

    /// Возвращает выбранную строку таблицы свойств.
    pub fn prop_selected(&self, id: NodeId) -> Option<usize> {
        if let NodeKind::PropGrid { selected, .. } = &self.nodes[id.0].kind {
            *selected
        } else {
            None
        }
    }

    /// Задаёт выбранную строку таблицы свойств.
    pub fn set_prop_selected(&mut self, id: NodeId, index: Option<usize>) {
        if let NodeKind::PropGrid { selected, .. } = &mut self.nodes[id.0].kind {
            *selected = index;
        }
    }

    /// Возвращает прокрутку таблицы свойств.
    pub fn prop_scroll(&self, id: NodeId) -> f32 {
        if let NodeKind::PropGrid { scroll, .. } = &self.nodes[id.0].kind {
            *scroll
        } else {
            0.0
        }
    }

    /// Задаёт прокрутку таблицы свойств.
    pub fn set_prop_scroll(&mut self, id: NodeId, value: f32) {
        if let NodeKind::PropGrid { scroll, .. } = &mut self.nodes[id.0].kind {
            *scroll = value;
        }
    }

    /// Число разделов строки меню.
    pub fn bar_len(&self, id: NodeId) -> usize {
        if let NodeKind::MenuBar { titles, .. } = &self.nodes[id.0].kind {
            titles.len()
        } else {
            0
        }
    }

    /// Пункты раздела строки меню.
    pub fn bar_items(&self, id: NodeId, index: usize) -> Vec<Vec<u16>> {
        if let NodeKind::MenuBar { items, .. } = &self.nodes[id.0].kind {
            items.get(index).cloned().unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Может ли узел получать фокус клавиатуры.
    pub fn is_focusable(&self, id: NodeId) -> bool {
        self.is_button(id)
            || self.is_checkbox(id)
            || self.is_switch(id)
            || self.is_radio(id)
            || self.is_toggle(id)
            || self.is_textbox(id)
            || self.is_slider(id)
            || self.is_dropdown(id)
            || self.is_tabs(id)
            || self.is_table(id)
            || self.is_list(id)
            || self.is_tree(id)
    }

    /// Является ли узел списком.
    pub fn is_list(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::List { .. })
    }

    /// Число пунктов списка.
    pub fn list_len(&self, id: NodeId) -> usize {
        if let NodeKind::List { items, .. } = &self.nodes[id.0].kind {
            items.len()
        } else {
            0
        }
    }

    /// Возвращает выбранный пункт списка.
    pub fn list_selected(&self, id: NodeId) -> Option<usize> {
        if let NodeKind::List { selected, .. } = &self.nodes[id.0].kind {
            *selected
        } else {
            None
        }
    }

    /// Задаёт выбранный пункт списка.
    pub fn set_list_selected(&mut self, id: NodeId, index: Option<usize>) {
        if let NodeKind::List { selected, .. } = &mut self.nodes[id.0].kind {
            *selected = index;
        }
    }

    /// Заменяет пункты списка, сохраняя выбор в пределах длины.
    pub fn set_list_items(&mut self, id: NodeId, data: Vec<Vec<u16>>) {
        if let NodeKind::List {
            items, selected, multi, ..
        } = &mut self.nodes[id.0].kind
        {
            if let Some(s) = selected {
                if *s >= data.len() {
                    *selected = None;
                }
            }
            multi.retain(|i| *i < data.len());
            *items = data;
            self.dirty = true;
        }
    }

    /// Возвращает прокрутку списка в пикселях.
    pub fn list_scroll(&self, id: NodeId) -> f32 {
        if let NodeKind::List { scroll, .. } = &self.nodes[id.0].kind {
            *scroll
        } else {
            0.0
        }
    }

    /// Задаёт прокрутку списка в пикселях.
    pub fn set_list_scroll(&mut self, id: NodeId, value: f32) {
        if let NodeKind::List { scroll, .. } = &mut self.nodes[id.0].kind {
            *scroll = value;
        }
    }

    /// Включён ли множественный выбор списка.
    pub fn is_list_msel(&self, id: NodeId) -> bool {
        matches!(&self.nodes[id.0].kind, NodeKind::List { msel, .. } if *msel)
    }

    /// Возвращает множественное выделение списка.
    pub fn list_multi(&self, id: NodeId) -> Vec<usize> {
        if let NodeKind::List { multi, .. } = &self.nodes[id.0].kind {
            multi.clone()
        } else {
            Vec::new()
        }
    }

    /// Задаёт множественное выделение списка.
    pub fn set_list_multi(&mut self, id: NodeId, mut data: Vec<usize>) {
        let n = self.list_len(id);
        data.retain(|i| *i < n);
        data.sort_unstable();
        data.dedup();
        if let NodeKind::List { multi, .. } = &mut self.nodes[id.0].kind {
            *multi = data;
        }
        self.dirty = true;
    }

    /// Прокручивает список так, чтобы строка была видна.
    pub fn reveal_list(&mut self, id: NodeId, index: usize) {
        let r = self.nodes[id.0].rect;
        let view = r.height.max(0.0);
        let n = self.list_len(id);
        let content = n as f32 * LIST_ROW;
        let max = (content - view).max(0.0);
        let top = index as f32 * LIST_ROW;
        let mut scroll = self.list_scroll(id);
        if top < scroll {
            scroll = top;
        } else if top + LIST_ROW > scroll + view {
            scroll = top + LIST_ROW - view;
        }
        self.set_list_scroll(id, scroll.clamp(0.0, max));
    }

    /// Список фокусируемых узлов в порядке обхода дерева.
    pub fn focusables(&self) -> Vec<NodeId> {
        let mut out = Vec::new();
        self.collect_focus(self.root, &mut out);
        out
    }

    fn collect_focus(&self, id: NodeId, out: &mut Vec<NodeId>) {
        if self.nodes[id.0].rect.x <= OFF_LIMIT {
            return;
        }
        if self.is_focusable(id) {
            out.push(id);
        }
        let count = self.nodes[id.0].children.len();
        for i in 0..count {
            let child = self.nodes[id.0].children[i];
            self.collect_focus(child, out);
        }
    }

    /// Задаёт значение ползунка в диапазоне 0..1.
    pub fn set_slider_value(&mut self, id: NodeId, value: f32) {
        if let NodeKind::Slider { value: v } = &mut self.nodes[id.0].kind {
            *v = value;
        }
    }

    /// Задаёт значение прогресс-бара в диапазоне 0..1.
    pub fn set_progress_value(&mut self, id: NodeId, value: f32) {
        if let NodeKind::Progress { value: v } = &mut self.nodes[id.0].kind {
            *v = value;
        }
    }

    /// Переключает состояние чекбокса.
    pub fn toggle_checkbox(&mut self, id: NodeId) {
        if let NodeKind::Checkbox { checked, .. } = &mut self.nodes[id.0].kind {
            *checked = !*checked;
        }
    }

    /// Возвращает состояние поля ввода.
    pub fn textbox_state(&self, id: NodeId) -> Option<&TextState> {
        if let NodeKind::TextBox { state } = &self.nodes[id.0].kind {
            Some(state)
        } else {
            None
        }
    }

    /// Возвращает изменяемое состояние поля ввода.
    pub fn textbox_state_mut(&mut self, id: NodeId) -> Option<&mut TextState> {
        if let NodeKind::TextBox { state } = &mut self.nodes[id.0].kind {
            Some(state)
        } else {
            None
        }
    }

    /// Многострочное ли поле ввода.
    pub fn is_multiline(&self, id: NodeId) -> bool {
        self.nodes[id.0].multiline
    }

    /// Делает поле ввода многострочным.
    pub fn set_multiline(&mut self, id: NodeId) {
        self.nodes[id.0].multiline = true;
    }

    /// Возвращает верхний узел, содержащий точку `(x, y)`.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<NodeId> {
        let mut hit = None;
        self.hit_walk(self.root, x, y, &mut hit);
        hit
    }

    fn hit_walk(&self, id: NodeId, x: f32, y: f32, hit: &mut Option<NodeId>) {
        if self.nodes[id.0].rect.x <= OFF_LIMIT {
            return;
        }
        if contains(self.nodes[id.0].rect, x, y) && !self.ghosts.contains(&id.0) {
            *hit = Some(id);
        }
        if self.layered(id) {
            for child in self.depth_order(id) {
                self.hit_walk(child, x, y, hit);
            }
        } else {
            let count = self.nodes[id.0].children.len();
            for i in 0..count {
                let child = self.nodes[id.0].children[i];
                self.hit_walk(child, x, y, hit);
            }
        }
    }

    /// Делает узел прозрачным для мыши (клики проходят насквозь).
    pub fn set_ghost(&mut self, id: NodeId, on: bool) {
        if on {
            self.ghosts.insert(id.0);
        } else {
            self.ghosts.remove(&id.0);
        }
    }

    /// Поднимать ли узел поверх соседей при нажатии внутри него.
    pub fn set_front(&mut self, id: NodeId, on: bool) {
        if on {
            self.fronts.insert(id.0);
        } else {
            self.fronts.remove(&id.0);
        }
    }

    /// Поднимает верхний front-узел под точкой поверх соседей.
    pub fn raise_front(&mut self, x: f32, y: f32) -> bool {
        if self.fronts.is_empty() {
            return false;
        }
        let mut target: Option<NodeId> = None;
        let mut top_z = i32::MIN;
        for &i in self.fronts.iter() {
            let n = &self.nodes[i];
            if n.rect.x <= OFF_LIMIT || !contains(n.rect, x, y) {
                continue;
            }
            if target.is_none() || n.props.z >= top_z {
                top_z = n.props.z;
                target = Some(NodeId(i));
            }
        }
        let id = match target {
            Some(id) => id,
            None => return false,
        };
        let parent = match self.nodes[id.0].parent {
            Some(p) => p,
            None => return false,
        };
        let mut max_other = i32::MIN;
        for &c in &self.nodes[parent.0].children {
            if c != id {
                max_other = max_other.max(self.nodes[c.0].props.z);
            }
        }
        if self.nodes[id.0].props.z <= max_other {
            self.nodes[id.0].props.z = max_other.saturating_add(1);
            true
        } else {
            false
        }
    }

    /// Задаёт текст-подсказку пустого поля ввода.
    pub fn set_placeholder(&mut self, id: NodeId, text: Vec<u16>) {
        self.placeholders.insert(id.0, text);
    }

    /// Пустые поля с подсказкой: прямоугольник, текст, многострочность.
    pub fn empty_placeholders(&self) -> Vec<(Rect, &Vec<u16>, bool)> {
        self.placeholders
            .iter()
            .filter_map(|(i, ph)| {
                let n = &self.nodes[*i];
                if n.rect.x <= OFF_LIMIT {
                    return None;
                }
                if let NodeKind::TextBox { state } = &n.kind {
                    if state.text.is_empty() {
                        return Some((n.rect, ph, n.multiline));
                    }
                }
                None
            })
            .collect()
    }

    /// Помечает раскладку устаревшей — следующий кадр пересчитает геометрию.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Вычисляет прямоугольники узлов; пропускает расчёт, если ничего не менялось.
    pub fn layout(&mut self, root_rect: Rect) {
        if !self.dirty && self.last_root == Some(root_rect) {
            return;
        }
        self.layout_node(self.root, root_rect);
        self.last_root = Some(root_rect);
        self.dirty = false;
    }

    fn layout_node(&mut self, id: NodeId, rect: Rect) {
        self.nodes[id.0].rect = rect;
        let props = self.nodes[id.0].props;
        let children = self.nodes[id.0].children.clone();
        if children.is_empty() {
            return;
        }

        let tabs_sel = if let NodeKind::Tabs { selected, .. } = &self.nodes[id.0].kind {
            Some(*selected)
        } else {
            None
        };
        if let Some(selected) = tabs_sel {
            let content = Rect::new(
                rect.x,
                rect.y + TAB_HEADER,
                rect.width,
                (rect.height - TAB_HEADER).max(0.0),
            );
            let off = OFF_RECT;
            for (i, &c) in children.iter().enumerate() {
                let cr = if i == selected { content } else { off };
                self.layout_node(c, cr);
            }
            return;
        }

        let dock_closed =
            matches!(&self.nodes[id.0].kind, NodeKind::Dock { open, .. } if !*open);
        if dock_closed {
            let off = OFF_RECT;
            for &c in &children {
                self.layout_node(c, off);
            }
            return;
        }

        if let NodeKind::Stack { page } = &self.nodes[id.0].kind {
            let page = *page;
            let off = OFF_RECT;
            for (i, &c) in children.iter().enumerate() {
                let cr = if i == page { rect } else { off };
                self.layout_node(c, cr);
            }
            return;
        }

        if let NodeKind::Splitter { ratio, vertical } = &self.nodes[id.0].kind {
            let ratio = *ratio;
            let vertical = *vertical;
            let off = OFF_RECT;
            let (r1, r2) = if vertical {
                let w1 = (rect.width - SPLIT_W) * ratio;
                let w2 = (rect.width - SPLIT_W - w1).max(0.0);
                (
                    Rect::new(rect.x, rect.y, w1.max(0.0), rect.height),
                    Rect::new(rect.x + w1 + SPLIT_W, rect.y, w2, rect.height),
                )
            } else {
                let h1 = (rect.height - SPLIT_W) * ratio;
                let h2 = (rect.height - SPLIT_W - h1).max(0.0);
                (
                    Rect::new(rect.x, rect.y, rect.width, h1.max(0.0)),
                    Rect::new(rect.x, rect.y + h1 + SPLIT_W, rect.width, h2),
                )
            };
            for (i, &c) in children.iter().enumerate() {
                let cr = match i {
                    0 => r1,
                    1 => r2,
                    _ => off,
                };
                self.layout_node(c, cr);
            }
            return;
        }

        if let NodeKind::Accordion { open, .. } = &self.nodes[id.0].kind {
            let open = *open;
            let pad = props.padding;
            let body = Rect::new(
                rect.x + pad,
                rect.y + ACC_HEADER,
                (rect.width - 2.0 * pad).max(0.0),
                (rect.height - ACC_HEADER - pad).max(0.0),
            );
            let off = OFF_RECT;
            let mut cursor = body.y;
            for &c in &children {
                if !open {
                    self.layout_node(c, off);
                    continue;
                }
                let ch = self.nodes[c.0].props.height.unwrap_or(36.0);
                self.layout_node(c, Rect::new(body.x, cursor, body.width, ch));
                cursor += ch + props.gap;
            }
            return;
        }

        if self.is_scroll(id) {
            let pad = props.padding;
            let gap = props.gap;
            let offset = self.scroll_offset(id);
            let inner_w = (rect.width - 2.0 * pad - SCROLLBAR_W - 4.0).max(0.0);
            let mut cursor = rect.y + pad - offset;
            let mut total = pad;
            for &c in &children {
                let ch = self.nodes[c.0].props.height.unwrap_or(40.0);
                self.layout_node(c, Rect::new(rect.x + pad, cursor, inner_w, ch));
                cursor += ch + gap;
                total += ch + gap;
            }
            let total = (total + pad - gap).max(0.0);
            if let NodeKind::Scroll { content, .. } = &mut self.nodes[id.0].kind {
                *content = total;
            }
            return;
        }

        let is_group = matches!(self.nodes[id.0].kind, NodeKind::Group { .. });
        let is_dock = matches!(self.nodes[id.0].kind, NodeKind::Dock { .. });
        let head = if is_group {
            GROUP_HEADER
        } else if is_dock {
            DOCK_HEADER
        } else {
            0.0
        };
        let pad = props.padding;
        let inner = Rect::new(
            rect.x + pad,
            rect.y + pad + head,
            (rect.width - 2.0 * pad).max(0.0),
            (rect.height - 2.0 * pad - head).max(0.0),
        );
        for &c in &children {
            let cp = self.nodes[c.0].props;
            if !cp.abs {
                continue;
            }
            let (ax, aw) = anchor_axis(inner.x, inner.width, cp.l, cp.r, cp.width);
            let (ay, ah) = anchor_axis(inner.y, inner.height, cp.t, cp.b, cp.height);
            self.layout_node(c, Rect::new(ax, ay, aw, ah));
        }
        let flow: Vec<NodeId> = children
            .iter()
            .copied()
            .filter(|c| !self.nodes[c.0].props.abs)
            .collect();
        if props.mode == 1 {
            self.layout_grid(&flow, inner, props);
            return;
        }
        if props.mode == 2 {
            self.layout_pack(&flow, inner, props);
            return;
        }
        let n = flow.len();
        let total_gap = props.gap * n.saturating_sub(1) as f32;
        let vertical = matches!(props.axis, Axis::Vertical);
        let inner_main = if vertical { inner.height } else { inner.width };
        let inner_cross = if vertical { inner.width } else { inner.height };

        let mut fixed_sum = 0.0;
        let mut weight_sum = 0.0;
        for &c in &flow {
            let cp = self.nodes[c.0].props;
            let main = if vertical { cp.height } else { cp.width };
            match main {
                Some(v) => fixed_sum += v,
                None => weight_sum += if cp.grow > 0.0 { cp.grow } else { 1.0 },
            }
        }
        let flex_space = (inner_main - total_gap - fixed_sum).max(0.0);
        let unit = if weight_sum > 0.0 {
            flex_space / weight_sum
        } else {
            0.0
        };

        let used = fixed_sum + total_gap + if weight_sum > 0.0 { flex_space } else { 0.0 };
        let leftover = (inner_main - used).max(0.0);
        let (start_off, gap_extra) = match props.justify {
            1 => (leftover / 2.0, 0.0),
            2 => (leftover, 0.0),
            3 if n > 1 => (0.0, leftover / (n - 1) as f32),
            _ => (0.0, 0.0),
        };

        let mut cursor = (if vertical { inner.y } else { inner.x }) + start_off;
        for &c in &flow {
            let cp = self.nodes[c.0].props;
            let main_fixed = if vertical { cp.height } else { cp.width };
            let main_size = match main_fixed {
                Some(v) => v,
                None => unit * if cp.grow > 0.0 { cp.grow } else { 1.0 },
            };
            let cross_fixed = if vertical { cp.width } else { cp.height };
            let (cx_off, cx_size) = match cross_fixed {
                None => (0.0, inner_cross),
                Some(s) => {
                    let s = s.min(inner_cross);
                    let off = match props.cross {
                        2 => (inner_cross - s) / 2.0,
                        3 => inner_cross - s,
                        _ => 0.0,
                    };
                    (off, s)
                }
            };
            let child_rect = if vertical {
                Rect::new(inner.x + cx_off, cursor, cx_size, main_size)
            } else {
                Rect::new(cursor, inner.y + cx_off, main_size, cx_size)
            };
            cursor += main_size + props.gap + gap_extra;
            self.layout_node(c, child_rect);
        }
    }

    fn layout_grid(&mut self, flow: &[NodeId], inner: Rect, props: Props) {
        let mut cols = 1usize;
        let mut rows = 1usize;
        for &c in flow {
            let cp = self.nodes[c.0].props;
            cols = cols.max(cp.col as usize + cp.cspan.max(1) as usize);
            rows = rows.max(cp.row as usize + cp.rspan.max(1) as usize);
        }
        let gap = props.gap;
        let cw = ((inner.width - gap * (cols - 1) as f32) / cols as f32).max(0.0);
        let rh = ((inner.height - gap * (rows - 1) as f32) / rows as f32).max(0.0);
        for &c in flow {
            let cp = self.nodes[c.0].props;
            let cs = cp.cspan.max(1) as f32;
            let rs = cp.rspan.max(1) as f32;
            let x = inner.x + (cw + gap) * cp.col as f32;
            let y = inner.y + (rh + gap) * cp.row as f32;
            let w = (cw * cs + gap * (cs - 1.0)).max(0.0);
            let h = (rh * rs + gap * (rs - 1.0)).max(0.0);
            let w = cp.width.unwrap_or(w).min(w);
            let h = cp.height.unwrap_or(h).min(h);
            self.layout_node(c, Rect::new(x, y, w, h));
        }
    }

    fn layout_pack(&mut self, flow: &[NodeId], inner: Rect, props: Props) {
        let gap = props.gap;
        let mut need_v = 0.0f32;
        let mut need_h = 0.0f32;
        let mut exp_v = 0usize;
        let mut exp_h = 0usize;
        for &c in flow {
            let cp = self.nodes[c.0].props;
            if cp.side >= 2 {
                need_h += cp.width.unwrap_or(0.0) + gap;
                if cp.expand {
                    exp_h += 1;
                }
            } else {
                need_v += cp.height.unwrap_or(0.0) + gap;
                if cp.expand {
                    exp_v += 1;
                }
            }
        }
        let free_v = (inner.height - need_v).max(0.0);
        let free_h = (inner.width - need_h).max(0.0);
        let add_v = if exp_v > 0 {
            free_v / exp_v as f32
        } else {
            0.0
        };
        let add_h = if exp_h > 0 {
            free_h / exp_h as f32
        } else {
            0.0
        };
        let mut cav = inner;
        for &c in flow {
            let cp = self.nodes[c.0].props;
            let rect = match cp.side {
                1 => {
                    let mut h = cp.height.unwrap_or(0.0);
                    if cp.expand {
                        h += add_v;
                    }
                    let h = h.min(cav.height);
                    let out = Rect::new(cav.x, cav.y + cav.height - h, cav.width, h);
                    cav = Rect::new(cav.x, cav.y, cav.width, (cav.height - h - gap).max(0.0));
                    out
                }
                2 => {
                    let mut w = cp.width.unwrap_or(0.0);
                    if cp.expand {
                        w += add_h;
                    }
                    let w = w.min(cav.width);
                    let out = Rect::new(cav.x, cav.y, w, cav.height);
                    cav = Rect::new(
                        cav.x + w + gap,
                        cav.y,
                        (cav.width - w - gap).max(0.0),
                        cav.height,
                    );
                    out
                }
                3 => {
                    let mut w = cp.width.unwrap_or(0.0);
                    if cp.expand {
                        w += add_h;
                    }
                    let w = w.min(cav.width);
                    let out = Rect::new(cav.x + cav.width - w, cav.y, w, cav.height);
                    cav = Rect::new(cav.x, cav.y, (cav.width - w - gap).max(0.0), cav.height);
                    out
                }
                _ => {
                    let mut h = cp.height.unwrap_or(0.0);
                    if cp.expand {
                        h += add_v;
                    }
                    let h = h.min(cav.height);
                    let out = Rect::new(cav.x, cav.y, cav.width, h);
                    cav = Rect::new(
                        cav.x,
                        cav.y + h + gap,
                        cav.width,
                        (cav.height - h - gap).max(0.0),
                    );
                    out
                }
            };
            let rect = pack_fill(rect, cp);
            self.layout_node(c, rect);
        }
    }

    /// Обходит дерево в глубину, вызывая `visit(id, node)` для каждого узла.
    pub fn for_each<F: FnMut(NodeId, &Node)>(&self, mut visit: F) {
        self.walk(self.root, &mut visit);
    }

    fn walk<F: FnMut(NodeId, &Node)>(&self, id: NodeId, visit: &mut F) {
        visit(id, &self.nodes[id.0]);
        if self.layered(id) {
            for child in self.depth_order(id) {
                self.walk(child, visit);
            }
        } else {
            let count = self.nodes[id.0].children.len();
            for i in 0..count {
                let child = self.nodes[id.0].children[i];
                self.walk(child, visit);
            }
        }
    }

    fn layered(&self, id: NodeId) -> bool {
        self.nodes[id.0]
            .children
            .iter()
            .any(|c| self.nodes[c.0].props.z != 0)
    }

    fn depth_order(&self, id: NodeId) -> Vec<NodeId> {
        let mut order = self.nodes[id.0].children.clone();
        order.sort_by_key(|c| self.nodes[c.0].props.z);
        order
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

fn pack_fill(rect: Rect, cp: Props) -> Rect {
    let mut out = rect;
    let fill_x = cp.fill == 1 || cp.fill == 3;
    let fill_y = cp.fill == 2 || cp.fill == 3;
    if !fill_x {
        if let Some(w) = cp.width {
            let w = w.min(out.width);
            out = Rect::new(out.x + (out.width - w) / 2.0, out.y, w, out.height);
        }
    }
    if !fill_y {
        if let Some(h) = cp.height {
            let h = h.min(out.height);
            out = Rect::new(out.x, out.y + (out.height - h) / 2.0, out.width, h);
        }
    }
    out
}

fn anchor_axis(
    start: f32,
    size: f32,
    near: Option<f32>,
    far: Option<f32>,
    fixed: Option<f32>,
) -> (f32, f32) {
    match (near, far, fixed) {
        (Some(a), Some(z), _) => (start + a, (size - a - z).max(0.0)),
        (Some(a), None, Some(w)) => (start + a, w),
        (None, Some(z), Some(w)) => (start + size - z - w, w),
        (Some(a), None, None) => (start + a, (size - a).max(0.0)),
        (None, Some(z), None) => (start, (size - z).max(0.0)),
        (None, None, Some(w)) => (start + (size - w) / 2.0, w),
        (None, None, None) => (start, size),
    }
}

pub fn kind_tag(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Container => "container",
        NodeKind::Frame { .. } => "frame",
        NodeKind::Label { .. } => "label",
        NodeKind::Button { .. } => "button",
        NodeKind::Slider { .. } => "slider",
        NodeKind::Progress { .. } => "progress",
        NodeKind::Checkbox { .. } => "checkbox",
        NodeKind::TextBox { .. } => "textbox",
        NodeKind::Dropdown { .. } => "dropdown",
        NodeKind::Tabs { .. } => "tabs",
        NodeKind::Table { .. } => "table",
        NodeKind::Image { .. } => "image",
        NodeKind::Switch { .. } => "switch",
        NodeKind::Radio { .. } => "radio",
        NodeKind::Toggle { .. } => "toggle",
        NodeKind::Separator { .. } => "separator",
        NodeKind::List { .. } => "list",
        NodeKind::Group { .. } => "group",
        NodeKind::Link { .. } => "link",
        NodeKind::Accordion { .. } => "accordion",
        NodeKind::Scroll { .. } => "scroll",
        NodeKind::Stack { .. } => "stack",
        NodeKind::Splitter { .. } => "splitter",
        NodeKind::Spinner { .. } => "spinner",
        NodeKind::Gauge { .. } => "gauge",
        NodeKind::Meter { .. } => "meter",
        NodeKind::Chart { .. } => "chart",
        NodeKind::Range { .. } => "range",
        NodeKind::Status { .. } => "status",
        NodeKind::Split { .. } => "split",
        NodeKind::MenuBar { .. } => "menubar",
        NodeKind::Dial { .. } => "dial",
        NodeKind::TreeView { .. } => "treeview",
        NodeKind::Calendar { .. } => "calendar",
        NodeKind::Color { .. } => "color",
        NodeKind::Time { .. } => "time",
        NodeKind::PropGrid { .. } => "propgrid",
        NodeKind::Badge { .. } => "badge",
        NodeKind::Crumbs { .. } => "crumbs",
        NodeKind::Pager { .. } => "pager",
        NodeKind::Rating { .. } => "rating",
        NodeKind::Canvas { .. } => "canvas",
        NodeKind::Term { .. } => "term",
        NodeKind::Dock { .. } => "dock",
        NodeKind::Drop { .. } => "drop",
    }
}

fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut prev = '\0';
            for c2 in chars.by_ref() {
                if prev == '*' && c2 == '/' {
                    break;
                }
                prev = c2;
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn apply_style_decl(style: &mut Style, key: &str, value: &str) {
    match key {
        "background" | "fill" => {
            if let Some(c) = parse_color(value) {
                style.fill = Some(c);
            }
        }
        "color" => {
            if let Some(c) = parse_color(value) {
                style.text = Some(c);
            }
        }
        "radius" => {
            style.radius = parse_num(value);
        }
        "font-size" => {
            style.size = parse_num(value);
        }
        "font-family" | "font" => {
            let name = value.trim().trim_matches('"').trim_matches('\'').trim();
            if !name.is_empty() {
                style.font = Some(intern_font(name));
            }
        }
        "wrap" => {
            style.wrap = Some(value == "true" || value == "1" || value == "wrap");
        }
        "shadow" | "elevation" => {
            style.elev = parse_num(value);
        }
        "gradient" => {
            let parts: Vec<&str> = value.split_whitespace().collect();
            if parts.len() >= 2 {
                if let (Some(a), Some(b)) = (parse_color(parts[0]), parse_color(parts[1])) {
                    style.grad = Some((a, b));
                    style.grad_dir = match parts.get(2).copied() {
                        Some("h") => 1,
                        Some("d") => 2,
                        Some("du") => 3,
                        _ => 0,
                    };
                }
            }
        }
        _ => {}
    }
}

fn apply_decl(node: &mut Node, key: &str, value: &str) {
    apply_style_decl(&mut node.style, key, value);
    match key {
        "padding" => {
            if let Some(n) = parse_num(value) {
                node.props.padding = n;
            }
        }
        "gap" => {
            if let Some(n) = parse_num(value) {
                node.props.gap = n;
            }
        }
        "width" => {
            node.props.width = parse_num(value);
        }
        "height" => {
            node.props.height = parse_num(value);
        }
        _ => {}
    }
}

fn parse_color(value: &str) -> Option<Color> {
    crate::render::types::parse_hex(value)
}

fn parse_num(value: &str) -> Option<f32> {
    value.trim().trim_end_matches("px").trim().parse::<f32>().ok()
}

fn contains(rect: Rect, x: f32, y: f32) -> bool {
    x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
}

fn ease_value(kind: Ease, t: f32) -> f32 {
    match kind {
        Ease::Linear => t,
        Ease::In => t * t * t,
        Ease::Out => {
            let u = 1.0 - t;
            1.0 - u * u * u
        }
        Ease::InOut => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                let u = -2.0 * t + 2.0;
                1.0 - u * u * u / 2.0
            }
        }
    }
}
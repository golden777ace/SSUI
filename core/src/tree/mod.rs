use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::render::types::{Color, Rect};

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
}

#[derive(Clone)]
pub struct Shape {
    pub kind: u8,
    pub args: [f32; 6],
    pub color: u32,
    pub text: Vec<u16>,
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
    ghosts: HashSet<usize>,
    placeholders: HashMap<usize, Vec<u16>>,
    on_dialog: Option<Box<dyn FnMut(&mut Tree, usize)>>,
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
            ghosts: HashSet::new(),
            placeholders: HashMap::new(),
            on_dialog: None,
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

    /// Есть ли активные анимации.
    pub fn has_anims(&self) -> bool {
        !self.anims.is_empty() || !self.pending.borrow().is_empty()
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

    /// Задаёт высоту тени (elevation) элемента.
    pub fn set_elev(&mut self, id: NodeId, elev: f32) {
        self.nodes[id.0].style.elev = Some(elev);
    }

    /// Разбирает подмножество CSS и применяет стили ко всем узлам.
    pub fn apply_css(&mut self, css: &str) {
        let rules = parse_css(css);
        let count = self.nodes.len();
        for i in 0..count {
            for rule in &rules {
                if sel_matches(&self.nodes[i], &rule.sel) {
                    match rule.state {
                        State::Base => {
                            for (k, v) in &rule.decls {
                                apply_decl(&mut self.nodes[i], k, v);
                            }
                        }
                        State::Focus => {
                            for (k, v) in &rule.decls {
                                apply_style_decl(&mut self.nodes[i].style_focus, k, v);
                            }
                        }
                        State::Hover => {
                            for (k, v) in &rule.decls {
                                apply_style_decl(&mut self.nodes[i].style_hover, k, v);
                            }
                        }
                    }
                }
            }
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
        if let NodeKind::Canvas { shapes } = &mut self.nodes[id.0].kind {
            *shapes = data;
        }
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
        let count = self.nodes[id.0].children.len();
        for i in 0..count {
            let child = self.nodes[id.0].children[i];
            self.hit_walk(child, x, y, hit);
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

    /// Обходит дерево в глубину, вызывая `visit(id, node)` для каждого узла.
    pub fn for_each<F: FnMut(NodeId, &Node)>(&self, mut visit: F) {
        self.walk(self.root, &mut visit);
    }

    fn walk<F: FnMut(NodeId, &Node)>(&self, id: NodeId, visit: &mut F) {
        visit(id, &self.nodes[id.0]);
        let count = self.nodes[id.0].children.len();
        for i in 0..count {
            let child = self.nodes[id.0].children[i];
            self.walk(child, visit);
        }
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
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

enum Sel {
    Any,
    Type(String),
    Class(String),
}

#[derive(Clone, Copy)]
enum State {
    Base,
    Focus,
    Hover,
}

struct Rule {
    sel: Sel,
    state: State,
    decls: Vec<(String, String)>,
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

fn sel_matches(node: &Node, sel: &Sel) -> bool {
    match sel {
        Sel::Any => true,
        Sel::Type(t) => kind_tag(&node.kind) == t,
        Sel::Class(c) => node.class_name.as_deref() == Some(c.as_str()),
    }
}

fn parse_css(css: &str) -> Vec<Rule> {
    let mut out = Vec::new();
    let cleaned = strip_comments(css);
    let mut rest = cleaned.as_str();
    while let Some(open) = rest.find('{') {
        let selector = rest[..open].trim().to_string();
        let after = &rest[open + 1..];
        let close = match after.find('}') {
            Some(c) => c,
            None => break,
        };
        let body = &after[..close];
        rest = &after[close + 1..];
        if selector.is_empty() {
            continue;
        }
        let (sel, state) = parse_head(&selector);
        let mut decls = Vec::new();
        for part in body.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if let Some(colon) = part.find(':') {
                let key = part[..colon].trim().to_lowercase();
                let value = part[colon + 1..].trim().to_string();
                if !key.is_empty() && !value.is_empty() {
                    decls.push((key, value));
                }
            }
        }
        out.push(Rule { sel, state, decls });
    }
    out
}

fn parse_selector(s: &str) -> Sel {
    if s == "*" {
        Sel::Any
    } else if let Some(name) = s.strip_prefix('.') {
        Sel::Class(name.to_string())
    } else {
        Sel::Type(s.to_lowercase())
    }
}

fn parse_head(s: &str) -> (Sel, State) {
    match s.split_once(':') {
        Some((name, st)) => {
            let state = match st.trim() {
                "focus" => State::Focus,
                "hover" => State::Hover,
                _ => State::Base,
            };
            (parse_selector(name.trim()), state)
        }
        None => (parse_selector(s), State::Base),
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
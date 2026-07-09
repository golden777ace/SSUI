use std::cell::RefCell;
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
}

impl Default for Props {
    fn default() -> Self {
        Self {
            axis: Axis::Vertical,
            padding: 0.0,
            gap: 0.0,
            width: None,
            height: None,
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
    },
}

/// Высота полосы вкладок в пикселях.
pub const TAB_HEADER: f32 = 40.0;

/// Высота заголовка таблицы и строки в пикселях.
pub const TABLE_HEADER: f32 = 34.0;
pub const TABLE_ROW: f32 = 30.0;

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
    on_dialog: Option<Box<dyn FnMut(&mut Tree, usize)>>,
    tint: f32,
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
            on_dialog: None,
            tint: 0.0,
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

    /// Задаёт альфа-канал фона окна (0..1).
    pub fn set_tint(&mut self, value: f32) {
        self.tint = value.clamp(0.0, 1.0);
    }

    /// Возвращает альфа-канал фона окна.
    pub fn tint(&self) -> f32 {
        self.tint
    }

    /// Возвращает очередь анимаций для внешнего добавления.
    pub fn anim_queue(&self) -> AnimQueue {
        self.pending.clone()
    }

    /// Есть ли активные анимации.
    pub fn has_anims(&self) -> bool {
        !self.anims.is_empty() || !self.pending.borrow().is_empty()
    }

    /// Продвигает анимации на `dt` секунд; true, если были активные.
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
        if let NodeKind::Label { text: t } = &mut self.nodes[id.0].kind {
            *t = text;
        }
    }

    /// Назначает обработчик клика для узла.
    pub fn set_on_click<F: FnMut(&mut Tree) + 'static>(&mut self, id: NodeId, f: F) {
        self.nodes[id.0].on_click = Some(Box::new(f));
    }

    /// Назначает обработчик изменения значения (ползунок).
    pub fn set_on_change<F: FnMut(&mut Tree, f32) + 'static>(&mut self, id: NodeId, f: F) {
        self.nodes[id.0].on_change = Some(Box::new(f));
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
            on_click: None,
            on_change: None,
            on_input: None,
        });
        self.nodes[parent.0].children.push(id);
        id
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

    /// Реагирует ли узел на клик (кнопка или чекбокс).
    pub fn is_interactive(&self, id: NodeId) -> bool {
        self.is_button(id) || self.is_checkbox(id)
    }

    /// Является ли узел ползунком.
    pub fn is_slider(&self, id: NodeId) -> bool {
        matches!(self.nodes[id.0].kind, NodeKind::Slider { .. })
    }

    /// Может ли узел получать фокус клавиатуры.
    pub fn is_focusable(&self, id: NodeId) -> bool {
        self.is_button(id)
            || self.is_checkbox(id)
            || self.is_textbox(id)
            || self.is_slider(id)
            || self.is_dropdown(id)
            || self.is_tabs(id)
            || self.is_table(id)
    }

    /// Список фокусируемых узлов в порядке обхода дерева.
    pub fn focusables(&self) -> Vec<NodeId> {
        let mut out = Vec::new();
        self.collect_focus(self.root, &mut out);
        out
    }

    fn collect_focus(&self, id: NodeId, out: &mut Vec<NodeId>) {
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

    /// Возвращает верхний узел, содержащий точку `(x, y)`.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<NodeId> {
        let mut hit = None;
        self.hit_walk(self.root, x, y, &mut hit);
        hit
    }

    fn hit_walk(&self, id: NodeId, x: f32, y: f32, hit: &mut Option<NodeId>) {
        if contains(self.nodes[id.0].rect, x, y) {
            *hit = Some(id);
        }
        let count = self.nodes[id.0].children.len();
        for i in 0..count {
            let child = self.nodes[id.0].children[i];
            self.hit_walk(child, x, y, hit);
        }
    }

    /// Вычисляет прямоугольники всех узлов линейной раскладкой.
    pub fn layout(&mut self, root_rect: Rect) {
        self.layout_node(self.root, root_rect);
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
            let off = Rect::new(-1.0e6, -1.0e6, 0.0, 0.0);
            for (i, &c) in children.iter().enumerate() {
                let cr = if i == selected { content } else { off };
                self.layout_node(c, cr);
            }
            return;
        }

        let pad = props.padding;
        let inner = Rect::new(
            rect.x + pad,
            rect.y + pad,
            (rect.width - 2.0 * pad).max(0.0),
            (rect.height - 2.0 * pad).max(0.0),
        );
        let n = children.len();
        let total_gap = props.gap * n.saturating_sub(1) as f32;

        let main_available = match props.axis {
            Axis::Vertical => inner.height - total_gap,
            Axis::Horizontal => inner.width - total_gap,
        };
        let mut fixed_sum = 0.0;
        let mut flex_count = 0usize;
        for &c in &children {
            let cp = self.nodes[c.0].props;
            let main = match props.axis {
                Axis::Vertical => cp.height,
                Axis::Horizontal => cp.width,
            };
            match main {
                Some(v) => fixed_sum += v,
                None => flex_count += 1,
            }
        }
        let flex_size = if flex_count > 0 {
            (main_available - fixed_sum).max(0.0) / flex_count as f32
        } else {
            0.0
        };

        let mut cursor = match props.axis {
            Axis::Vertical => inner.y,
            Axis::Horizontal => inner.x,
        };
        for &c in &children {
            let cp = self.nodes[c.0].props;
            let child_rect = match props.axis {
                Axis::Vertical => {
                    let h = cp.height.unwrap_or(flex_size);
                    let w = cp.width.unwrap_or(inner.width);
                    let r = Rect::new(inner.x, cursor, w, h);
                    cursor += h + props.gap;
                    r
                }
                Axis::Horizontal => {
                    let w = cp.width.unwrap_or(flex_size);
                    let h = cp.height.unwrap_or(inner.height);
                    let r = Rect::new(cursor, inner.y, w, h);
                    cursor += w + props.gap;
                    r
                }
            };
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

fn kind_tag(kind: &NodeKind) -> &'static str {
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
    let hex = value.trim().strip_prefix('#')?;
    match hex.len() {
        8 => u32::from_str_radix(hex, 16).ok().map(Color::hexa),
        6 => u32::from_str_radix(hex, 16).ok().map(Color::hex),
        3 => {
            let mut full = String::new();
            for ch in hex.chars() {
                full.push(ch);
                full.push(ch);
            }
            u32::from_str_radix(&full, 16).ok().map(Color::hex)
        }
        _ => None,
    }
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
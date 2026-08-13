use super::render::Input;
use crate::render::CursorKind;
use crate::tree::{NodeId, NodeKind, Tree};

/// Состояние указателя и фокуса между кадрами.
#[derive(Default)]
pub struct InputState {
    pub hot: Option<NodeId>,
    pub pressed: Option<NodeId>,
    pub focused: Option<NodeId>,
    /// Узел, чьё значение тянут мышью.
    pub dragging: Option<NodeId>,
    pub mouse: (f32, f32),
}

impl InputState {
    /// Снимок состояний для отрисовки кадра.
    pub fn snapshot(&self) -> Input {
        Input {
            hovered: self.hot,
            pressed: self.pressed,
            focused: self.focused,
        }
    }

    /// Движение мыши; `true` — нужна перерисовка.
    pub fn mouse_move(&mut self, tree: &mut Tree, x: f32, y: f32) -> bool {
        self.mouse = (x, y);
        if let Some(id) = self.dragging {
            let value = value_at(tree, id, x);
            tree.set_input(id, value);
            tree.fire_change(id, value);
            return true;
        }
        let hit = tree.hit_test(x, y);
        let changed = hit != self.hot;
        self.hot = hit;
        changed
    }

    /// Вид курсора для узла под мышью.
    pub fn cursor(&self, tree: &Tree) -> CursorKind {
        let id = match self.hot {
            Some(v) => v,
            None => return CursorKind::Arrow,
        };
        if tree.is_textbox(id) || tree.is_term(id) {
            return CursorKind::IBeam;
        }
        if tree.is_interactive(id)
            || tree.is_dropdown(id)
            || tree.is_split(id)
            || tree.is_menubar(id)
            || tree.is_crumbs(id)
            || tree.is_pager(id)
            || tree.is_rating(id)
            || tree.is_tabs(id)
            || tree.is_accordion(id)
        {
            return CursorKind::Hand;
        }
        CursorKind::Arrow
    }

    /// Курсор ушёл из окна.
    pub fn mouse_leave(&mut self) -> bool {
        let changed = self.hot.is_some();
        self.hot = None;
        changed
    }

    /// Нажатие левой кнопки; `true` — нужна перерисовка.
    pub fn mouse_down(&mut self, tree: &mut Tree, x: f32, y: f32) -> bool {
        self.mouse = (x, y);
        let hit = tree.hit_test(x, y);
        self.hot = hit;
        self.pressed = hit;
        self.focused = hit;
        if let Some(id) = hit {
            if is_track(tree, id) {
                self.dragging = Some(id);
                let value = value_at(tree, id, x);
                tree.set_input(id, value);
                tree.fire_change(id, value);
            }
        }
        true
    }

    /// Отпускание левой кнопки; `true` — нужна перерисовка.
    pub fn mouse_up(&mut self, tree: &mut Tree) -> bool {
        let dragged = self.dragging.take().is_some();
        let click = match (self.pressed.take(), self.hot) {
            (Some(p), Some(h)) if p == h => Some(p),
            _ => None,
        };
        if let Some(id) = click {
            activate(tree, id);
        }
        dragged || click.is_some()
    }
}

impl InputState {
    /// Колесо мыши; `d` — щелчки вверх. `true` — нужна перерисовка.
    pub fn wheel(&mut self, tree: &mut Tree, d: f32) -> bool {
        let (x, y) = self.mouse;
        let mut id = match tree.hit_test(x, y) {
            Some(v) => v,
            None => return false,
        };
        let mut guard = 0;
        loop {
            if tree.has_wheel(id) {
                return tree.fire_wheel(id, d, x, y);
            }
            match tree.parent(id) {
                Some(p) if guard < 64 => {
                    id = p;
                    guard += 1;
                }
                _ => return false,
            }
        }
    }

    /// Переводит фокус на следующий или предыдущий узел.
    pub fn focus_step(&mut self, tree: &Tree, back: bool) -> bool {
        let list = tree.focusables();
        if list.is_empty() {
            return false;
        }
        let pos = self.focused.and_then(|f| list.iter().position(|&x| x == f));
        let next = match (pos, back) {
            (Some(i), false) => (i + 1) % list.len(),
            (Some(0), true) => list.len() - 1,
            (Some(i), true) => i - 1,
            (None, false) => 0,
            (None, true) => list.len() - 1,
        };
        self.focused = Some(list[next]);
        true
    }

    /// Пробел или Enter по узлу в фокусе.
    pub fn activate_focused(&mut self, tree: &mut Tree) -> bool {
        match self.focused {
            Some(id) => {
                activate(tree, id);
                true
            }
            None => false,
        }
    }
}

/// Тянется ли значение узла мышью вдоль дорожки.
fn is_track(tree: &Tree, id: NodeId) -> bool {
    matches!(tree.get(id).kind, NodeKind::Slider { .. })
}

/// Доля 0..1 по горизонтали внутри прямоугольника узла.
fn value_at(tree: &Tree, id: NodeId, x: f32) -> f32 {
    let r = tree.get(id).rect;
    if r.width <= 0.0 {
        return 0.0;
    }
    ((x - r.x) / r.width).clamp(0.0, 1.0)
}

/// Отрабатывает клик по узлу: переключает состояние и зовёт обработчики.
fn activate(tree: &mut Tree, id: NodeId) {
    match tree.get(id).kind {
        NodeKind::Checkbox { .. } => {
            tree.toggle_checkbox(id);
            let on = matches!(&tree.get(id).kind, NodeKind::Checkbox { checked, .. } if *checked);
            tree.fire_change(id, if on { 1.0 } else { 0.0 });
            tree.fire_click(id);
        }
        NodeKind::Switch { .. } => {
            tree.toggle_switch(id);
            let v = if tree.switch_on(id) { 1.0 } else { 0.0 };
            tree.fire_change(id, v);
            tree.fire_click(id);
        }
        NodeKind::Toggle { .. } => {
            tree.flip_toggle(id);
            let v = if tree.toggle_on(id) { 1.0 } else { 0.0 };
            tree.fire_change(id, v);
            tree.fire_click(id);
        }
        NodeKind::Radio { .. } => {
            tree.select_radio(id);
            tree.fire_change(id, 1.0);
            tree.fire_click(id);
        }
        _ => tree.fire_click(id),
    }
}
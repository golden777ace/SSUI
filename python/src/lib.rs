use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use ssui_core::platform::{dpi, Window as CoreWindow, WindowOpts};
use ssui_core::tree::{
    Anim, AnimQueue, Axis, DialogData, DialogQueue, Ease, FileQueue, FileReq, FocusQueue,
    NodeId, NodeKind, CanvasQueue, NoteData, NoteQueue, PointerCb, Props, RectTable, Shape,
    TextState, TimerKill, TimerQueue, TimerReq, Tree, TreeGeom, TreeItem, TreeQueue, WheelCb,
    LIST_ROW, OFF_COORD,
};
use pyo3::types::PyAnyMethods;

type ShapeSpec = (String, Vec<f32>, String, String);

fn hexa(s: &str) -> u32 {
    ssui_core::render::parse_hex(s)
        .unwrap_or(ssui_core::render::Color::rgba(1.0, 1.0, 1.0, 1.0))
        .pack()
}

fn make_shapes(items: Vec<ShapeSpec>) -> Vec<Shape> {
    items
        .iter()
        .map(|(k, a, c, t)| {
            let kind = match k.as_str() {
                "rect" => 0u8,
                "circle" => 1,
                "line" => 2,
                "text" => 3,
                "arrow" => 4,
                "arc" => 5,
                "sector" => 6,
                "poly" => 7,
                _ => 3,
            };
            let mut args = [0.0f32; 6];
            let mut pts = Vec::new();
            if kind == 7 {
                let n = a.len();
                let cut = n - n % 2;
                if cut < n {
                    args[0] = a[n - 1];
                }
                pts.extend_from_slice(&a[..cut]);
            } else {
                for (i, v) in a.iter().take(6).enumerate() {
                    args[i] = *v;
                }
            }
            Shape {
                kind,
                args,
                color: hexa(c),
                text: utf16(t),
                pts,
            }
        })
        .collect()
}

/// Проверяет, принимает ли вызываемый объект два позиционных аргумента.
fn takes_two(py: Python, f: &PyObject) -> bool {
    let code = "import inspect\n\
                def n(f):\n\
                \x20 try:\n\
                \x20  return sum(1 for p in inspect.signature(f).parameters.values()\n\
                \x20   if p.kind in (p.POSITIONAL_ONLY, p.POSITIONAL_OR_KEYWORD))\n\
                \x20 except Exception:\n\
                \x20  return 1\n";
    let probe = || -> PyResult<usize> {
        let m = pyo3::types::PyModule::from_code(
            py,
            std::ffi::CString::new(code).unwrap().as_c_str(),
            std::ffi::CString::new("probe.py").unwrap().as_c_str(),
            std::ffi::CString::new("probe").unwrap().as_c_str(),
        )?;
        m.getattr("n")?.call1((f,))?.extract::<usize>()
    };
    probe().unwrap_or(1) >= 2
}

/// Оборачивает питоновский колбэк `f(строка, колонка)`.
fn pair_cb(f: PyObject, texts: Bindings, values: Bindings) -> PointerCb {
    Box::new(move |t, i, c, _| {
        Python::with_gil(|py| {
            if let Err(e) = f.bind(py).call1((i, c as i64)) {
                e.print(py);
            }
            refresh_all(py, t, &texts, &values);
        });
    })
}

/// Разбирает строки дерева: старые кортежи и новые словари.
fn tree_rows(items: &Bound<'_, PyAny>) -> PyResult<Vec<TreeItem>> {
    let mut out = Vec::new();
    for obj in items.try_iter()? {
        let obj = obj?;
        if let Ok((d, s, leaf)) = obj.extract::<(usize, String, bool)>() {
            out.push(TreeItem::new(d, utf16(&s), true, leaf));
            continue;
        }
        let pick = |k: &str| obj.get_item(k).ok();
        let depth = pick("depth")
            .and_then(|v| v.extract::<usize>().ok())
            .unwrap_or(0);
        let text = pick("text")
            .and_then(|v| v.extract::<String>().ok())
            .unwrap_or_default();
        let leaf = pick("leaf")
            .and_then(|v| v.extract::<bool>().ok())
            .unwrap_or(false);
        let open = pick("open")
            .and_then(|v| v.extract::<bool>().ok())
            .unwrap_or(true);
        let values = pick("values")
            .and_then(|v| v.extract::<Vec<String>>().ok())
            .unwrap_or_default();
        let bg = pick("bg")
            .and_then(|v| v.extract::<String>().ok())
            .map(|s| hexa(&s));
        let fg = pick("fg")
            .and_then(|v| v.extract::<String>().ok())
            .map(|s| hexa(&s));
        let icon = pick("icon").and_then(|v| v.extract::<String>().ok());
        let paint = |k: &str| -> Vec<Option<u32>> {
            pick(k)
                .and_then(|v| v.extract::<Vec<String>>().ok())
                .unwrap_or_default()
                .iter()
                .map(|s| if s.is_empty() { None } else { Some(hexa(s)) })
                .collect()
        };
        out.push(TreeItem {
            depth,
            label: utf16(&text),
            open,
            leaf,
            values: values.iter().map(|s| utf16(s)).collect(),
            bg,
            fg,
            icon,
            cbg: paint("cbg"),
            cfg: paint("cfg"),
        });
    }
    Ok(out)
}

#[pyclass(name = "N")]
#[derive(Clone, Copy)]
struct PyNode {
    id: NodeId,
}

type Bindings = Rc<RefCell<Vec<(NodeId, Py<PyAny>)>>>;
type Stack = Rc<RefCell<Vec<NodeId>>>;
type BindVec = RefCell<Vec<(NodeId, Py<PyAny>)>>;

/// Состояние окна, доступное из любого потока.
struct Share {
    id: u64,
    hwnd: AtomicIsize,
    wake: AtomicBool,
    alive: AtomicBool,
}

static POSTS: Mutex<Vec<(u64, Py<PyAny>)>> = Mutex::new(Vec::new());
static WIN_SEQ: AtomicU64 = AtomicU64::new(0);
static TIMER_SEQ: AtomicU64 = AtomicU64::new(0);

/// Разбирает очередь вызовов окна `id`; true — что-то выполнено.
fn drain_posts(share: &Share, t: &mut Tree, texts: &Bindings, values: &Bindings) -> bool {
    share.wake.store(false, Ordering::Release);
    let mine = {
        let mut q = match POSTS.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        if q.is_empty() {
            return false;
        }
        let taken = std::mem::take(&mut *q);
        let mut mine = Vec::new();
        let mut rest = Vec::with_capacity(taken.len());
        for (wid, f) in taken {
            if wid == share.id {
                mine.push(f);
            } else {
                rest.push((wid, f));
            }
        }
        *q = rest;
        mine
    };
    if mine.is_empty() {
        return false;
    }
    Python::with_gil(|py| {
        for f in mine {
            if let Err(e) = f.bind(py).call0() {
                e.print(py);
            }
        }
        refresh_all(py, t, texts, values);
    });
    true
}

#[pyclass(name = "Post")]
struct Post {
    share: Arc<Share>,
}

#[pymethods]
impl Post {
    /// Ставит вызов в очередь UI-потока; безопасно из любого потока.
    fn __call__(&self, f: PyObject) -> PyResult<()> {
        if !self.share.alive.load(Ordering::Acquire) {
            return Ok(());
        }
        {
            let mut q = match POSTS.lock() {
                Ok(g) => g,
                Err(e) => e.into_inner(),
            };
            q.push((self.share.id, f));
        }
        if self
            .share
            .wake
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            CoreWindow::wake(self.share.hwnd.load(Ordering::Acquire));
        }
        Ok(())
    }
}

/// Биндинги, приватные для одного окна.
#[derive(Default)]
struct Extra {
    chart: BindVec,
    prop: BindVec,
    canvas: BindVec,
    boxes: BindVec,
    list: BindVec,
    table: BindVec,
    depth: BindVec,
    pos: BindVec,
    src: BindVec,
    tre: BindVec,
    tbg: BindVec,
}

impl Extra {
    fn slot(&self, tag: u8) -> &BindVec {
        match tag {
            0 => &self.chart,
            1 => &self.prop,
            2 => &self.canvas,
            3 => &self.boxes,
            4 => &self.list,
            5 => &self.table,
            6 => &self.depth,
            7 => &self.pos,
            9 => &self.tre,
            10 => &self.tbg,
            _ => &self.src,
        }
    }
}

/// Ссылка на слот биндингов текущего окна.
struct BindSlot(u8);

impl BindSlot {
    fn with<R>(&self, f: impl FnOnce(&BindVec) -> R) -> R {
        let cur = CUR_EXTRA.with(|c| c.borrow().clone());
        match cur {
            Some(e) => f(e.slot(self.0)),
            None => f(&BindVec::default()),
        }
    }
}

static CHART_BINDINGS: BindSlot = BindSlot(0);
static PROP_BINDINGS: BindSlot = BindSlot(1);
static CANVAS_BINDINGS: BindSlot = BindSlot(2);
static BOX_BINDINGS: BindSlot = BindSlot(3);
static LIST_BINDINGS: BindSlot = BindSlot(4);
static TABLE_BINDINGS: BindSlot = BindSlot(5);
static DEPTH_BINDINGS: BindSlot = BindSlot(6);
static TREE_BINDINGS: BindSlot = BindSlot(9);
static TBG_BINDINGS: BindSlot = BindSlot(10);

/// Подставляет биндинги окна на время обновления.
struct ExtraGuard(Option<Rc<Extra>>);

impl ExtraGuard {
    fn enter(texts: &Bindings) -> Self {
        let key = Rc::as_ptr(texts) as usize;
        let next = EXTRAS.with(|m| m.borrow().get(&key).cloned());
        let prev = CUR_EXTRA.with(|c| c.borrow_mut().take());
        CUR_EXTRA.with(|c| *c.borrow_mut() = next);
        ExtraGuard(prev)
    }
}

impl Drop for ExtraGuard {
    fn drop(&mut self) {
        let prev = self.0.take();
        CUR_EXTRA.with(|c| *c.borrow_mut() = prev);
    }
}

thread_local! {
    static ALIVE: RefCell<Vec<CoreWindow>> = RefCell::new(Vec::new());
    static WIN_SETTINGS: RefCell<Vec<(u8, Py<PyAny>)>> = RefCell::new(Vec::new());
    static EXTRAS: RefCell<HashMap<usize, Rc<Extra>>> = RefCell::new(HashMap::new());
    static CUR_EXTRA: RefCell<Option<Rc<Extra>>> = const { RefCell::new(None) };
    static TRACK: RefCell<Option<Vec<u32>>> = RefCell::new(None);
    static DIRTY: RefCell<HashSet<u32>> = RefCell::new(HashSet::new());
    static DEPS: RefCell<HashMap<usize, Vec<u32>>> = RefCell::new(HashMap::new());
    static SIG_SEQ: Cell<u32> = Cell::new(0);
}

fn next_sig_id() -> u32 {
    SIG_SEQ.with(|c| {
        let v = c.get().wrapping_add(1);
        c.set(v);
        v
    })
}

fn mark_read(id: u32) {
    TRACK.with(|t| {
        if let Some(v) = t.borrow_mut().as_mut() {
            if !v.contains(&id) {
                v.push(id);
            }
        }
    });
}

fn mark_dirty(id: u32) {
    DIRTY.with(|s| {
        s.borrow_mut().insert(id);
    });
}

/// Вызывает биндинг с учётом подписок на сигналы.
fn run_binding<'py>(py: Python<'py>, f: &Py<PyAny>) -> Option<Bound<'py, PyAny>> {
    let key = f.as_ptr() as usize;
    let skip = DEPS.with(|d| {
        let d = d.borrow();
        match d.get(&key) {
            Some(deps) if !deps.is_empty() => DIRTY.with(|s| {
                let s = s.borrow();
                !deps.iter().any(|i| s.contains(i))
            }),
            _ => false,
        }
    });
    if skip {
        return None;
    }
    TRACK.with(|t| *t.borrow_mut() = Some(Vec::new()));
    let r = f.bind(py).call0();
    let read = TRACK.with(|t| t.borrow_mut().take()).unwrap_or_default();
    DEPS.with(|d| {
        let mut d = d.borrow_mut();
        let e = d.entry(key).or_default();
        for i in read {
            if !e.contains(&i) {
                e.push(i);
            }
        }
    });
    match r {
        Ok(v) => Some(v),
        Err(e) => {
            e.print(py);
            None
        }
    }
}

/// Код клавиши по имени: буква, цифра, fN или именованная клавиша.
fn key_vk(name: &str) -> Option<u32> {
    Some(match name {
        "enter" | "return" => 0x0D,
        "space" => 0x20,
        "tab" => 0x09,
        "escape" | "esc" => 0x1B,
        "delete" | "del" => 0x2E,
        "backspace" => 0x08,
        "insert" | "ins" => 0x2D,
        "home" => 0x24,
        "end" => 0x23,
        "pageup" => 0x21,
        "pagedown" => 0x22,
        "left" => 0x25,
        "up" => 0x26,
        "right" => 0x27,
        "down" => 0x28,
        "plus" => 0xBB,
        "minus" => 0xBD,
        s if s.len() == 1 => {
            let c = s.as_bytes()[0];
            if c.is_ascii_lowercase() {
                c.to_ascii_uppercase() as u32
            } else if c.is_ascii_digit() {
                c as u32
            } else {
                return None;
            }
        }
        s if s.starts_with('f') => {
            let n: u32 = s[1..].parse().ok()?;
            if (1..=24).contains(&n) {
                0x70 + n - 1
            } else {
                return None;
            }
        }
        _ => return None,
    })
}

/// Разбирает спецификацию хоткея в маску модификаторов и код клавиши.
fn parse_hotkey(spec: &str) -> Option<(u8, u32)> {
    let mut mods = 0u8;
    let mut vk = None;
    for part in spec.split('+') {
        let p = part.trim().to_ascii_lowercase();
        match p.as_str() {
            "" => {}
            "ctrl" | "control" => mods |= 1,
            "shift" => mods |= 2,
            "alt" => mods |= 4,
            other => vk = Some(key_vk(other)?),
        }
    }
    vk.map(|v| (mods, v))
}

#[pyclass(unsendable, name = "Ctx")]
struct Ctx {
    stack: Stack,
    node: NodeId,
}

#[pymethods]
impl Ctx {
    fn __enter__(&self) -> PyNode {
        self.stack.borrow_mut().push(self.node);
        PyNode { id: self.node }
    }

    #[pyo3(signature = (_t=None, _v=None, _tb=None))]
    fn __exit__(
        &self,
        _t: Option<PyObject>,
        _v: Option<PyObject>,
        _tb: Option<PyObject>,
    ) -> bool {
        self.stack.borrow_mut().pop();
        false
    }
}

#[pyclass(unsendable, name = "Fx")]
struct Fx {
    queue: AnimQueue,
    texts: Bindings,
    values: Bindings,
}

#[pymethods]
impl Fx {
    /// Анимирует сигнал `sig` к значению `to` за `dur` секунд.
    #[pyo3(signature = (sig, to, *, frm=None, dur=0.3, ease="out"))]
    fn __call__(
        &self,
        py: Python,
        sig: PyObject,
        to: f32,
        frm: Option<f32>,
        dur: f32,
        ease: &str,
    ) -> PyResult<()> {
        let from = match frm {
            Some(v) => v,
            None => sig.bind(py).call0()?.extract::<f32>()?,
        };
        let e = parse_ease(ease);
        let texts = self.texts.clone();
        let values = self.values.clone();
        self.queue
            .borrow_mut()
            .push(Anim::new(from, to, dur, e, move |t, v| {
                Python::with_gil(|py| {
                    if let Err(err) = sig.bind(py).call_method1("st", (v,)) {
                        err.print(py);
                    }
                    refresh_all(py, t, &texts, &values);
                });
            }));
        Ok(())
    }
}

#[pyclass(unsendable, name = "Rct")]
struct Rct {
    table: RectTable,
}

#[pymethods]
impl Rct {
    /// Прямоугольник узла после раскладки: `(x, y, w, h)`.
    fn __call__(&self, node: PyNode) -> (f32, f32, f32, f32) {
        let t = self.table.borrow();
        match t.get(node.id.index()) {
            Some(r) => (r.x, r.y, r.width, r.height),
            None => (0.0, 0.0, 0.0, 0.0),
        }
    }
}

#[pyclass(unsendable, name = "Fl")]
struct Fl {
    queue: FileQueue,
    share: Arc<Share>,
    texts: Bindings,
    values: Bindings,
}

impl Fl {
    fn push(
        &self,
        mode: u8,
        title: &str,
        name: &str,
        patterns: Vec<(String, String)>,
        on: Option<PyObject>,
    ) {
        let texts = self.texts.clone();
        let values = self.values.clone();
        let cb: Box<dyn FnMut(&mut Tree, String)> = Box::new(move |t, path| {
            Python::with_gil(|py| {
                if let Some(f) = &on {
                    if let Err(e) = f.bind(py).call1((path,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        self.queue.borrow_mut().push(FileReq {
            mode,
            title: title.to_string(),
            name: name.to_string(),
            patterns,
            cb,
        });
        CoreWindow::post_files(self.share.hwnd.load(Ordering::Acquire));
    }
}

#[pymethods]
impl Fl {
    /// Диалог выбора файла; `on(path)` с путём или "" при отмене.
    #[pyo3(signature = (*, title="", patterns=Vec::new(), on=None))]
    fn open(
        &self,
        title: &str,
        patterns: Vec<(String, String)>,
        on: Option<PyObject>,
    ) -> PyResult<()> {
        self.push(0, title, "", patterns, on);
        Ok(())
    }

    /// Диалог сохранения; `name` — имя по умолчанию.
    #[pyo3(signature = (*, title="", name="", patterns=Vec::new(), on=None))]
    fn save(
        &self,
        title: &str,
        name: &str,
        patterns: Vec<(String, String)>,
        on: Option<PyObject>,
    ) -> PyResult<()> {
        self.push(1, title, name, patterns, on);
        Ok(())
    }

    /// Диалог выбора папки; `on(path)` с путём или "".
    #[pyo3(signature = (*, title="", on=None))]
    fn dir(&self, title: &str, on: Option<PyObject>) -> PyResult<()> {
        self.push(2, title, "", Vec::new(), on);
        Ok(())
    }
}

#[pyclass(unsendable, name = "Clip")]
struct Clip;

#[pymethods]
impl Clip {
    /// Возвращает текст из буфера обмена; пустая строка, если он пуст.
    fn get(&self) -> String {
        ssui_core::render::clipboard_get()
    }

    /// Кладёт текст в буфер обмена.
    fn set(&self, text: &str) {
        ssui_core::render::clipboard_set(text);
    }
}

#[pyclass(unsendable, name = "Fnt")]
struct Fnt {
    queue: Rc<RefCell<Option<(String, f32)>>>,
}

#[pymethods]
impl Fnt {
    /// Меняет базовый шрифт вживую: семейство и размер.
    #[pyo3(signature = (family="Segoe UI", size=20.0))]
    fn __call__(&self, family: &str, size: f32) {
        *self.queue.borrow_mut() = Some((family.to_string(), size.max(1.0)));
    }
}

#[pyclass(unsendable, name = "Thm")]
struct Thm {
    queue: Rc<RefCell<Option<usize>>>,
}

#[pymethods]
impl Thm {
    /// Меняет тему окна: `wht`, `lit`, `drk`, `blk`.
    fn __call__(&self, name: &str) {
        *self.queue.borrow_mut() = Some(theme_index(name));
    }
}

#[pyclass(unsendable, name = "Note")]
struct Note {
    queue: NoteQueue,
    texts: Bindings,
    values: Bindings,
}

impl Note {
    fn push(
        &self,
        kind: u8,
        title: Vec<u16>,
        text: Vec<u16>,
        action: Vec<u16>,
        secs: f32,
        on: Option<PyObject>,
    ) {
        let texts = self.texts.clone();
        let values = self.values.clone();
        let cb: Option<Box<dyn FnMut(&mut Tree)>> = match on {
            Some(f) => Some(Box::new(move |t: &mut Tree| {
                Python::with_gil(|py| {
                    if let Err(e) = f.bind(py).call0() {
                        e.print(py);
                    }
                    refresh_all(py, t, &texts, &values);
                });
            })),
            None => None,
        };
        self.queue.borrow_mut().push(NoteData {
            title,
            text,
            action,
            secs,
            kind,
            cb,
        });
    }
}

#[pymethods]
impl Note {
    /// Уведомление в правом верхнем углу; `on()` по кнопке действия.
    #[pyo3(signature = (title, text, *, secs=4.0, action="", on=None))]
    fn __call__(
        &self,
        title: &str,
        text: &str,
        secs: f32,
        action: &str,
        on: Option<PyObject>,
    ) -> PyResult<()> {
        self.push(0, utf16(title), utf16(text), utf16(action), secs, on);
        Ok(())
    }

    /// Снэкбар внизу окна; `on()` по кнопке действия.
    #[pyo3(signature = (text, *, secs=4.0, action="", on=None))]
    fn snack(
        &self,
        text: &str,
        secs: f32,
        action: &str,
        on: Option<PyObject>,
    ) -> PyResult<()> {
        self.push(1, Vec::new(), utf16(text), utf16(action), secs, on);
        Ok(())
    }
}

#[pyclass(unsendable, name = "Dlg")]
struct Dlg {
    queue: DialogQueue,
    texts: Bindings,
    values: Bindings,
}

#[pymethods]
impl Dlg {
    /// Показывает модальный диалог; `on(index)` по нажатию кнопки.
    #[pyo3(signature = (title, message, buttons, *, on=None))]
    fn __call__(
        &self,
        title: &str,
        message: &str,
        buttons: Vec<String>,
        on: Option<PyObject>,
    ) -> PyResult<()> {
        let btns: Vec<Vec<u16>> = buttons.iter().map(|s| utf16(s)).collect();
        let texts = self.texts.clone();
        let values = self.values.clone();
        let data = DialogData {
            title: utf16(title),
            message: utf16(message),
            buttons: btns,
            cb: Box::new(move |t, i| {
                Python::with_gil(|py| {
                    if let Some(cb) = &on {
                        if let Err(e) = cb.bind(py).call1((i as i64,)) {
                            e.print(py);
                        }
                    }
                    refresh_all(py, t, &texts, &values);
                });
            }),
        };
        *self.queue.borrow_mut() = Some(data);
        Ok(())
    }

    /// Информационное окно с одной кнопкой.
    #[pyo3(signature = (title, message, *, ok="Ок", on=None))]
    fn msg(
        &self,
        title: &str,
        message: &str,
        ok: &str,
        on: Option<PyObject>,
    ) -> PyResult<()> {
        self.__call__(title, message, vec![ok.to_string()], on)
    }

    /// Предупреждение с одной кнопкой.
    #[pyo3(signature = (message, *, title="Внимание", ok="Ок", on=None))]
    fn alert(
        &self,
        message: &str,
        title: &str,
        ok: &str,
        on: Option<PyObject>,
    ) -> PyResult<()> {
        let head = format!("\u{26A0} {}", title);
        self.__call__(&head, message, vec![ok.to_string()], on)
    }
}

#[pyclass(unsendable, name = "W")]
struct PyWindow {
    tree: Option<Tree>,
    title: String,
    width: i32,
    height: i32,
    root: NodeId,
    stack: Stack,
    bindings: Bindings,
    value_bindings: Bindings,
    anim_queue: AnimQueue,
    dialog_queue: DialogQueue,
    note_queue: NoteQueue,
    theme_queue: Rc<RefCell<Option<usize>>>,
    font_queue: Rc<RefCell<Option<(String, f32)>>>,
    focus_queue: FocusQueue,
    rect_table: RectTable,
    timer_queue: TimerQueue,
    kill_timers: TimerKill,
    canvas_queue: CanvasQueue,
    tree_queue: TreeQueue,
    tree_geom: TreeGeom,
    file_queue: FileQueue,
    extra: Rc<Extra>,
    hwnd: Rc<Cell<isize>>,
    share: Arc<Share>,
    icon: Option<String>,
    caption: Option<u32>,
    caption_text: Option<u32>,
    border: Option<u32>,
    dark: Option<bool>,
    on_close: Option<PyObject>,
    parent: Option<(AnimQueue, Bindings, Bindings)>,
    owner: isize,
    modal: bool,
    frameless: bool,
    topmost: bool,
    center: bool,
    resizable: bool,
    minbox: bool,
    maxbox: bool,
    closebox: bool,
    glass: bool,
    tint: f32,
    blur: bool,
}

#[pymethods]
impl PyWindow {
    #[new]
    #[pyo3(signature = (
        ttl="SSUI", w=1280, h=720, thm="drk", glass=false, tint=0.0, blur=false,
        frameless=false, topmost=false, center=false, resizable=true,
        minbox=true, maxbox=true, closebox=true, insp=false
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        ttl: &str,
        w: i32,
        h: i32,
        thm: &str,
        glass: bool,
        tint: f32,
        blur: bool,
        frameless: bool,
        topmost: bool,
        center: bool,
        resizable: bool,
        minbox: bool,
        maxbox: bool,
        closebox: bool,
        insp: bool,
    ) -> Self {
        let mut tree = Tree::new();
        tree.set_inspect(insp);
        tree.set_theme(theme_index(thm));
        let root = tree.root();
        let anim_queue = tree.anim_queue();
        let dialog_queue = tree.dialog_queue();
        let note_queue = tree.note_queue();
        let theme_queue = tree.theme_queue();
        let font_queue = tree.font_queue();
        let focus_queue = tree.focus_queue();
        let rect_table = tree.rect_table();
        let timer_queue = tree.timer_queue();
        let kill_timers = tree.kill_queue();
        let canvas_queue = tree.canvas_queue();
        let tree_queue = tree.tree_queue();
        let tree_geom = tree.tree_geom();
        let file_queue = tree.file_queue();
        let bindings: Bindings = Rc::new(RefCell::new(Vec::new()));
        let extra = Rc::new(Extra::default());
        EXTRAS.with(|m| {
            m.borrow_mut()
                .insert(Rc::as_ptr(&bindings) as usize, extra.clone())
        });
        CUR_EXTRA.with(|c| *c.borrow_mut() = Some(extra.clone()));
        Self {
            tree: Some(tree),
            title: ttl.to_string(),
            width: w,
            height: h,
            root,
            stack: Rc::new(RefCell::new(Vec::new())),
            bindings,
            value_bindings: Rc::new(RefCell::new(Vec::new())),
            anim_queue,
            dialog_queue,
            note_queue,
            theme_queue,
            font_queue,
            focus_queue,
            rect_table,
            timer_queue,
            kill_timers,
            canvas_queue,
            tree_queue,
            tree_geom,
            file_queue,
            extra,
            hwnd: Rc::new(Cell::new(0)),
            share: Arc::new(Share {
                id: WIN_SEQ.fetch_add(1, Ordering::Relaxed),
                hwnd: AtomicIsize::new(0),
                wake: AtomicBool::new(false),
                alive: AtomicBool::new(true),
            }),
            icon: None,
            caption: None,
            caption_text: None,
            border: None,
            dark: None,
            on_close: None,
            parent: None,
            owner: 0,
            modal: false,
            frameless,
            topmost,
            center,
            resizable,
            minbox,
            maxbox,
            closebox,
            glass,
            tint,
            blur,
        }
    }

    /// Создаёт дочернее окно с собственным деревом виджетов.
    #[pyo3(signature = (
        ttl="", w=520, h=420, *, thm="drk", modal=false, center=true,
        frameless=false, topmost=false, resizable=true,
        minbox=false, maxbox=false, closebox=true,
        glass=false, tint=0.0, blur=false, insp=false, on_close=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn subwin(
        &self,
        py: Python,
        ttl: &str,
        w: i32,
        h: i32,
        thm: &str,
        modal: bool,
        center: bool,
        frameless: bool,
        topmost: bool,
        resizable: bool,
        minbox: bool,
        maxbox: bool,
        closebox: bool,
        glass: bool,
        tint: f32,
        blur: bool,
        insp: bool,
        on_close: Option<PyObject>,
    ) -> PyResult<Py<PyWindow>> {
        let mut child = PyWindow::new(
            ttl, w, h, thm, glass, tint, blur, frameless, topmost, center, resizable,
            minbox, maxbox, closebox, insp,
        );
        child.owner = self.hwnd.get();
        child.modal = modal;
        child.on_close = on_close;
        child.parent = Some((
            self.anim_queue.clone(),
            self.bindings.clone(),
            self.value_bindings.clone(),
        ));
        Py::new(py, child)
    }

    /// Закрывает окно программно.
    fn close(&self) -> PyResult<()> {
        self.share.alive.store(false, Ordering::Release);
        let h = self.hwnd.get();
        if h != 0 {
            ALIVE.with(|v| {
                if let Some(win) = v.borrow().iter().find(|x| x.handle() == h) {
                    win.close();
                }
            });
            self.hwnd.set(0);
        }
        Ok(())
    }

    /// Показывает окно, не блокируя цикл сообщений.
    fn show(slf: &Bound<'_, Self>) -> PyResult<()> {
        let (tree, title, width, height, mut opts, cell, done, parent, share) = {
            let mut me = slf.borrow_mut();
            let tree = me.tree.take().ok_or_else(consumed)?;
            let done = me.on_close.take();
            let parent = me.parent.take();
            (
                tree,
                me.title.clone(),
                me.width,
                me.height,
                me.opts(),
                me.hwnd.clone(),
                done,
                parent,
                me.share.clone(),
            )
        };
        let notify = cell.clone();
        let dead = share.clone();
        opts.on_close = Some(Box::new(move || {
            dead.alive.store(false, Ordering::Release);
            notify.set(0);
            if let Some(cb) = &done {
                Python::with_gil(|py| {
                    if let Err(e) = cb.bind(py).call0() {
                        e.print(py);
                    }
                });
            }
            if let Some((queue, texts, values)) = &parent {
                let t2 = texts.clone();
                let v2 = values.clone();
                queue.borrow_mut().push(Anim::new(
                    0.0,
                    1.0,
                    0.01,
                    Ease::Linear,
                    move |t, _| {
                        Python::with_gil(|py| refresh_all(py, t, &t2, &v2));
                    },
                ));
            }
        }));
        dpi::enable_dpi_awareness();
        let window = CoreWindow::with_opts(&title, width, height, tree, opts)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        cell.set(window.handle());
        share.hwnd.store(window.handle(), Ordering::Release);
        window.raise();
        ALIVE.with(|v| v.borrow_mut().push(window));
        Ok(())
    }

    /// Возвращает корневой узел окна.
    fn rt(&self) -> PyNode {
        PyNode { id: self.root }
    }

    /// Число кадров в файле изображения; для GIF — длина анимации.
    #[staticmethod]
    fn frames(path: &str) -> u32 {
        ssui_core::render::frame_count(path)
    }

    /// Ширина и высота строки в пикселях: `(w, h)`.
    #[staticmethod]
    #[pyo3(signature = (text, size=15.0, family="Segoe UI"))]
    fn measure_text(text: &str, size: f32, family: &str) -> (f32, f32) {
        ssui_core::render::measure_text(text, family, size)
    }

    /// Смена темы из кода: `thm("drk")`.
    fn thm(&self) -> Thm {
        Thm {
            queue: self.theme_queue.clone(),
        }
    }

    /// Живая смена шрифта: `fnt(family, size)`.
    fn fnt(&self) -> Fnt {
        Fnt {
            queue: self.font_queue.clone(),
        }
    }

    /// Делает узел прозрачным для мыши; клики проходят насквозь.
    #[pyo3(signature = (node, on=true))]
    fn ghost(&mut self, node: PyNode, on: bool) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_ghost(node.id, on);
        Ok(())
    }

    /// Поднимает узел поверх соседей при нажатии внутри него.
    #[pyo3(signature = (node, on=true))]
    fn front(&mut self, node: PyNode, on: bool) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_front(node.id, on);
        Ok(())
    }

    /// Привязывает числовой колбэк к узлу (значение или страница стопки).
    fn bindv(&mut self, node: PyNode, f: PyObject) -> PyResult<()> {
        self.value_bindings.borrow_mut().push((node.id, f));
        Ok(())
    }

    /// Привязывает `(padding, gap)` контейнера к колбэку.
    fn bindb(&mut self, node: PyNode, f: PyObject) -> PyResult<()> {
        self.extra.boxes.borrow_mut().push((node.id, f));
        Ok(())
    }

    /// Привязывает пункты списка к колбэку, возвращающему список строк.
    fn bindl(&mut self, node: PyNode, f: PyObject) -> PyResult<()> {
        self.extra.list.borrow_mut().push((node.id, f));
        Ok(())
    }

    /// Привязывает строки таблицы к колбэку, возвращающему список рядов.
    fn bindt(&mut self, node: PyNode, f: PyObject) -> PyResult<()> {
        self.extra.table.borrow_mut().push((node.id, f));
        Ok(())
    }

    /// Привязывает глубину `z` узла к колбэку, возвращающему число.
    fn bindz(&mut self, node: PyNode, f: PyObject) -> PyResult<()> {
        self.extra.depth.borrow_mut().push((node.id, f));
        Ok(())
    }

    /// Оформление рамки окна; вызывается до показа окна.
    #[pyo3(signature = (*, icon=None, cap=None, cap_txt=None, brd=None, dark=None))]
    fn frame(
        &mut self,
        icon: Option<String>,
        cap: Option<&str>,
        cap_txt: Option<&str>,
        brd: Option<&str>,
        dark: Option<bool>,
    ) -> PyResult<()> {
        if icon.is_some() {
            self.icon = icon;
        }
        if let Some(c) = Self::colorref(cap) {
            self.caption = Some(c);
        }
        if let Some(c) = Self::colorref(cap_txt) {
            self.caption_text = Some(c);
        }
        if let Some(c) = Self::colorref(brd) {
            self.border = Some(c);
        }
        if dark.is_some() {
            self.dark = dark;
        }
        Ok(())
    }

    /// Периодический вызов каждые `ms` мс; возвращает идентификатор.
    fn every(&self, ms: f32, f: PyObject) -> PyResult<u64> {
        Ok(self.push_timer(ms, false, f))
    }

    /// Одноразовый вызов через `ms` мс; возвращает идентификатор.
    fn after(&self, ms: f32, f: PyObject) -> PyResult<u64> {
        Ok(self.push_timer(ms, true, f))
    }

    /// Отменяет таймер по идентификатору; повторный вызов безвреден.
    fn cancel(&self, tid: u64) -> PyResult<()> {
        self.kill_timers.borrow_mut().push(tid);
        Ok(())
    }

    /// Привязывает абсолютную позицию узла к колбэку `(x, y, w, h)`.
    fn bindp(&mut self, node: PyNode, f: PyObject) -> PyResult<()> {
        self.extra.pos.borrow_mut().push((node.id, f));
        Ok(())
    }

    /// Доступ к прямоугольникам узлов: `rc = win.rects()`, затем `rc(node)`.
    fn rects(&self) -> Rct {
        Rct {
            table: self.rect_table.clone(),
        }
    }

    /// Очередь вызовов в UI-потоке: `p = win.post()`, далее `p(f)` из любого потока.
    fn post(&mut self) -> PyResult<Post> {
        let share = self.share.clone();
        let hook = share.clone();
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_on_frame(Box::new(move |t| drain_posts(&hook, t, &texts, &values)));
        Ok(Post { share })
    }

    /// Ставит фокус на узел; `txt` подменяет текст, `sel` выделяет всё.
    #[pyo3(signature = (node=None, *, txt=None, sel=true))]
    fn focus(&self, node: Option<PyNode>, txt: Option<&str>, sel: bool) -> PyResult<()> {
        let text = txt.map(utf16);
        *self.focus_queue.borrow_mut() = Some((node.map(|n| n.id), text, sel));
        Ok(())
    }

    /// Колесо над узлом: `f(дельта, x, y)`; узел перехватывает событие.
    fn wheel(&mut self, node: PyNode, f: PyObject) -> PyResult<()> {
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let cb: WheelCb = Box::new(move |t, d, x, y| {
            Python::with_gil(|py| {
                if let Err(e) = f.bind(py).call1((d, x, y)) {
                    e.print(py);
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        tree.set_on_wheel(node.id, cb);
        Ok(())
    }

    /// Реакция поля ввода на клавиши: `cb(1)` — Enter, `cb(0)` — Esc.
    fn keys(&mut self, node: PyNode, f: PyObject) -> PyResult<()> {
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_on_change(node.id, move |t, v| {
            Python::with_gil(|py| {
                if let Err(e) = f.bind(py).call1((v as i64,)) {
                    e.print(py);
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(())
    }

    /// Уведомления: `nt(title, text)` и `nt.snack(text)`.
    fn nt(&self) -> Note {
        Note {
            queue: self.note_queue.clone(),
            texts: self.bindings.clone(),
            values: self.value_bindings.clone(),
        }
    }

    /// Задаёт flex-вес узла вдоль главной оси.
    fn grow(&mut self, n: PyNode, g: f32) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_grow(n.id, g);
        Ok(())
    }

    /// Выравнивает детей узла: `justify` (st/cnt/end/btw), `cross` (str/st/cnt/end).
    #[pyo3(signature = (n, *, justify="st", cross="str"))]
    fn align(&mut self, n: PyNode, justify: &str, cross: &str) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_align(n.id, justify_code(justify), cross_code(cross));
        Ok(())
    }

    /// Пинит узел к краям родителя; `l/t/r/b` — отступы, `None` — не привязан.
    #[pyo3(signature = (n, *, l=None, t=None, r=None, b=None))]
    fn pin(
        &mut self,
        n: PyNode,
        l: Option<f32>,
        t: Option<f32>,
        r: Option<f32>,
        b: Option<f32>,
    ) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_pin(n.id, l, t, r, b);
        Ok(())
    }

    /// Возвращает контроллер анимаций.
    fn fx(&self) -> Fx {
        Fx {
            queue: self.anim_queue.clone(),
            texts: self.bindings.clone(),
            values: self.value_bindings.clone(),
        }
    }

    /// Базовый шрифт приложения: семейство и размер.
    #[pyo3(signature = (family="Segoe UI", size=20.0))]
    fn font(&mut self, family: &str, size: f32) -> PyResult<()> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        tree.set_font(family, size);
        Ok(())
    }

    /// Привязывает прозрачность фона окна к сигналу 0..1.
    fn tint(&mut self, sig: PyObject) -> PyResult<()> {
        set_win_setting(0, sig);
        Ok(())
    }

    /// Привязывает силу размытия к сигналу 0..1 (0 — выключено).
    fn blur(&mut self, sig: PyObject) -> PyResult<()> {
        set_win_setting(1, sig);
        Ok(())
    }

    /// Привязывает режим фона к сигналу: 0 — нет, иначе — размытие.
    fn blur_mode(&mut self, sig: PyObject) -> PyResult<()> {
        set_win_setting(2, sig);
        Ok(())
    }

    /// Привязывает гашение размытия при перемещении к сигналу (0/1).
    fn drag_smooth(&mut self, sig: PyObject) -> PyResult<()> {
        set_win_setting(3, sig);
        Ok(())
    }

    /// Возвращает контроллер диалогов.
    fn dlg(&self) -> Dlg {
        Dlg {
            queue: self.dialog_queue.clone(),
            texts: self.bindings.clone(),
            values: self.value_bindings.clone(),
        }
    }

    /// Добавляет панель; возвращает её узел.
    #[pyo3(signature = (rad=12.0, *, pr=None, ax="v", pd=0.0, gp=0.0, w=None, h=None, elev=0.0))]
    fn fr(
        &mut self,
        rad: f32,
        pr: Option<PyNode>,
        ax: &str,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
        elev: f32,
    ) -> PyResult<PyNode> {
        let props = make_props(ax, pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Frame { radius: rad }, props);
        if elev > 0.0 {
            tree.set_elev(id, elev);
        }
        Ok(PyNode { id })
    }

    /// Панель-контейнер как контекст: `with win.bx(...) as p:`.
    #[pyo3(signature = (rad=12.0, *, pr=None, ax="v", pd=0.0, gp=0.0, w=None, h=None, elev=0.0))]
    fn bx(
        &mut self,
        rad: f32,
        pr: Option<PyNode>,
        ax: &str,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
        elev: f32,
    ) -> PyResult<Ctx> {
        let props = make_props(ax, pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Frame { radius: rad }, props);
        if elev > 0.0 {
            tree.set_elev(id, elev);
        }
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Стопка страниц как контекст; `bind` — колбэк, возвращающий индекс.
    #[pyo3(signature = (*, pr=None, page=0, bind=None, w=None, h=None))]
    fn stk(
        &mut self,
        py: Python,
        pr: Option<PyNode>,
        page: usize,
        bind: Option<PyObject>,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<Ctx> {
        let props = make_props("v", 0.0, 0.0, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<f32>()? as usize,
            None => page,
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Stack { page: initial }, props);
        if let Some(f) = bind {
            self.value_bindings.borrow_mut().push((id, f));
        }
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Разделитель двух областей как контекст; тянется мышью.
    #[pyo3(signature = (*, pr=None, ratio=0.5, vertical=true, w=None, h=None))]
    fn spl(
        &mut self,
        pr: Option<PyNode>,
        ratio: f32,
        vertical: bool,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<Ctx> {
        let props = make_props("v", 0.0, 0.0, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Splitter {
                ratio: ratio.clamp(0.1, 0.9),
                vertical,
            },
            props,
        );
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Секция аккордеона как контекст: `with win.acc("Имя"):`.
    #[pyo3(signature = (title="", *, pr=None, open=false, rad=10.0, pd=8.0, gp=8.0, w=None, h=None))]
    fn acc(
        &mut self,
        title: &str,
        pr: Option<PyNode>,
        open: bool,
        rad: f32,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<Ctx> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Accordion {
                title: utf16(title),
                open,
                radius: rad,
            },
            props,
        );
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Область прокрутки как контекст: `with win.scr():`.
    #[pyo3(signature = (*, pr=None, pd=8.0, gp=8.0, w=None, h=None))]
    fn scr(
        &mut self,
        pr: Option<PyNode>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<Ctx> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Scroll {
                offset: 0.0,
                content: 0.0,
            },
            props,
        );
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Группа с заголовком как контекст: `with win.grp("Имя"):`.
    #[pyo3(signature = (title="", *, pr=None, rad=12.0, ax="v", pd=12.0, gp=8.0, w=None, h=None))]
    fn grp(
        &mut self,
        title: &str,
        pr: Option<PyNode>,
        rad: f32,
        ax: &str,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<Ctx> {
        let props = make_props(ax, pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Group {
                title: utf16(title),
                radius: rad,
            },
            props,
        );
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Добавляет ссылку; `clk` вызывается по нажатию.
    #[pyo3(signature = (lb="", *, pr=None, clk=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn lnk(
        &mut self,
        lb: &str,
        pr: Option<PyNode>,
        clk: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(28.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Link { label: utf16(lb) }, props);
        tree.set_on_click(id, move |t| {
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call0() {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Добавляет метку; `bind` — колбэк, возвращающий текст.
    #[pyo3(signature = (txt="", *, pr=None, bind=None, icon=None, pd=0.0, gp=0.0, w=None, h=None, wrap=false))]
    fn lb(
        &mut self,
        py: Python,
        txt: &str,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        icon: Option<String>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
        wrap: bool,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<String>()?,
            None => txt.to_string(),
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Label {
                text: utf16(&initial),
            },
            props,
        );
        if wrap {
            tree.set_wrap(id, true);
        }
        if let Some(ic) = &icon {
            tree.set_icon(id, ic);
        }
        if let Some(f) = bind {
            self.bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Добавляет изображение; `src_bind` — путь из колбэка.
    #[pyo3(signature = (src="", *, pr=None, src_bind=None, fit="contain", fit_bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    #[allow(clippy::too_many_arguments)]
    fn img(
        &mut self,
        py: Python,
        src: &str,
        pr: Option<PyNode>,
        src_bind: Option<PyObject>,
        fit: &str,
        fit_bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let code = match &fit_bind {
            Some(f) => f.bind(py).call0()?.extract::<f32>()? as u8,
            None => fit_code(fit),
        };
        let path = match &src_bind {
            Some(f) => f.bind(py).call0()?.extract::<String>()?,
            None => src.to_string(),
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Image { path, fit: code }, props);
        if let Some(f) = fit_bind {
            self.value_bindings.borrow_mut().push((id, f));
        }
        if let Some(f) = src_bind {
            self.extra.src.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Добавляет кнопку; `clk` вызывается по нажатию.
    #[pyo3(signature = (lb="", *, pr=None, rad=10.0, icon=None, tip=None, toast=None, pd=0.0, gp=0.0, w=None, h=None, clk=None, elev=0.0))]
    fn bt(
        &mut self,
        lb: &str,
        pr: Option<PyNode>,
        rad: f32,
        icon: Option<String>,
        tip: Option<String>,
        toast: Option<String>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
        clk: Option<PyObject>,
        elev: f32,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Button {
                label: utf16(lb),
                radius: rad,
            },
            props,
        );
        if elev > 0.0 {
            tree.set_elev(id, elev);
        }
        if let Some(ic) = &icon {
            tree.set_icon(id, ic);
        }
        if let Some(tp) = &tip {
            tree.set_tip(id, utf16(tp));
        }
        let toast_msg = toast.map(|s| utf16(&s));
        tree.set_on_click(id, move |t| {
            if let Some(m) = &toast_msg {
                t.push_toast(m.clone(), 2.5);
            }
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call0() {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Добавляет ползунок 0..1; `ch(value)` при перетаскивании.
    #[pyo3(signature = (vl=0.5, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn sl(
        &mut self,
        vl: f32,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Slider { value: vl }, props);
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((v,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Док-панель с заголовком; клик по шапке сворачивает.
    #[pyo3(signature = (ttl="", *, pr=None, side="l", size=260.0, open=true, ax="v", pd=10.0, gp=8.0))]
    fn dock(
        &mut self,
        ttl: &str,
        pr: Option<PyNode>,
        side: &str,
        size: f32,
        open: bool,
        ax: &str,
        pd: f32,
        gp: f32,
    ) -> PyResult<Ctx> {
        let code = match side {
            "r" => 1u8,
            "t" => 2,
            "b" => 3,
            _ => 0,
        };
        let main = if open { size } else { 32.0 };
        let (w, h) = if code == 0 || code == 1 {
            (Some(main), None)
        } else {
            (None, Some(main))
        };
        let props = make_props(ax, pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Dock {
                title: utf16(ttl),
                side: code,
                size,
                open,
            },
            props,
        );
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Зона приёма файлов; `on(paths)` при перетаскивании из проводника.
    #[pyo3(signature = (txt="Перетащите файлы сюда", *, pr=None, on=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn drop(
        &mut self,
        txt: &str,
        pr: Option<PyNode>,
        on: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(160.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Drop { label: utf16(txt) }, props);
        tree.set_on_input(id, move |t, text| {
            let list: Vec<&str> = text.split('\n').filter(|s| !s.is_empty()).collect();
            Python::with_gil(|py| {
                if let Some(cb) = &on {
                    if let Err(e) = cb.bind(py).call1((list,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Терминал; `on(cmd)` при Enter, вывод — возврат строки из `on`.
    #[pyo3(signature = (lines=Vec::new(), *, pr=None, prompt="$", on=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn term(
        &mut self,
        lines: Vec<String>,
        pr: Option<PyNode>,
        prompt: &str,
        on: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(320.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Term {
                lines: lines.iter().map(|s| utf16(s)).collect(),
                input: TextState::new(),
                prompt: utf16(prompt),
                scroll: 0.0,
            },
            props,
        );
        tree.set_on_input(id, move |t, cmd| {
            Python::with_gil(|py| {
                if let Some(cb) = &on {
                    match cb.bind(py).call1((cmd,)) {
                        Ok(out) => {
                            if let Ok(s) = out.extract::<String>() {
                                for line in s.lines() {
                                    t.term_push(id, utf16(line));
                                }
                            }
                        }
                        Err(e) => e.print(py),
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Очищает вывод терминала.
    fn term_clear(&mut self, node: PyNode) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.term_clear(node.id);
        Ok(())
    }

    /// Область рисования; фигуры — `(вид, [args], цвет, текст)`.
    /// Виды: `rect` `[x,y,w,h,rad,stroke]`, `circle` `[cx,cy,r,stroke]`,
    /// `line` `[x1,y1,x2,y2,w]`, `text` `[x,y,w,h]`,
    /// `poly` `[x1,y1,...,xn,yn,stroke]`.
    #[pyo3(signature = (shapes=Vec::new(), *, pr=None, bind=None, ch=None, down=None,
                        r#move=None, up=None, dbl=None, scroll=false,
                        pd=0.0, gp=0.0, w=None, h=None))]
    #[allow(clippy::too_many_arguments)]
    fn cv(
        &mut self,
        py: Python,
        shapes: Vec<ShapeSpec>,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        ch: Option<PyObject>,
        down: Option<PyObject>,
        r#move: Option<PyObject>,
        up: Option<PyObject>,
        dbl: Option<PyObject>,
        scroll: bool,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(280.0));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<Vec<ShapeSpec>>()?,
            None => shapes,
        };
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Canvas {
                shapes: make_shapes(initial),
                ox: 0.0,
                oy: 0.0,
                rx: 0.0,
                ry: 0.0,
                rw: 0.0,
                rh: 0.0,
                scroll,
            },
            props,
        );
        let ct = texts.clone();
        let cvl = values.clone();
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((v as i64,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &ct, &cvl);
            });
        });
        for (phase, cb) in [(0u8, down), (1u8, r#move), (2u8, up), (3u8, dbl)] {
            if let Some(f) = cb {
                tree.set_on_point(id, phase, point_cb(f, texts.clone(), values.clone()));
            }
        }
        if let Some(f) = bind {
            self.extra.canvas.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Задаёт виртуальную область прокрутки области рисования.
    fn cv_region(&self, node: PyNode, x1: f32, y1: f32, x2: f32, y2: f32) -> PyResult<()> {
        self.canvas_queue
            .borrow_mut()
            .push((node.id, 0, x1, y1, x2, y2));
        Ok(())
    }

    /// Прокручивает область рисования к точке содержимого.
    fn cv_view(&self, node: PyNode, x: f32, y: f32) -> PyResult<()> {
        self.canvas_queue
            .borrow_mut()
            .push((node.id, 1, x, y, 0.0, 0.0));
        Ok(())
    }

    /// Задаёт выделенные строки дерева; первая становится текущей.
    fn tre_sel(&self, node: PyNode, indexes: Vec<usize>) -> PyResult<()> {
        self.tree_queue.borrow_mut().push((node.id, 0, indexes));
        Ok(())
    }

    /// Прокручивает дерево к строке, раскрывая её предков.
    fn tre_see(&self, node: PyNode, index: usize) -> PyResult<()> {
        self.tree_queue.borrow_mut().push((node.id, 1, vec![index]));
        Ok(())
    }

    /// Раскрывает или сворачивает поддеревья; пустой список — всё дерево.
    #[pyo3(signature = (node, indexes=Vec::new(), *, on=true))]
    fn tre_open(&self, node: PyNode, indexes: Vec<usize>, on: bool) -> PyResult<()> {
        let op = if on { 2 } else { 3 };
        self.tree_queue.borrow_mut().push((node.id, op, indexes));
        Ok(())
    }

    /// Слой поверх всего окна; содержимое строится внутри `with`.
    #[pyo3(signature = (*, w=240.0, h=40.0, on_close=None))]
    fn pop(&mut self, w: f32, h: f32, on_close: Option<PyObject>) -> PyResult<Ctx> {
        let props = make_props("v", 0.0, 0.0, Some(w), Some(h));
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let root = tree.root();
        let id = tree.add_child(root, NodeKind::Frame { radius: 10.0 }, props);
        tree.set_place(
            id,
            Some(OFF_COORD),
            Some(OFF_COORD),
            None,
            None,
            Some(w),
            Some(h),
        );
        tree.set_depth(id, 1000);
        if let Some(f) = on_close {
            let cb: PointerCb = Box::new(move |t, _, _, _| {
                Python::with_gil(|py| {
                    if let Err(e) = f.bind(py).call0() {
                        e.print(py);
                    }
                    refresh_all(py, t, &texts, &values);
                });
            });
            tree.set_on_point(id, 8, cb);
        }
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Показывает слой в прямоугольнике окна; ноль — оставить размер.
    #[pyo3(signature = (node, x, y, w=0.0, h=0.0))]
    fn pop_at(&self, node: PyNode, x: f32, y: f32, w: f32, h: f32) -> PyResult<()> {
        self.canvas_queue.borrow_mut().push((node.id, 2, x, y, w, h));
        Ok(())
    }

    /// Прячет слой; `on_close` при этом не вызывается.
    fn pop_off(&self, node: PyNode) -> PyResult<()> {
        self.canvas_queue
            .borrow_mut()
            .push((node.id, 3, 0.0, 0.0, 0.0, 0.0));
        Ok(())
    }

    /// Прямоугольник ячейки в координатах окна: `(x, y, w, h)`.
    fn tre_cell(&self, node: PyNode, index: usize, col: usize) -> (f32, f32, f32, f32) {
        let map = self.tree_geom.borrow();
        let Some((rect, head, scroll, bounds, vis)) = map.get(&node.id.index()) else {
            return (0.0, 0.0, 0.0, 0.0);
        };
        let Some(pos) = vis.iter().position(|v| *v == index) else {
            return (0.0, 0.0, 0.0, 0.0);
        };
        let y = rect.y + head + pos as f32 * LIST_ROW - scroll;
        if y + LIST_ROW < rect.y + head || y > rect.y + rect.height {
            return (0.0, 0.0, 0.0, 0.0);
        }
        let (cx, cw) = match bounds.get(col) {
            Some(v) => *v,
            None => (rect.x, rect.width),
        };
        (cx, y, cw, LIST_ROW)
    }

    /// Постраничная навигация; `ch(page)` при смене страницы.
    #[pyo3(signature = (total, *, pr=None, page=0, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn pgn(
        &mut self,
        total: usize,
        pr: Option<PyNode>,
        page: usize,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(44.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let total = total.max(1);
        let id = tree.add_child(
            parent,
            NodeKind::Pager {
                page: page.min(total - 1),
                total,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let i = v.max(0.0) as i64;
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((i,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Оценка звёздами; `ch(value)` при клике.
    #[pyo3(signature = (vl=0, *, pr=None, max=5, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn rat(
        &mut self,
        vl: usize,
        pr: Option<PyNode>,
        max: usize,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(40.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let max = max.max(1);
        let id = tree.add_child(
            parent,
            NodeKind::Rating {
                value: vl.min(max),
                max,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let i = v.max(0.0) as i64;
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((i,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Значок-счётчик; `dot=True` — точка без текста, `bind` — текст.
    #[pyo3(signature = (txt="", *, pr=None, dot=false, bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn bdg(
        &mut self,
        py: Python,
        txt: &str,
        pr: Option<PyNode>,
        dot: bool,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(26.0));
        let w = w.or(Some(if dot { 16.0 } else { 46.0 }));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<String>()?,
            None => txt.to_string(),
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Badge {
                text: utf16(&initial),
                dot,
            },
            props,
        );
        if let Some(f) = bind {
            self.bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Хлебные крошки; клик обрезает путь, `ch(i)`.
    #[pyo3(signature = (items, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn crumb(
        &mut self,
        items: Vec<String>,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(34.0));
        let props = make_props("h", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Crumbs {
                items: items.iter().map(|s| utf16(s)).collect(),
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let i = v.max(0.0) as i64;
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((i,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Возвращает элементы хлебных крошек узла.
    fn crumb_get(&mut self, node: PyNode) -> PyResult<Vec<String>> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(tree
            .crumb_items(node.id)
            .iter()
            .map(|s| String::from_utf16_lossy(s))
            .collect())
    }

    /// Задаёт элементы хлебных крошек узла.
    fn crumb_set(&mut self, node: PyNode, items: Vec<String>) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_crumb_items(node.id, items.iter().map(|s| utf16(s)).collect());
        Ok(())
    }

    /// Выбор времени; клики по половинам, `ch(часы, минуты)`.
    #[pyo3(signature = (hour=12, minute=0, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn tm(
        &mut self,
        hour: u32,
        minute: u32,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(120.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Time {
                hour: hour % 24,
                minute: minute % 60,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let code = v.max(0.0) as i64;
            let (hh, mm) = (code / 100, code % 100);
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((hh, mm)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Таблица свойств «ключ — значение»; `bind` — колбэк списка пар.
    #[pyo3(signature = (rows, *, pr=None, bind=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn pg(
        &mut self,
        py: Python,
        rows: Vec<(String, String)>,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(260.0));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<Vec<(String, String)>>()?,
            None => rows,
        };
        let data = initial
            .iter()
            .map(|(k, v)| (utf16(k), utf16(v)))
            .collect::<Vec<_>>();
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::PropGrid {
                rows: data,
                selected: None,
                scroll: 0.0,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let i = v.max(0.0) as i64;
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((i,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        if let Some(f) = bind {
            self.extra.prop.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Календарь; `ch(год, месяц, день)` при выборе даты.
    #[pyo3(signature = (year=2026, month=7, day=1, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn cal(
        &mut self,
        year: i32,
        month: u32,
        day: u32,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(300.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Calendar {
                year,
                month: month.clamp(1, 12),
                day: day.max(1),
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let code = v.max(0.0) as i64;
            let y = 2000 + code / 10000;
            let m = (code / 100) % 100;
            let d = code % 100;
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((y, m, d)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Палитра цвета (HSV); `ch("#RRGGBB")` при выборе.
    #[pyo3(signature = (hue=0.58, sat=0.75, val=0.96, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn clr(
        &mut self,
        hue: f32,
        sat: f32,
        val: f32,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(220.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Color {
                hue: hue.clamp(0.0, 1.0),
                sat: sat.clamp(0.0, 1.0),
                val: val.clamp(0.0, 1.0),
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            let code = v.max(0.0) as u32;
            let hex = format!("#{:06X}", code);
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((hex.as_str(),)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Круговой регулятор 0..1; тянуть вверх/вниз, `ch(value)`.
    #[pyo3(signature = (vl=0.5, *, pr=None, lb="", ch=None, bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn dl(
        &mut self,
        py: Python,
        vl: f32,
        pr: Option<PyNode>,
        lb: &str,
        ch: Option<PyObject>,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(140.0));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<f32>()?,
            None => vl,
        };
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Dial {
                value: initial.clamp(0.0, 1.0),
                label: utf16(lb),
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((v,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        if let Some(f) = bind {
            self.value_bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Дерево; строка — кортеж `(глубина, текст, лист)` либо словарь.
    #[pyo3(signature = (items, *, pr=None, cols=None, widths=None, multi=false,
                        bind=None, ch=None, clk=None, dbl=None,
                        pd=0.0, gp=0.0, w=None, h=None))]
    #[allow(clippy::too_many_arguments)]
    fn tre(
        &mut self,
        py: Python,
        items: &Bound<'_, PyAny>,
        pr: Option<PyNode>,
        cols: Option<Vec<String>>,
        widths: Option<Vec<f32>>,
        multi: bool,
        bind: Option<PyObject>,
        ch: Option<PyObject>,
        clk: Option<PyObject>,
        dbl: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(300.0));
        let props = make_props("v", pd, gp, w, h);
        let nodes: Vec<TreeItem> = match &bind {
            Some(f) => tree_rows(&f.bind(py).call0()?)?,
            None => tree_rows(items)?,
        };
        let heads: Vec<Vec<u16>> = cols
            .unwrap_or_default()
            .iter()
            .map(|s| utf16(s))
            .collect();
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::TreeView {
                items: nodes,
                selected: None,
                scroll: 0.0,
                cols: heads,
                widths: widths.unwrap_or_default(),
                multi: Vec::new(),
                msel: multi,
            },
            props,
        );
        let (ch_one, ch_many) = if multi { (None, ch) } else { (ch, None) };
        tree.set_on_change(id, move |t, v| {
            let i = v.max(0.0) as i64;
            Python::with_gil(|py| {
                if let Some(cb) = &ch_one {
                    if let Err(e) = cb.bind(py).call1((i,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        if let Some(f) = ch_many {
            let ct = self.bindings.clone();
            let cv = self.value_bindings.clone();
            let cb: PointerCb = Box::new(move |t, _, _, _| {
                let list = t.tree_multi(id);
                Python::with_gil(|py| {
                    if let Err(e) = f.bind(py).call1((list,)) {
                        e.print(py);
                    }
                    refresh_all(py, t, &ct, &cv);
                });
            });
            tree.set_on_point(id, 7, cb);
        }
        if let Some(f) = clk {
            let cb = pair_cb(f, self.bindings.clone(), self.value_bindings.clone());
            tree.set_on_point(id, 5, cb);
        }
        if let Some(f) = dbl {
            let cb = pair_cb(f, self.bindings.clone(), self.value_bindings.clone());
            tree.set_on_point(id, 6, cb);
        }
        if let Some(f) = bind {
            self.extra.tre.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Кнопка с меню; `clk` — основное действие, `ch(i)` — пункт меню.
    #[pyo3(signature = (lb, opts, *, pr=None, clk=None, ch=None, rad=10.0, pd=0.0, gp=0.0, w=None, h=None))]
    fn sbt(
        &mut self,
        lb: &str,
        opts: Vec<String>,
        pr: Option<PyNode>,
        clk: Option<PyObject>,
        ch: Option<PyObject>,
        rad: f32,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(44.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let texts2 = self.bindings.clone();
        let values2 = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Split {
                label: utf16(lb),
                options: opts.iter().map(|s| utf16(s)).collect(),
                radius: rad,
            },
            props,
        );
        tree.set_on_click(id, move |t| {
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call0() {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        tree.set_on_change(id, move |t, v| {
            let i = v.max(0.0) as usize % 1000;
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((i as i64,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts2, &values2);
            });
        });
        Ok(PyNode { id })
    }

    /// Строка меню; `on_select(menu, item)` при выборе пункта.
    #[pyo3(signature = (menus, *, pr=None, on_select=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn mb(
        &mut self,
        menus: Vec<(String, Vec<String>)>,
        pr: Option<PyNode>,
        on_select: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(40.0));
        let props = make_props("h", pd, gp, w, h);
        let titles: Vec<Vec<u16>> = menus.iter().map(|(t, _)| utf16(t)).collect();
        let items: Vec<Vec<Vec<u16>>> = menus
            .iter()
            .map(|(_, its)| its.iter().map(|s| utf16(s)).collect())
            .collect();
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::MenuBar { titles, items }, props);
        tree.set_on_change(id, move |t, v| {
            let idx = v.max(0.0) as usize;
            let (m, i) = (idx / 1000, idx % 1000);
            Python::with_gil(|py| {
                if let Some(cb) = &on_select {
                    if let Err(e) = cb.bind(py).call1((m as i64, i as i64)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Диапазонный ползунок 0..1; `ch(lo, hi)` при перетаскивании.
    #[pyo3(signature = (lo=0.25, hi=0.75, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn rsl(
        &mut self,
        lo: f32,
        hi: f32,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(36.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Range {
                lo: lo.min(hi).clamp(0.0, 1.0),
                hi: lo.max(hi).clamp(0.0, 1.0),
            },
            props,
        );
        tree.set_on_change(id, move |t, _v| {
            let (a, b) = t.range_values(id);
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((a, b)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Строка состояния; `bind` — колбэк текста.
    #[pyo3(signature = (txt="", *, pr=None, bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn stb(
        &mut self,
        py: Python,
        txt: &str,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(32.0));
        let props = make_props("h", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<String>()?,
            None => txt.to_string(),
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Status {
                text: utf16(&initial),
            },
            props,
        );
        if let Some(f) = bind {
            self.bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Сегментная шкала 0..1; `bind` — колбэк значения.
    #[pyo3(signature = (vl=0.0, *, pr=None, bind=None, seg=10, pd=0.0, gp=0.0, w=None, h=None))]
    fn mt(
        &mut self,
        py: Python,
        vl: f32,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        seg: usize,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(28.0));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<f32>()?,
            None => vl,
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Meter {
                value: initial.clamp(0.0, 1.0),
                segments: seg.max(1),
            },
            props,
        );
        if let Some(f) = bind {
            self.value_bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Столбчатая диаграмма; `bind` — колбэк списка значений.
    #[pyo3(signature = (data, *, pr=None, bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn cht(
        &mut self,
        py: Python,
        data: Vec<f32>,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(200.0));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<Vec<f32>>()?,
            None => data,
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Chart { values: initial }, props);
        if let Some(f) = bind {
            self.extra.chart.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Добавляет вращающийся индикатор загрузки.
    #[pyo3(signature = (*, pr=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn spn(
        &mut self,
        pr: Option<PyNode>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(48.0));
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Spinner { phase: 0.0 }, props);
        Ok(PyNode { id })
    }

    /// Круговой индикатор; `bind` — колбэк значения 0..1.
    #[pyo3(signature = (value=0.0, *, pr=None, lb="", bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn gg(
        &mut self,
        py: Python,
        value: f32,
        pr: Option<PyNode>,
        lb: &str,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(140.0));
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<f32>()?,
            None => value,
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Gauge {
                value: initial.clamp(0.0, 1.0),
                label: utf16(lb),
            },
            props,
        );
        if let Some(f) = bind {
            self.value_bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Добавляет индикатор прогресса; `bind` — колбэк значения 0..1.
    #[pyo3(signature = (vl=0.0, *, pr=None, bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn pr(
        &mut self,
        py: Python,
        vl: f32,
        pr: Option<PyNode>,
        bind: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let initial = match &bind {
            Some(f) => f.bind(py).call0()?.extract::<f32>()?,
            None => vl,
        };
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Progress { value: initial }, props);
        if let Some(f) = bind {
            self.value_bindings.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Добавляет флажок; `clk` вызывается после переключения.
    #[pyo3(signature = (lb="", *, pr=None, chk=false, clk=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn ch(
        &mut self,
        lb: &str,
        pr: Option<PyNode>,
        chk: bool,
        clk: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Checkbox {
                label: utf16(lb),
                checked: chk,
            },
            props,
        );
        tree.set_on_click(id, move |t| {
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call0() {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Добавляет переключатель; `clk(on)` при смене состояния.
    #[pyo3(signature = (lb="", *, pr=None, on=false, clk=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn sw(
        &mut self,
        lb: &str,
        pr: Option<PyNode>,
        on: bool,
        clk: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Switch { label: utf16(lb), on }, props);
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call1((v >= 0.5,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Возвращает состояние переключателя.
    fn swv(&self, n: PyNode) -> PyResult<bool> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(tree.switch_on(n.id))
    }

    /// Добавляет радиокнопку группы `grp`; `clk()` при выборе.
    #[pyo3(signature = (lb="", *, pr=None, grp=0, on=false, clk=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn rd(
        &mut self,
        lb: &str,
        pr: Option<PyNode>,
        grp: u32,
        on: bool,
        clk: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Radio { label: utf16(lb), on, group: grp },
            props,
        );
        tree.set_on_change(id, move |t, _v| {
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call0() {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Возвращает состояние радиокнопки.
    fn rdv(&self, n: PyNode) -> PyResult<bool> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(tree.radio_on(n.id))
    }

    /// Добавляет кнопку-переключатель; `clk(on)` при смене состояния.
    #[pyo3(signature = (lb="", *, pr=None, on=false, clk=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn tgl(
        &mut self,
        lb: &str,
        pr: Option<PyNode>,
        on: bool,
        clk: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Toggle { label: utf16(lb), on }, props);
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &clk {
                    if let Err(e) = cb.bind(py).call1((v >= 0.5,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Возвращает состояние кнопки-переключателя.
    fn tglv(&self, n: PyNode) -> PyResult<bool> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(tree.toggle_on(n.id))
    }

    /// Добавляет разделитель; `vertical` — вертикальная линия.
    #[pyo3(signature = (*, pr=None, vertical=false, pd=0.0, gp=0.0, w=None, h=None))]
    fn sep(
        &mut self,
        pr: Option<PyNode>,
        vertical: bool,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let (w2, h2) = if vertical {
            (w.or(Some(12.0)), h)
        } else {
            (w, h.or(Some(12.0)))
        };
        let props = make_props("v", pd, gp, w2, h2);
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::Separator { vertical }, props);
        Ok(PyNode { id })
    }

    /// Добавляет поле ввода; `sig` — сигнал текста, `ph` — подсказка пустого поля.
    #[pyo3(signature = (txt="", *, pr=None, sig=None, ph="", pd=0.0, gp=0.0, w=None, h=None))]
    fn tx(
        &mut self,
        txt: &str,
        pr: Option<PyNode>,
        sig: Option<PyObject>,
        ph: &str,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("v", pd, gp, w, h);
        let mut st = TextState::new();
        if !txt.is_empty() {
            st.text = utf16(txt);
            st.caret = st.text.len();
            st.anchor = st.caret;
        }
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::TextBox { state: st }, props);
        if !ph.is_empty() {
            tree.set_placeholder(id, utf16(ph));
        }
        if let Some(sig) = sig {
            tree.set_on_input(id, move |t, text| {
                Python::with_gil(|py| {
                    if let Err(e) = sig.bind(py).call_method1("st", (text,)) {
                        e.print(py);
                    }
                    refresh_all(py, t, &texts, &values);
                });
            });
        }
        Ok(PyNode { id })
    }

    /// Многострочное поле ввода; Enter — перенос, `ph` — подсказка пустого поля.
    #[pyo3(signature = (txt="", *, pr=None, sig=None, ph="", pd=0.0, gp=0.0, w=None, h=None))]
    fn ta(
        &mut self,
        txt: &str,
        pr: Option<PyNode>,
        sig: Option<PyObject>,
        ph: &str,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(120.0));
        let props = make_props("v", pd, gp, w, h);
        let mut st = TextState::new();
        if !txt.is_empty() {
            st.text = utf16(txt);
            st.caret = st.text.len();
            st.anchor = st.caret;
        }
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent, NodeKind::TextBox { state: st }, props);
        tree.set_multiline(id);
        if !ph.is_empty() {
            tree.set_placeholder(id, utf16(ph));
        }
        if let Some(sig) = sig {
            tree.set_on_input(id, move |t, text| {
                Python::with_gil(|py| {
                    if let Err(e) = sig.bind(py).call_method1("st", (text,)) {
                        e.print(py);
                    }
                    refresh_all(py, t, &texts, &values);
                });
            });
        }
        Ok(PyNode { id })
    }

    /// Числовое поле с кнопками −/+; `ch(value)` при изменении.
    #[pyo3(signature = (value=0.0, *, pr=None, min=0.0, max=100.0, step=1.0, ch=None, pd=0.0, gp=6.0, w=None, h=None))]
    fn spin(
        &mut self,
        value: f32,
        pr: Option<PyNode>,
        min: f32,
        max: f32,
        step: f32,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let bh = h.or(Some(44.0));
        let props = make_props("h", pd, gp, w, bh);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let box_id = tree.add_child(parent, NodeKind::Container, props);

        let minus = tree.add_child(
            box_id,
            NodeKind::Button { label: utf16("−"), radius: 8.0 },
            make_props("v", 0.0, 0.0, Some(44.0), bh),
        );
        let lbl = tree.add_child(
            box_id,
            NodeKind::Label {
                text: utf16(&fmt_num(value.clamp(min, max), step)),
            },
            make_props("v", 0.0, 0.0, None, bh),
        );
        tree.set_grow(lbl, 1.0);
        let plus = tree.add_child(
            box_id,
            NodeKind::Button { label: utf16("+"), radius: 8.0 },
            make_props("v", 0.0, 0.0, Some(44.0), bh),
        );

        let cur = std::rc::Rc::new(std::cell::Cell::new(value.clamp(min, max)));
        let cb_rc = std::rc::Rc::new(ch);

        let c1 = cur.clone();
        let ch1 = cb_rc.clone();
        let t1 = texts.clone();
        let v1 = values.clone();
        tree.set_on_click(minus, move |t| {
            let nv = (c1.get() - step).clamp(min, max);
            c1.set(nv);
            t.set_label_text(lbl, utf16(&fmt_num(nv, step)));
            Python::with_gil(|py| {
                if let Some(cb) = ch1.as_ref() {
                    if let Err(e) = cb.bind(py).call1((nv,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &t1, &v1);
            });
        });

        let c2 = cur.clone();
        let ch2 = cb_rc.clone();
        let t2 = texts.clone();
        let v2 = values.clone();
        tree.set_on_click(plus, move |t| {
            let nv = (c2.get() + step).clamp(min, max);
            c2.set(nv);
            t.set_label_text(lbl, utf16(&fmt_num(nv, step)));
            Python::with_gil(|py| {
                if let Some(cb) = ch2.as_ref() {
                    if let Err(e) = cb.bind(py).call1((nv,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &t2, &v2);
            });
        });

        Ok(PyNode { id: box_id })
    }

    /// Возвращает текущий текст поля ввода.
    fn tv(&self, n: PyNode) -> PyResult<String> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(tree.textbox_text(n.id).unwrap_or_default())
    }

    /// Добавляет выпадающий список; `ch(index)` при выборе.
    #[pyo3(signature = (options, *, pr=None, sel=0, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn dd(
        &mut self,
        options: Vec<String>,
        pr: Option<PyNode>,
        sel: usize,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let opts: Vec<Vec<u16>> = options.iter().map(|s| utf16(s)).collect();
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Dropdown {
                options: opts,
                selected: sel,
                open: false,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((v as i64,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(PyNode { id })
    }

    /// Добавляет вкладки как контекст; вложенные контейнеры — их содержимое.
    #[pyo3(signature = (labels, *, pr=None, sel=0, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn tab(
        &mut self,
        labels: Vec<String>,
        pr: Option<PyNode>,
        sel: usize,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<Ctx> {
        let labs: Vec<Vec<u16>> = labels.iter().map(|s| utf16(s)).collect();
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Tabs {
                labels: labs,
                selected: sel,
            },
            props,
        );
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &ch {
                    if let Err(e) = cb.bind(py).call1((v as i64,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(Ctx {
            stack: self.stack.clone(),
            node: id,
        })
    }

    /// Задаёт всплывающую подсказку узла.
    fn tip(&mut self, n: PyNode, text: &str) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_tip(n.id, utf16(text));
        Ok(())
    }

    /// Присваивает узлу CSS-класс.
    fn cls(&mut self, n: PyNode, name: &str) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_class(n.id, Some(name.to_string()));
        Ok(())
    }

    /// Размещает узел абсолютно: `x`, `y`, `r`, `b`, `w`, `h`.
    #[pyo3(signature = (n, *, x=None, y=None, r=None, b=None, w=None, h=None))]
    fn pl(
        &mut self,
        n: PyNode,
        x: Option<f32>,
        y: Option<f32>,
        r: Option<f32>,
        b: Option<f32>,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_place(n.id, x, y, r, b, w, h);
        Ok(())
    }

    /// Ставит узел в ячейку сетки: строка, столбец, растяжки.
    #[pyo3(signature = (n, r=0, c=0, *, rs=1, cs=1))]
    fn gr(&mut self, n: PyNode, r: u8, c: u8, rs: u8, cs: u8) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_grid(n.id, r, c, rs, cs);
        Ok(())
    }

    /// Прижимает узел к стороне: `t`/`b`/`l`/`r`, `fill`, `exp`.
    #[pyo3(signature = (n, side="t", *, fill=None, exp=false))]
    fn pk(&mut self, n: PyNode, side: &str, fill: Option<&str>, exp: bool) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_pack(n.id, side_code(side), fill_mask(fill), exp);
        Ok(())
    }

    /// Задаёт глубину узла: больше `z` — ближе к зрителю (`dep(node, 2)`).
    #[pyo3(signature = (n, z=0))]
    fn dep(&mut self, n: PyNode, z: i32) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_depth(n.id, z);
        Ok(())
    }

    /// Применяет CSS-подмножество из строки.
    fn css(&mut self, text: &str) -> PyResult<()> {
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.apply_css(text);
        Ok(())
    }

    /// Применяет CSS из файла по пути.
    fn css_file(&mut self, path: &str) -> PyResult<()> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.apply_css(&text);
        Ok(())
    }

    /// Следит за CSS-файлом и перезагружает его на лету.
    fn css_hot(&mut self, path: &str) -> PyResult<()> {
        let full = std::fs::canonicalize(path)
            .map_err(|e| PyRuntimeError::new_err(format!("{path}: {e}")))?;
        let full = full.to_string_lossy().to_string();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.css_watch(&full);
        Ok(())
    }

    /// Добавляет список; `ch` — индекс, при `multi` — список индексов.
    #[pyo3(signature = (items, *, pr=None, sel=None, multi=false, ch=None,
                        pd=0.0, gp=0.0, w=None, h=None))]
    #[allow(clippy::too_many_arguments)]
    fn lst(
        &mut self,
        py: Python,
        items: Vec<String>,
        pr: Option<PyNode>,
        sel: Option<PyObject>,
        multi: bool,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(240.0));
        let props = make_props("v", pd, gp, w, h);
        let its: Vec<Vec<u16>> = items.iter().map(|s| utf16(s)).collect();
        let picks: Vec<usize> = match &sel {
            Some(o) => {
                let b = o.bind(py);
                match b.extract::<usize>() {
                    Ok(i) => vec![i],
                    Err(_) => b.extract::<Vec<usize>>().unwrap_or_default(),
                }
            }
            None => Vec::new(),
        };
        let first = picks.first().copied();
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::List {
                items: its,
                selected: first,
                scroll: 0.0,
                multi: Vec::new(),
                msel: multi,
            },
            props,
        );
        if multi && !picks.is_empty() {
            tree.set_list_multi(id, picks);
        }
        let (ch_one, ch_many) = if multi { (None, ch) } else { (ch, None) };
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &ch_one {
                    if let Err(e) = cb.bind(py).call1((v as i64,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        if let Some(f) = ch_many {
            let ct = self.bindings.clone();
            let cv = self.value_bindings.clone();
            let cb: PointerCb = Box::new(move |t, _, _, _| {
                let list = t.list_multi(id);
                Python::with_gil(|py| {
                    if let Err(e) = f.bind(py).call1((list,)) {
                        e.print(py);
                    }
                    refresh_all(py, t, &ct, &cv);
                });
            });
            tree.set_on_point(id, 7, cb);
        }
        Ok(PyNode { id })
    }

    /// Возвращает выбранный пункт списка или -1.
    fn lstv(&self, n: PyNode) -> PyResult<i64> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(tree.list_selected(n.id).map_or(-1, |i| i as i64))
    }

    /// Задаёт выделенные пункты списка; первый становится текущим.
    fn lst_sel(&self, node: PyNode, indexes: Vec<usize>) -> PyResult<()> {
        self.tree_queue.borrow_mut().push((node.id, 5, indexes));
        Ok(())
    }

    /// Регистрирует горячую клавишу окна; молчит при фокусе в поле ввода.
    fn hotkey(&mut self, spec: &str, f: PyObject) -> PyResult<()> {
        let (mods, vk) = parse_hotkey(spec)
            .ok_or_else(|| PyRuntimeError::new_err(format!("плохой хоткей: {spec}")))?;
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.add_hotkey(mods, vk, move |t| {
            Python::with_gil(|py| {
                if let Err(e) = f.bind(py).call0() {
                    e.print(py);
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(())
    }

    /// Возвращает контроллер буфера обмена.
    fn clip(&self) -> Clip {
        Clip
    }

    /// Возвращает контроллер файловых диалогов.
    fn file(&self) -> Fl {
        Fl {
            queue: self.file_queue.clone(),
            share: self.share.clone(),
            texts: self.bindings.clone(),
            values: self.value_bindings.clone(),
        }
    }

    /// Правый клик по узлу; `f(x, y)` в координатах окна.
    fn rmb(&mut self, node: PyNode, f: PyObject) -> PyResult<()> {
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let cb: PointerCb = Box::new(move |t, _, x, y| {
            Python::with_gil(|py| {
                if let Err(e) = f.bind(py).call1((x, y)) {
                    e.print(py);
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        tree.set_on_point(node.id, 9, cb);
        Ok(())
    }

    /// Размер основного экрана в пикселях: `(ширина, высота)`.
    #[staticmethod]
    fn screen() -> (f32, f32) {
        CoreWindow::screen()
    }

    /// Размер клиентской области окна: `(ширина, высота)`.
    fn size(&self) -> (f32, f32) {
        CoreWindow::client_size(self.hwnd.get())
    }

    /// Перемещает окно; `(x, y)` — левый верхний угол на экране.
    #[pyo3(name = "move")]
    fn move_win(&self, x: f32, y: f32) -> PyResult<()> {
        CoreWindow::move_to(self.hwnd.get(), x, y);
        Ok(())
    }

    /// Колбэк изменения размера окна; `f(width, height)` клиентской области.
    fn on_resize(&mut self, f: PyObject) -> PyResult<()> {
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_resize_cb(move |t, w, h| {
            Python::with_gil(|py| {
                if let Err(e) = f.bind(py).call1((w, h)) {
                    e.print(py);
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(())
    }

    /// Добавляет таблицу; `hl`/`vl` — толщина разделителей строк и столбцов.
    #[pyo3(signature = (columns, rows, *, pr=None, ch=None, bg=None, hl=0.0, vl=0.0,
                        pd=0.0, gp=0.0, w=None, h=None))]
    #[allow(clippy::too_many_arguments)]
    fn tbl(
        &mut self,
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
        bg: Option<PyObject>,
        hl: f32,
        vl: f32,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let cols: Vec<Vec<u16>> = columns.iter().map(|s| utf16(s)).collect();
        let rws: Vec<Vec<Vec<u16>>> = rows
            .iter()
            .map(|row| row.iter().map(|c| utf16(c)).collect())
            .collect();
        let props = make_props("v", pd, gp, w, h);
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Table {
                columns: cols,
                rows: rws,
                selected: None,
                scroll: 0.0,
                hline: hl,
                vline: vl,
                cbg: Vec::new(),
            },
            props,
        );
        let two = ch
            .as_ref()
            .map(|f| Python::with_gil(|py| takes_two(py, f)))
            .unwrap_or(false);
        let (ch_one, ch_two) = if two { (None, ch) } else { (ch, None) };
        tree.set_on_change(id, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &ch_one {
                    if let Err(e) = cb.bind(py).call1((v as i64,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        if let Some(f) = ch_two {
            let cb = pair_cb(f, self.bindings.clone(), self.value_bindings.clone());
            tree.set_on_point(id, 5, cb);
        }
        if let Some(f) = bg {
            self.extra.tbg.borrow_mut().push((id, f));
        }
        Ok(PyNode { id })
    }

    /// Прокручивает таблицу к строке.
    fn tbl_see(&self, node: PyNode, row: usize) -> PyResult<()> {
        self.tree_queue.borrow_mut().push((node.id, 4, vec![row]));
        Ok(())
    }

    /// Задаёт контекстное меню окна; `on_select(index)` по ПКМ.
    #[pyo3(signature = (items, *, on_select=None))]
    fn menu(&mut self, items: Vec<String>, on_select: Option<PyObject>) -> PyResult<()> {
        let its: Vec<Vec<u16>> = items.iter().map(|s| utf16(s)).collect();
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        tree.set_menu(its);
        let root = tree.root();
        tree.set_on_change(root, move |t, v| {
            Python::with_gil(|py| {
                if let Some(cb) = &on_select {
                    if let Err(e) = cb.bind(py).call1((v as i64,)) {
                        e.print(py);
                    }
                }
                refresh_all(py, t, &texts, &values);
            });
        });
        Ok(())
    }

    /// Показывает окно и запускает цикл сообщений.
    fn go(slf: &Bound<'_, Self>) -> PyResult<()> {
        let (tree, title, width, height, opts, cell, share) = {
            let mut me = slf.borrow_mut();
            let tree = me.tree.take().ok_or_else(consumed)?;
            (
                tree,
                me.title.clone(),
                me.width,
                me.height,
                me.opts(),
                me.hwnd.clone(),
                me.share.clone(),
            )
        };
        dpi::enable_dpi_awareness();
        let window = CoreWindow::with_opts(&title, width, height, tree, opts)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        cell.set(window.handle());
        share.hwnd.store(window.handle(), Ordering::Release);
        window.mark_main();
        slf.py().allow_threads(|| CoreWindow::loop_messages());
        share.alive.store(false, Ordering::Release);
        Ok(())
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    #[pyo3(signature = (_t=None, _v=None, _tb=None))]
    fn __exit__(
        slf: &Bound<'_, Self>,
        _t: Option<PyObject>,
        _v: Option<PyObject>,
        _tb: Option<PyObject>,
    ) -> PyResult<bool> {
        PyWindow::show(slf)?;
        Ok(false)
    }
}

impl PyWindow {
    /// Кладёт таймер в очередь и будит цикл; возвращает идентификатор.
    fn push_timer(&self, ms: f32, once: bool, f: PyObject) -> u64 {
        let id = TIMER_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        self.timer_queue.borrow_mut().push(TimerReq {
            id,
            ms,
            once,
            cb: Box::new(move |t| {
                Python::with_gil(|py| {
                    if let Err(e) = f.bind(py).call0() {
                        e.print(py);
                    }
                    refresh_all(py, t, &texts, &values);
                });
            }),
        });
        CoreWindow::wake(self.share.hwnd.load(Ordering::Acquire));
        id
    }

    /// Параметры создания окна для ядра.
    fn opts(&self) -> WindowOpts {
        WindowOpts {
            glass: self.glass,
            tint: self.tint,
            blur: self.blur,
            frameless: self.frameless,
            topmost: self.topmost,
            center: self.center,
            resizable: self.resizable,
            minbox: self.minbox,
            maxbox: self.maxbox,
            closebox: self.closebox,
            owner: if self.owner == 0 {
                None
            } else {
                Some(self.owner)
            },
            modal: self.modal,
            icon: self.icon.clone(),
            caption: self.caption,
            caption_text: self.caption_text,
            border: self.border,
            dark: self.dark,
            on_close: None,
        }
    }

    fn colorref(hex: Option<&str>) -> Option<u32> {
        let s = hex?.trim_start_matches('#');
        let v = u32::from_str_radix(&s[..6.min(s.len())], 16).ok()?;
        Some(((v & 0xFF) << 16) | (v & 0xFF00) | ((v >> 16) & 0xFF))
    }

    fn parent_of(&self, pr: Option<PyNode>) -> NodeId {
        match pr {
            Some(p) => p.id,
            None => *self.stack.borrow().last().unwrap_or(&self.root),
        }
    }
}

#[pyclass(name = "S")]
struct Signal {
    id: u32,
    value: PyObject,
}

#[pymethods]
impl Signal {
    #[new]
    fn new(vl: PyObject) -> Self {
        Self {
            id: next_sig_id(),
            value: vl,
        }
    }

    fn __call__(&self, py: Python) -> PyObject {
        mark_read(self.id);
        self.value.clone_ref(py)
    }

    /// Возвращает текущее значение.
    fn gt(&self, py: Python) -> PyObject {
        mark_read(self.id);
        self.value.clone_ref(py)
    }

    /// Устанавливает новое значение.
    fn st(&mut self, vl: PyObject) {
        self.value = vl;
        mark_dirty(self.id);
    }
}

/// Создаёт сигнал с начальным значением.
#[pyfunction]
fn sgnl(vl: PyObject) -> Signal {
    Signal::new(vl)
}

fn consumed() -> PyErr {
    PyRuntimeError::new_err("окно уже запущено")
}

fn set_win_setting(tag: u8, sig: PyObject) {
    WIN_SETTINGS.with(|s| {
        let mut v = s.borrow_mut();
        v.retain(|(t, _)| *t != tag);
        v.push((tag, sig));
    });
}

fn make_props(ax: &str, pd: f32, gp: f32, w: Option<f32>, h: Option<f32>) -> Props {
    Props {
        axis: parse_axis(ax),
        padding: pd,
        gap: gp,
        width: w,
        height: h,
        ..Default::default()
    }
}

fn justify_code(s: &str) -> u8 {
    match s {
        "cnt" => 1,
        "end" => 2,
        "btw" => 3,
        _ => 0,
    }
}

fn cross_code(s: &str) -> u8 {
    match s {
        "st" => 1,
        "cnt" => 2,
        "end" => 3,
        _ => 0,
    }
}

fn side_code(s: &str) -> u8 {
    match s {
        "b" => 1,
        "l" => 2,
        "r" => 3,
        _ => 0,
    }
}

fn fill_mask(s: Option<&str>) -> u8 {
    match s {
        Some("x") => 1,
        Some("y") => 2,
        Some("both") | Some("xy") => 3,
        _ => 0,
    }
}

fn parse_axis(s: &str) -> Axis {
    match s {
        "h" => Axis::Horizontal,
        _ => Axis::Vertical,
    }
}

fn fit_code(s: &str) -> u8 {
    match s {
        "cover" => 1,
        "fill" | "stretch" => 2,
        "center" | "none" => 3,
        _ => 0,
    }
}

fn fmt_num(v: f32, step: f32) -> String {
    if (step - step.round()).abs() < 1e-6 {
        format!("{}", v.round() as i64)
    } else {
        format!("{:.1}", v)
    }
}

fn parse_ease(s: &str) -> Ease {
    match s {
        "lin" => Ease::Linear,
        "in" => Ease::In,
        "io" => Ease::InOut,
        _ => Ease::Out,
    }
}

fn theme_index(name: &str) -> usize {
    match name {
        "wht" => 0,
        "lit" => 1,
        "blk" => 3,
        _ => 2,
    }
}

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// Оборачивает питоновский колбэк указателя `f(i, x, y)`.
fn point_cb(f: PyObject, texts: Bindings, values: Bindings) -> PointerCb {
    Box::new(move |t, i, x, y| {
        Python::with_gil(|py| {
            if let Err(e) = f.bind(py).call1((i, x, y)) {
                e.print(py);
            }
            refresh_all(py, t, &texts, &values);
        });
    })
}

fn refresh_all(py: Python, t: &mut Tree, texts: &Bindings, values: &Bindings) {
    let _guard = ExtraGuard::enter(texts);
    let cur = CUR_EXTRA.with(|c| c.borrow().clone());
    if let Some(e) = &cur {
        for (id, f) in e.pos.borrow().iter() {
            let Some(r) = run_binding(py, f) else { continue };
            match r.extract::<(f32, f32, f32, f32)>() {
                Ok((x, y, w, h)) => {
                    t.set_place(*id, Some(x), Some(y), None, None, Some(w), Some(h))
                }
                Err(e) => e.print(py),
            }
        }
        for (id, f) in e.src.borrow().iter() {
            let Some(r) = run_binding(py, f) else { continue };
            match r.extract::<String>() {
                Ok(p) => t.set_image_path(*id, &p),
                Err(e) => e.print(py),
            }
        }
    }
    for (id, f) in texts.borrow().iter() {
        let Some(r) = run_binding(py, f) else { continue };
        match r.extract::<String>() {
            Ok(s) => t.set_label_text(*id, utf16(&s)),
            Err(e) => e.print(py),
        }
    }
    for (id, f) in values.borrow().iter() {
        let Some(r) = run_binding(py, f) else { continue };
        match r.extract::<f32>() {
            Ok(v) => {
                t.set_slider_value(*id, v);
                t.set_progress_value(*id, v);
                t.set_gauge_value(*id, v);
                t.set_meter_value(*id, v);
                t.set_dial_value(*id, v);
                t.set_stack_page(*id, v.max(0.0) as usize);
                t.set_image_fit(*id, v as u8);
                if t.is_stack(*id) {
                    t.set_stack_page(*id, v.max(0.0) as usize);
                }
            }
            Err(e) => e.print(py),
        }
    }
    DEPTH_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            let Some(r) = run_binding(py, f) else { continue };
            match r.extract::<f32>() {
                Ok(v) => t.set_depth(*id, v as i32),
                Err(e) => e.print(py),
            }
        }
    });
    BOX_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            let Some(r) = run_binding(py, f) else { continue };
            match r.extract::<(f32, f32)>() {
                Ok((pd, gp)) => t.set_box(*id, pd, gp),
                Err(e) => e.print(py),
            }
        }
    });
    LIST_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            let Some(r) = run_binding(py, f) else { continue };
            match r.extract::<Vec<String>>() {
                Ok(rows) => {
                    let data = rows.iter().map(|s| utf16(s)).collect();
                    t.set_list_items(*id, data);
                }
                Err(e) => e.print(py),
            }
        }
    });
    TABLE_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            let Some(r) = run_binding(py, f) else { continue };
            match r.extract::<Vec<Vec<String>>>() {
                Ok(rows) => {
                    let data = rows
                        .iter()
                        .map(|r| r.iter().map(|c| utf16(c)).collect())
                        .collect();
                    t.set_table_rows(*id, data);
                }
                Err(e) => e.print(py),
            }
        }
    });
    CHART_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            let Some(r) = run_binding(py, f) else { continue };
            match r.extract::<Vec<f32>>() {
                Ok(v) => t.set_chart_values(*id, v),
                Err(e) => e.print(py),
            }
        }
    });
    CANVAS_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            let Some(r) = run_binding(py, f) else { continue };
            match r.extract::<Vec<ShapeSpec>>() {
                Ok(items) => t.set_canvas_shapes(*id, make_shapes(items)),
                Err(e) => e.print(py),
            }
        }
    });
    PROP_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            let Some(r) = run_binding(py, f) else { continue };
            match r.extract::<Vec<(String, String)>>() {
                Ok(rows) => {
                    let data = rows
                        .iter()
                        .map(|(k, v)| (utf16(k), utf16(v)))
                        .collect::<Vec<_>>();
                    t.set_prop_rows(*id, data);
                }
                Err(e) => e.print(py),
            }
        }
    });
    TREE_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            let Some(r) = run_binding(py, f) else { continue };
            match tree_rows(&r) {
                Ok(rows) => t.set_tree_items(*id, rows),
                Err(e) => e.print(py),
            }
        }
    });
    TBG_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            let Some(r) = run_binding(py, f) else { continue };
            match r.extract::<Vec<((usize, usize), String)>>() {
                Ok(list) => {
                    let data = list.iter().map(|(k, s)| (*k, hexa(s))).collect();
                    t.set_table_cbg(*id, data);
                }
                Err(e) => e.print(py),
            }
        }
    });
    WIN_SETTINGS.with(|s| {
        for (tag, f) in s.borrow().iter() {
            let Some(r) = run_binding(py, f) else { continue };
            let v = match r.extract::<f32>() {
                Ok(v) => v,
                Err(e) => {
                    e.print(py);
                    continue;
                }
            };
            match tag {
                0 => t.set_tint(v),
                1 => {
                    if v <= 0.001 {
                        t.set_blur_mode(0);
                    } else {
                        let a = (v.clamp(0.0, 1.0) * 255.0) as u32;
                        t.set_blur_mode(3);
                        t.set_blur_tint((a << 24) | 0x101418);
                    }
                }
                2 => {
                    let m = if v as i32 <= 0 { 0u32 } else { 3u32 };
                    t.set_blur_mode(m);
                }
                _ => t.set_drag_smooth(v >= 0.5),
            }
        }
    });
    DIRTY.with(|s| s.borrow_mut().clear());
}

#[pymodule]
fn _ssui(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWindow>()?;
    m.add_class::<PyNode>()?;
    m.add_class::<Ctx>()?;
    m.add_class::<Fx>()?;
    m.add_class::<Fnt>()?;
    m.add_class::<Post>()?;
    m.add_class::<Rct>()?;
    m.add_class::<Dlg>()?;
    m.add_class::<Signal>()?;
    m.add_function(wrap_pyfunction!(sgnl, m)?)?;
    Ok(())
}
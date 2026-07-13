use std::cell::RefCell;
use std::rc::Rc;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use ssui_core::platform::{dpi, Window as CoreWindow};
use ssui_core::tree::{
    Anim, AnimQueue, Axis, DialogData, DialogQueue, Ease, NodeId, NodeKind, Props, TextState, Tree,
};

#[pyclass(name = "N")]
#[derive(Clone, Copy)]
struct PyNode {
    id: NodeId,
}

type Bindings = Rc<RefCell<Vec<(NodeId, Py<PyAny>)>>>;
type Stack = Rc<RefCell<Vec<NodeId>>>;

thread_local! {
    static WIN_SETTINGS: RefCell<Vec<(u8, Py<PyAny>)>> = RefCell::new(Vec::new());
    static CHART_BINDINGS: RefCell<Vec<(NodeId, Py<PyAny>)>> = RefCell::new(Vec::new());
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
    glass: bool,
    tint: f32,
    blur: bool,
}

#[pymethods]
impl PyWindow {
    #[new]
    #[pyo3(signature = (ttl="SSUI", w=1280, h=720, thm="drk", glass=false, tint=0.0, blur=false))]
    fn new(ttl: &str, w: i32, h: i32, thm: &str, glass: bool, tint: f32, blur: bool) -> Self {
        let mut tree = Tree::new();
        tree.set_theme(theme_index(thm));
        let root = tree.root();
        let anim_queue = tree.anim_queue();
        let dialog_queue = tree.dialog_queue();
        Self {
            tree: Some(tree),
            title: ttl.to_string(),
            width: w,
            height: h,
            root,
            stack: Rc::new(RefCell::new(Vec::new())),
            bindings: Rc::new(RefCell::new(Vec::new())),
            value_bindings: Rc::new(RefCell::new(Vec::new())),
            anim_queue,
            dialog_queue,
            glass,
            tint,
            blur,
        }
    }

    /// Возвращает корневой узел окна.
    fn rt(&self) -> PyNode {
        PyNode { id: self.root }
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

    /// Привязывает прозрачность фона окна к сигналу 0..1.
    fn tint(&mut self, sig: PyObject) -> PyResult<()> {
        WIN_SETTINGS.with(|s| s.borrow_mut().push((0, sig)));
        Ok(())
    }

    /// Привязывает силу размытия к сигналу 0..1 (0 — выключено).
    fn blur(&mut self, sig: PyObject) -> PyResult<()> {
        WIN_SETTINGS.with(|s| s.borrow_mut().push((1, sig)));
        Ok(())
    }

    /// Привязывает режим фона к сигналу: 0 — нет, иначе — размытие.
    fn blur_mode(&mut self, sig: PyObject) -> PyResult<()> {
        WIN_SETTINGS.with(|s| s.borrow_mut().push((2, sig)));
        Ok(())
    }

    /// Привязывает гашение размытия при перемещении к сигналу (0/1).
    fn drag_smooth(&mut self, sig: PyObject) -> PyResult<()> {
        WIN_SETTINGS.with(|s| s.borrow_mut().push((3, sig)));
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

    /// Добавляет изображение; `fit`/`fit_bind` — режим вписывания.
    #[pyo3(signature = (src, *, pr=None, fit="contain", fit_bind=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn img(
        &mut self,
        py: Python,
        src: &str,
        pr: Option<PyNode>,
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
        let parent = self.parent_of(pr);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::Image {
                path: src.to_string(),
                fit: code,
            },
            props,
        );
        if let Some(f) = fit_bind {
            self.value_bindings.borrow_mut().push((id, f));
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
            CHART_BINDINGS.with(|c| c.borrow_mut().push((id, f)));
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

    /// Добавляет поле ввода; `sig` — сигнал, куда пишется текст.
    #[pyo3(signature = (txt="", *, pr=None, sig=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn tx(
        &mut self,
        txt: &str,
        pr: Option<PyNode>,
        sig: Option<PyObject>,
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

    /// Многострочное поле ввода; Enter — перенос строки, `sig` — сигнал текста.
    #[pyo3(signature = (txt="", *, pr=None, sig=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn ta(
        &mut self,
        txt: &str,
        pr: Option<PyNode>,
        sig: Option<PyObject>,
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

    /// Добавляет список; `ch(index)` при выборе пункта.
    #[pyo3(signature = (items, *, pr=None, sel=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn lst(
        &mut self,
        items: Vec<String>,
        pr: Option<PyNode>,
        sel: Option<usize>,
        ch: Option<PyObject>,
        pd: f32,
        gp: f32,
        w: Option<f32>,
        h: Option<f32>,
    ) -> PyResult<PyNode> {
        let h = h.or(Some(240.0));
        let props = make_props("v", pd, gp, w, h);
        let its: Vec<Vec<u16>> = items.iter().map(|s| utf16(s)).collect();
        let parent = self.parent_of(pr);
        let texts = self.bindings.clone();
        let values = self.value_bindings.clone();
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent,
            NodeKind::List {
                items: its,
                selected: sel,
                scroll: 0.0,
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

    /// Возвращает выбранный пункт списка или -1.
    fn lstv(&self, n: PyNode) -> PyResult<i64> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(tree.list_selected(n.id).map_or(-1, |i| i as i64))
    }

    /// Добавляет таблицу; `ch(row)` при выборе строки.
    #[pyo3(signature = (columns, rows, *, pr=None, ch=None, pd=0.0, gp=0.0, w=None, h=None))]
    fn tbl(
        &mut self,
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
        pr: Option<PyNode>,
        ch: Option<PyObject>,
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
    fn go(&mut self) -> PyResult<()> {
        dpi::enable_dpi_awareness();
        let tree = self.tree.take().ok_or_else(consumed)?;
        let window = CoreWindow::new(&self.title, self.width, self.height, tree, self.glass, self.tint, self.blur)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        window.run();
        Ok(())
    }
}

impl PyWindow {
    fn parent_of(&self, pr: Option<PyNode>) -> NodeId {
        match pr {
            Some(p) => p.id,
            None => *self.stack.borrow().last().unwrap_or(&self.root),
        }
    }
}

#[pyclass(name = "S")]
struct Signal {
    value: PyObject,
}

#[pymethods]
impl Signal {
    #[new]
    fn new(vl: PyObject) -> Self {
        Self { value: vl }
    }

    fn __call__(&self, py: Python) -> PyObject {
        self.value.clone_ref(py)
    }

    /// Возвращает текущее значение.
    fn gt(&self, py: Python) -> PyObject {
        self.value.clone_ref(py)
    }

    /// Устанавливает новое значение.
    fn st(&mut self, vl: PyObject) {
        self.value = vl;
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

fn refresh_all(py: Python, t: &mut Tree, texts: &Bindings, values: &Bindings) {
    for (id, f) in texts.borrow().iter() {
        match f.bind(py).call0().and_then(|v| v.extract::<String>()) {
            Ok(s) => t.set_label_text(*id, utf16(&s)),
            Err(e) => e.print(py),
        }
    }
    for (id, f) in values.borrow().iter() {
        match f.bind(py).call0().and_then(|v| v.extract::<f32>()) {
            Ok(v) => {
                t.set_slider_value(*id, v);
                t.set_progress_value(*id, v);
                t.set_gauge_value(*id, v);
                t.set_meter_value(*id, v);
                t.set_image_fit(*id, v as u8);
                if t.is_stack(*id) {
                    t.set_stack_page(*id, v.max(0.0) as usize);
                }
            }
            Err(e) => e.print(py),
        }
    }
    CHART_BINDINGS.with(|c| {
        for (id, f) in c.borrow().iter() {
            match f.bind(py).call0().and_then(|v| v.extract::<Vec<f32>>()) {
                Ok(v) => t.set_chart_values(*id, v),
                Err(e) => e.print(py),
            }
        }
    });
    WIN_SETTINGS.with(|s| {
        for (tag, f) in s.borrow().iter() {
            let v = match f.bind(py).call0().and_then(|x| x.extract::<f32>()) {
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
}

#[pymodule]
fn ssui(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWindow>()?;
    m.add_class::<PyNode>()?;
    m.add_class::<Ctx>()?;
    m.add_class::<Fx>()?;
    m.add_class::<Dlg>()?;
    m.add_class::<Signal>()?;
    m.add_function(wrap_pyfunction!(sgnl, m)?)?;
    Ok(())
}
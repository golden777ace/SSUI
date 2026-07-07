use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use ssui_core::platform::{dpi, Window as CoreWindow};
use ssui_core::tree::{Axis, NodeId, NodeKind, Props, Tree};

#[pyclass(name = "Node")]
#[derive(Clone, Copy)]
struct PyNode {
    id: NodeId,
}

#[pyclass(unsendable, name = "Window")]
struct PyWindow {
    tree: Option<Tree>,
    title: String,
    width: i32,
    height: i32,
}

#[pymethods]
impl PyWindow {
    #[new]
    #[pyo3(signature = (title="SSUI", width=1280, height=720))]
    fn new(title: &str, width: i32, height: i32) -> Self {
        Self {
            tree: Some(Tree::new()),
            title: title.to_string(),
            width,
            height,
        }
    }

    /// Возвращает корневой узел окна.
    fn root(&self) -> PyResult<PyNode> {
        let tree = self.tree.as_ref().ok_or_else(consumed)?;
        Ok(PyNode { id: tree.root() })
    }

    /// Добавляет панель со скруглением; возвращает её узел.
    #[pyo3(signature = (parent, radius=12.0, axis="vertical", padding=0.0, gap=0.0, width=None, height=None))]
    fn frame(
        &mut self,
        parent: PyNode,
        radius: f32,
        axis: &str,
        padding: f32,
        gap: f32,
        width: Option<f32>,
        height: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props(axis, padding, gap, width, height);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent.id, NodeKind::Frame { radius }, props);
        Ok(PyNode { id })
    }

    /// Добавляет текстовую метку; возвращает её узел.
    #[pyo3(signature = (parent, text, padding=0.0, gap=0.0, width=None, height=None))]
    fn label(
        &mut self,
        parent: PyNode,
        text: &str,
        padding: f32,
        gap: f32,
        width: Option<f32>,
        height: Option<f32>,
    ) -> PyResult<PyNode> {
        let props = make_props("vertical", padding, gap, width, height);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(parent.id, NodeKind::Label { text: utf16(text) }, props);
        Ok(PyNode { id })
    }

    /// Добавляет кнопку с колбэком; возвращает её узел.
    #[pyo3(signature = (parent, label, radius=10.0, padding=0.0, gap=0.0, width=None, height=None, on_click=None))]
    fn button(
        &mut self,
        parent: PyNode,
        label: &str,
        radius: f32,
        padding: f32,
        gap: f32,
        width: Option<f32>,
        height: Option<f32>,
        on_click: Option<PyObject>,
    ) -> PyResult<PyNode> {
        let props = make_props("vertical", padding, gap, width, height);
        let tree = self.tree.as_mut().ok_or_else(consumed)?;
        let id = tree.add_child(
            parent.id,
            NodeKind::Button {
                label: utf16(label),
                radius,
            },
            props,
        );
        if let Some(cb) = on_click {
            tree.set_on_click(id, move |_t| {
                Python::with_gil(|py| {
                    if let Err(e) = cb.bind(py).call0() {
                        e.print(py);
                    }
                });
            });
        }
        Ok(PyNode { id })
    }

    /// Показывает окно и запускает цикл сообщений.
    fn run(&mut self) -> PyResult<()> {
        dpi::enable_dpi_awareness();
        let tree = self.tree.take().ok_or_else(consumed)?;
        let window = CoreWindow::new(&self.title, self.width, self.height, tree)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        window.run();
        Ok(())
    }
}

fn consumed() -> PyErr {
    PyRuntimeError::new_err("окно уже запущено")
}

fn make_props(
    axis: &str,
    padding: f32,
    gap: f32,
    width: Option<f32>,
    height: Option<f32>,
) -> Props {
    Props {
        axis: parse_axis(axis),
        padding,
        gap,
        width,
        height,
    }
}

fn parse_axis(s: &str) -> Axis {
    match s {
        "horizontal" | "h" | "row" => Axis::Horizontal,
        _ => Axis::Vertical,
    }
}

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

#[pymodule]
fn ssui(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWindow>()?;
    m.add_class::<PyNode>()?;
    Ok(())
}
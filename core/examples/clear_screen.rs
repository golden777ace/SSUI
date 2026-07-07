use std::cell::Cell;
use std::rc::Rc;

use ssui_core::platform::{dpi, Window};
use ssui_core::render::Color;
use ssui_core::tree::{Axis, NodeKind, Props, Style, TextState, Tree};

fn label(text: &str) -> NodeKind {
    NodeKind::Label {
        text: text.encode_utf16().collect(),
    }
}

fn utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn main() -> windows::core::Result<()> {
    dpi::enable_dpi_awareness();

    let mut tree = Tree::new();
    let root = tree.root();
    tree.set_props(
        root,
        Props {
            axis: Axis::Vertical,
            padding: 24.0,
            gap: 16.0,
            ..Default::default()
        },
    );

    let panel = tree.add_child(
        root,
        NodeKind::Frame { radius: 16.0 },
        Props {
            axis: Axis::Vertical,
            padding: 20.0,
            gap: 12.0,
            ..Default::default()
        },
    );

    let count_label = tree.add_child(
        panel,
        label("Clicks: 0"),
        Props {
            height: Some(40.0),
            ..Default::default()
        },
    );

    let row = tree.add_child(
        panel,
        NodeKind::Container,
        Props {
            axis: Axis::Horizontal,
            gap: 12.0,
            height: Some(48.0),
            ..Default::default()
        },
    );
    let minus = tree.add_child(
        row,
        NodeKind::Button {
            label: utf16("-"),
            radius: 10.0,
        },
        Props {
            width: Some(64.0),
            ..Default::default()
        },
    );
    tree.set_style(
        minus,
        Style {
            fill: Some(Color::hex(0xE5484D)),
            text: Some(Color::hex(0xFFFFFF)),
        },
    );
    let plus = tree.add_child(
        row,
        NodeKind::Button {
            label: utf16("+"),
            radius: 10.0,
        },
        Props {
            width: Some(64.0),
            ..Default::default()
        },
    );
    tree.set_style(
        plus,
        Style {
            fill: Some(Color::hex(0x2FBF71)),
            text: Some(Color::hex(0xFFFFFF)),
        },
    );

    tree.add_child(
        panel,
        NodeKind::Checkbox {
            label: utf16("Enable feature"),
            checked: false,
        },
        Props {
            height: Some(28.0),
            ..Default::default()
        },
    );

    tree.add_child(
        panel,
        label("Type in the box:"),
        Props {
            height: Some(24.0),
            ..Default::default()
        },
    );
    tree.add_child(
        panel,
        NodeKind::TextBox {
            state: TextState::new(),
        },
        Props {
            height: Some(44.0),
            width: Some(280.0),
            ..Default::default()
        },
    );

    tree.add_child(
        panel,
        label("Press Space to cycle themes"),
        Props {
            height: Some(28.0),
            ..Default::default()
        },
    );
    let value_label = tree.add_child(
        panel,
        label("Value: 50%"),
        Props {
            height: Some(28.0),
            ..Default::default()
        },
    );
    let slider = tree.add_child(
        panel,
        NodeKind::Slider { value: 0.5 },
        Props {
            height: Some(40.0),
            width: Some(240.0),
            ..Default::default()
        },
    );
    let progress = tree.add_child(
        panel,
        NodeKind::Progress { value: 0.5 },
        Props {
            height: Some(24.0),
            width: Some(240.0),
            ..Default::default()
        },
    );

    let counter = Rc::new(Cell::new(0i32));
    {
        let counter = counter.clone();
        tree.set_on_click(plus, move |t| {
            counter.set(counter.get() + 1);
            t.set_label_text(count_label, utf16(&format!("Clicks: {}", counter.get())));
        });
    }
    {
        let counter = counter.clone();
        tree.set_on_click(minus, move |t| {
            counter.set(counter.get() - 1);
            t.set_label_text(count_label, utf16(&format!("Clicks: {}", counter.get())));
        });
    }
    tree.set_on_change(slider, move |t, v| {
        let percent = (v * 100.0).round() as i32;
        t.set_label_text(value_label, utf16(&format!("Value: {}%", percent)));
        t.set_progress_value(progress, v);
    });

    let window = Window::new("SSUI Demo", 1280, 720, tree)?;
    window.run();
    Ok(())
}
use ssui_core::platform::{dpi, Window};

fn main() -> windows::core::Result<()> {
    dpi::enable_dpi_awareness();

    let window = Window::new("SSUI — Phase 0: Clear Screen", 1280, 720)?;
    window.run();
    Ok(())
}

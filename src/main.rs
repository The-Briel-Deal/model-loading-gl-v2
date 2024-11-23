use model_loading::window::GfWindow;

fn main() -> anyhow::Result<()> {
    GfWindow::new()?.run()
}

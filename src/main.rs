use glutin::prelude::NotCurrentGlContext;
use model_loading::window::GfWindow;
use winit::event_loop::EventLoop;

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::builder().build().unwrap();

    let window = GfWindow::new(&event_loop)?;

    let surface = window.create_window_surface()?;
    let context = window.create_context()?.make_current(&surface)?;
    let renderer = window.create_gl_renderer();

    window.run(event_loop, surface, renderer, context)
}

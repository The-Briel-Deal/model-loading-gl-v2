use glutin::prelude::NotCurrentGlContext;
use model_loading::window::GfWindow;
use winit::event_loop::EventLoop;

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::builder().build().unwrap();

    let mut window = GfWindow::new(&event_loop)?;

    let context = window.create_context()?;
    let surface = window.create_window_surface()?;
    let current_context = context.make_current(&surface)?;
    let renderer = window.create_gl_renderer();

    window.surface = Some(surface);
    window.context = Some(current_context);
    window.renderer = Some(renderer);

    window.run(event_loop)
}

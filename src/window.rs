use std::num::NonZero;

use anyhow::Context;
use glutin::{
    config::{Config, ConfigTemplateBuilder, GlConfig},
    context::{ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext},
    display::GetGlDisplay,
    prelude::{GlDisplay, NotCurrentGlContext},
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::{DisplayBuilder, GlWindow};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::EventLoop,
    raw_window_handle::HasWindowHandle,
    window::{Window, WindowAttributes},
};

use crate::renderer::Renderer;

pub mod gl {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

pub struct GfWindow {
    window: Window,
    config: Config,
    pub renderer: Option<Renderer>,
    pub surface: Option<Surface<WindowSurface>>,
    pub context: Option<PossiblyCurrentContext>,
    exit_state: anyhow::Result<()>,
}

impl GfWindow {
    pub fn new(event_loop: &EventLoop<()>) -> anyhow::Result<Self> {
        let config_template_builder = ConfigTemplateBuilder::default();

        let config_picker = |configs: Box<dyn Iterator<Item = Config> + '_>| {
            configs
                .reduce(|acc, config| {
                    if config.num_samples() > acc.num_samples() {
                        config
                    } else {
                        acc
                    }
                })
                .unwrap()
        };
        let window_attributes = WindowAttributes::default().with_title("Model Testing Window");

        let (window, config) = DisplayBuilder::default()
            .with_window_attributes(Some(window_attributes))
            .build(event_loop, config_template_builder, config_picker)
            .unwrap();
        let window = window.unwrap();

        Ok(GfWindow {
            window,
            config,
            renderer: None,
            context: None,
            surface: None,
            exit_state: Ok(()),
        })
    }
    pub fn run(mut self, event_loop: EventLoop<()>) -> anyhow::Result<()> {
        event_loop
            .run_app(&mut self)?;
        Ok(())
    }
    pub fn create_context(&self) -> anyhow::Result<NotCurrentContext> {
        let window_handle = self.window.window_handle()?.as_raw();
        let context_attributes = ContextAttributesBuilder::new().build(Some(window_handle));
        let gl_display = self.config.display();
        unsafe { Ok(gl_display.create_context(&self.config, &context_attributes)?) }
    }
    pub fn create_window_surface(&self) -> anyhow::Result<Surface<WindowSurface>> {
        let display = self.config.display();
        let surface_attributes_builder = SurfaceAttributesBuilder::new();
        let surface_attributes = self
            .window
            .build_surface_attributes(surface_attributes_builder)?;
        unsafe { Ok(display.create_window_surface(&self.config, &surface_attributes)?) }
    }
    pub fn create_gl_renderer(&self) -> Renderer {
        // Renderer can't be instantiated until context is current
        Renderer::new(&self.config.display())
    }
}

impl ApplicationHandler for GfWindow {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {}
    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                self.renderer.as_ref().unwrap().draw();
                self.window.request_redraw();
                let _ = self
                    .surface
                    .as_ref()
                    .unwrap()
                    .swap_buffers(self.context.as_ref().unwrap());
            }
            WindowEvent::Resized(size) => {
                self.surface.as_ref().unwrap().resize(
                    self.context.as_ref().unwrap(),
                    NonZero::new(size.width).unwrap(),
                    NonZero::new(size.height).unwrap(),
                );
                self.renderer
                    .as_ref()
                    .unwrap()
                    .resize(size.width as i32, size.height as i32);
            }
            _ => (),
        }
    }
}

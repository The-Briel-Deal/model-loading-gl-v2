use std::{
    ffi::{CStr, CString},
    num::NonZero,
    ops::Deref,
};

use anyhow::Context;
use bytemuck::{cast_slice, offset_of, Pod, Zeroable};
use gl::{types::GLfloat, Gl};
use glam::{vec2, vec3, Vec2, Vec3};
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
    event_loop: Option<EventLoop<()>>,
    window: Window,
    config: Config,
    renderer: Option<Renderer>,
    surface: Option<Surface<WindowSurface>>,
    context: Option<PossiblyCurrentContext>,
    exit_state: anyhow::Result<()>,
}

impl GfWindow {
    pub fn new() -> anyhow::Result<Self> {
        let event_loop = EventLoop::builder().build().unwrap();
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
            .build(&event_loop, config_template_builder, config_picker)
            .unwrap();
        let window = window.unwrap();

        Ok(GfWindow {
            event_loop: Some(event_loop),
            window,
            config,
            renderer: None,
            context: None,
            surface: None,
            exit_state: Ok(()),
        })
    }
    pub fn run(mut self) -> anyhow::Result<()> {
        self.event_loop
            .take()
            .context("No event loop.")?
            .run_app(&mut self)?;
        Ok(())
    }
    fn create_context(&self) -> anyhow::Result<NotCurrentContext> {
        let window_handle = self.window.window_handle()?.as_raw();
        let context_attributes = ContextAttributesBuilder::new().build(Some(window_handle));
        let gl_display = self.config.display();
        unsafe { Ok(gl_display.create_context(&self.config, &context_attributes)?) }
    }
}

impl ApplicationHandler for GfWindow {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let context = match self.create_context() {
            Ok(context) => context,
            Err(err) => {
                self.exit_state = Err(err);
                event_loop.exit();
                return;
            }
        };
        let surface_attributes_builder = SurfaceAttributesBuilder::new();
        let surface_attributes = self
            .window
            .build_surface_attributes(surface_attributes_builder)
            .unwrap();
        let surface = unsafe {
            self.config
                .display()
                .create_window_surface(&self.config, &surface_attributes)
                .unwrap()
        };
        let possibly_current_context = context.make_current(&surface).unwrap();

        // Renderer can't be instantiated until context is current
        let renderer = Renderer::new(&self.config.display());

        self.renderer = Some(renderer);
        self.context = Some(possibly_current_context);
        self.surface = Some(surface);
    }
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



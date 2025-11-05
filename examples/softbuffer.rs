use std::{num::NonZero, time::Instant};

enum Application {}

struct Uninitialized {
    window_attributes: winit::window::WindowAttributes,
    context: softbuffer::Context<winit::event_loop::OwnedDisplayHandle>,
    start_time: Instant,
}

struct Suspended {
    uninitialized: Uninitialized,
}

struct Resumed {
    suspended: Suspended,
    surface: softbuffer::Surface<winit::event_loop::OwnedDisplayHandle, winit::window::Window>,
    size: winit::dpi::PhysicalSize<u32>,
}

struct Exited;

type Error = Box<dyn std::error::Error>;

impl winit_ext::Application for Application {
    type Uninitialized = Uninitialized;
    type Resumed = Resumed;
    type Suspended = Suspended;
    type Exited = Exited;
    type Error = Error;
}

impl winit_ext::ApplicationUninitialized for Uninitialized {
    type Application = Application;

    fn initialize(self, event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Resumed, Error> {
        winit_ext::ApplicationSuspended::resume(Suspended {
            uninitialized: self,
        }, event_loop)
    }
}

impl winit_ext::ApplicationResumed for Resumed {
    type Application = Application;

    fn handle(
            mut self,
            event_loop: &winit::event_loop::ActiveEventLoop,
            event: winit_ext::EventResumed,
        ) -> Result<Self, <Self::Application as winit_ext::Application>::Error> {
        match event {
            winit_ext::EventResumed::WindowEvent { window_id, event } if window_id == self.surface.window().id() => {
                match event {
                    winit::event::WindowEvent::Resized(size) if size != self.size => {
                        self.size = size;
                        self.surface.window().request_redraw();
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        event_loop.exit();
                    },
                    winit::event::WindowEvent::RedrawRequested => {
                        if let (Some(width), Some(height)) = (NonZero::new(self.size.width), NonZero::new(self.size.height)) {
                            self.surface.resize(width, height)?;
                        };
                        let dt = self.suspended.uninitialized.start_time.elapsed().as_secs_f32();
                        render(dt, &mut *self.surface.buffer_mut()?, self.size);
                        self.surface.window().pre_present_notify();
                        self.surface.buffer_mut()?.present()?;
                        self.surface.window().request_redraw();
                    },
                    winit::event::WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: false } => {
                        match event.physical_key {
                            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyQ) |
                            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape)  => {
                                event_loop.exit()
                            },
                            _ => {}
                        }
                    }
                    _ => {}
                }
            },
            _ => {}
        }
        Ok(self)
    }

    fn suspend(self, _event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Suspended, Error> {
        Ok(self.suspended)
    }

    fn exit(self, _event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Exited, Error> {
        Ok(Exited)
    }
}

impl winit_ext::ApplicationSuspended for Suspended {
    type Application = Application;

    fn handle(
        self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: winit_ext::EventSuspended<()>,
    ) -> Result<Self, <Self::Application as winit_ext::Application<()>>::Error> {
        let _ = (event_loop, event);
        Ok(self)
    }

    fn resume(self, event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Resumed, Error> {
        let window = event_loop.create_window(self.uninitialized.window_attributes.clone())?;
        let size = window.inner_size();
        let surface = softbuffer::Surface::new(&self.uninitialized.context, window)?;
        Ok(Resumed {
            suspended: self,
            surface,
            size,
        })
    }

    fn exit(self, _event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Exited, Error> {
        Ok(Exited)
    }
}

fn render(dt: f32, buffer: &mut [u32], size: winit::dpi::PhysicalSize<u32>) {
    for (index, pixel) in buffer.iter_mut().enumerate() {
        let dx = size.width;

        let x = index as u32 % dx + (dt * 123.45) as u32;
        let y = index as u32 / dx + (dt * 69.0) as u32;

        let a = 0x88;
        let r = x as u8;
        let g = y as u8;
        let b = 0x00;

        *pixel = u32::from_be_bytes([a, r, g, b]);
    }
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().expect("event loop error");
    let context = softbuffer::Context::new(event_loop.owned_display_handle()).expect("failed to create softbuffer context");
    let Exited = winit_ext::run_app(
        event_loop,
        Uninitialized {
            context,
            window_attributes: winit::window::WindowAttributes::default()
                .with_active(true)
                .with_title(env!("CARGO_PKG_NAME"))
                .with_transparent(true),
                start_time: Instant::now(),
        },
    )
    .expect("event loop error")
    .expect("application error");
}

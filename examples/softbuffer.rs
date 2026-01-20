use std::{num::NonZero, time::Instant};

type Error = Box<dyn std::error::Error>;

struct Uninitialized {
    window_attributes: winit::window::WindowAttributes,
    context: softbuffer::Context<winit::event_loop::OwnedDisplayHandle>,
    start_time: Instant,
}

impl Uninitialized {
    fn exit(self) -> Exited {
        Exited {
            context: self.context,
        }
    }
}

struct Suspended {
    uninitialized: Uninitialized,
    // NOTE: This state may be used to contain state that can be re-used across
    // suspend-resume cycles that is dependent on the created surface, such as a
    // rendering device.
}

impl Suspended {
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

    fn exit(self) -> Exited {
        self.uninitialized.exit()
    }
}

struct Resumed {
    suspended: Suspended,
    surface: softbuffer::Surface<winit::event_loop::OwnedDisplayHandle, winit::window::Window>,
    size: winit::dpi::PhysicalSize<u32>,
}

impl Resumed {
    fn handle_window_event(
        mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) -> Result<Self, Error> {
        match event {
            winit::event::WindowEvent::Resized(size) if size != self.size => {
                self.size = size;
                self.surface.window().request_redraw();
            }
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::RedrawRequested => {
                if let (Some(width), Some(height)) = (
                    NonZero::new(self.size.width),
                    NonZero::new(self.size.height),
                ) {
                    self.surface.resize(width, height)?;
                }
                let dt = self
                    .suspended
                    .uninitialized
                    .start_time
                    .elapsed()
                    .as_secs_f32();
                render(dt, &mut *self.surface.buffer_mut()?, self.size);
                self.surface.window().pre_present_notify();
                self.surface.buffer_mut()?.present()?;
                self.surface.window().request_redraw();
            }
            winit::event::WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: false,
            } => match event.physical_key {
                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyQ)
                | winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape) => {
                    event_loop.exit()
                }
                _ => {}
            },
            _ => {}
        }
        Ok(self)
    }

    fn suspend(self) -> Result<Suspended, Error> {
        Ok(self.suspended)
    }

    fn exit(self) -> Exited {
        self.suspended.exit()
    }
}

struct Exited {
    context: softbuffer::Context<winit::event_loop::OwnedDisplayHandle>,
}

macro_rules! impl_application {
    ($Application:ident { $($Variant:ident),* $(,)? }) => {
        #[allow(clippy::large_enum_variant)]
        enum $Application {
            $($Variant($Variant),)*
        }

        $(
            impl From<$Variant> for $Application {
                fn from(value: $Variant) -> Self {
                    Self::$Variant(value)
                }
            }
        )*
    };
}

impl_application!(Application {
    Uninitialized,
    Suspended,
    Resumed,
    Exited,
});

impl Application {
    fn exit(self) -> Result<Exited, Error> {
        Ok(match self {
            Application::Uninitialized(uninitialized) => uninitialized.exit(),
            Application::Suspended(suspended) => suspended.exit(),
            Application::Resumed(resumed) => resumed.exit(),
            Application::Exited(exited) => exited,
        })
    }
}

#[cold]
fn invalid_transition() -> ! {
    panic!("invalid transition")
}

impl winit_ext::ApplicationHandlerFallibleOwned for Application {
    type Error = Error;

    fn resumed(self, event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Self, Self::Error> {
        match self {
            Application::Uninitialized(uninitialized) => {
                Ok(Suspended { uninitialized }.resume(event_loop)?.into())
            }
            Application::Suspended(suspended) => Ok(suspended.resume(event_loop)?.into()),
            _ => invalid_transition(),
        }
    }

    fn suspended(
        self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<Self, Self::Error> {
        match self {
            Application::Resumed(resumed) => Ok(resumed.suspend()?.into()),
            _ => invalid_transition(),
        }
    }

    fn window_event(
        self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) -> Result<Self, Self::Error> {
        let resumed = match self {
            Application::Resumed(resumed) => resumed,
            _ => invalid_transition(),
        };

        Ok(resumed
            .handle_window_event(event_loop, window_id, event)?
            .into())
    }

    fn about_to_wait(
        self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<Self, Self::Error> {
        if let Application::Resumed(resumed) = &self {
            resumed.surface.window().request_redraw();
        }
        Ok(self)
    }

    fn exiting(
        self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<Self, Self::Error> {
        Ok(Application::Exited(self.exit()?))
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
    use winit_ext::EventLoopExt;

    let event_loop = winit::event_loop::EventLoop::new().expect("event loop error");
    let context = softbuffer::Context::new(event_loop.owned_display_handle())
        .expect("failed to create softbuffer context");

    let Exited { context } = event_loop
        .run_app_fallible_owned(Application::Uninitialized(Uninitialized {
            context,
            window_attributes: winit::window::WindowAttributes::default()
                .with_active(true)
                .with_title(env!("CARGO_PKG_NAME"))
                .with_transparent(true),
            start_time: Instant::now(),
        }))
        .expect("event loop error")
        .expect("application error")
        .exit()
        .expect("exit error");

    // We have access to the context we passed into the application again, on platforms where the event loop actually exits.
    _ = context;
}

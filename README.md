Your application is a state machine. This crate makes that state machine explicit which has a couple of advantages over the `ApplicationHandler` interface that `winit` exposes. 

- some transitions that can not occur at run time can now instead not occur at compile time.
- you have access to your application by value, rather than by reference, which allows you to implement sub-state machines particular to your application.

See the [examples](./examples/) for a demonstration:

```rust
enum Application {}

struct Uninitialized {
    window_attributes: winit::window::WindowAttributes,
}

struct Resumed {
    window_attributes: winit::window::WindowAttributes,
    #[allow(unused)]
    window: winit::window::Window,
}

struct Suspended {
    window_attributes: winit::window::WindowAttributes,
}

struct Exited;

type Error = Box<dyn std::error::Error>;

impl<TUserEvent: 'static> winit_ext::Application<TUserEvent> for Application {
    type Uninitialized = Uninitialized;
    type Resumed = Resumed;
    type Suspended = Suspended;
    type Exited = Exited;
    type Error = Error;
}

impl<TUserEvent: 'static> winit_ext::ApplicationUninitialized<TUserEvent> for Uninitialized {
    type Application = Application;

    fn initialize(self, event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Resumed, Error> {
        let Self { window_attributes } = self;
        let window = event_loop.create_window(window_attributes.clone())?;
        Ok(Resumed {
            window_attributes,
            window,
        })
    }
}

impl<TUserEvent: 'static> winit_ext::ApplicationResumed<TUserEvent> for Resumed {
    type Application = Application;

    fn suspend(self, _event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Suspended, Error> {
        let Self {
            window_attributes, ..
        } = self;
        Ok(Suspended { window_attributes })
    }

    fn exit(self, _event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Exited, Error> {
        Ok(Exited)
    }
}

impl<TUserEvent: 'static> winit_ext::ApplicationSuspended<TUserEvent> for Suspended {
    type Application = Application;

    fn resume(self, event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Resumed, Error> {
        let Self { window_attributes } = self;
        let window = event_loop.create_window(window_attributes.clone())?;
        Ok(Resumed {
            window_attributes,
            window,
        })
    }

    fn exit(self, _event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Exited, Error> {
        Ok(Exited)
    }
}

fn main() {
    let event_loop = winit::event_loop::EventLoop::new().expect("event loop error");
    let Exited = winit_ext::run(
        event_loop,
        Uninitialized {
            window_attributes: winit::window::WindowAttributes::default(),
        },
    )
    .expect("event loop error")
    .expect("application error");
}
```

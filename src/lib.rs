use winit::{
    event::{DeviceEvent, DeviceId, StartCause, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

/// A version of [`winit::application::ApplicationHandler::new_events`] that provides `Self` by value and allows
/// implementations to return an error type. When an error occurs, [`winit::event_loop::ActiveEventLoop::exit`] is
/// called automatically and further events are ignored.
pub trait ApplicationHandlerFallibleOwned<T: 'static = ()>: Sized {
    type Error;

    /// See [`winit::application::ApplicationHandler::new_events`].
    fn new_events(
        self,
        event_loop: &ActiveEventLoop,
        cause: StartCause,
    ) -> Result<Self, Self::Error> {
        let _ = (event_loop, cause);
        Ok(self)
    }

    /// See [`winit::application::ApplicationHandler::resumed`].
    fn resumed(self, event_loop: &ActiveEventLoop) -> Result<Self, Self::Error>;

    /// See [`winit::application::ApplicationHandler::user_event`].
    fn user_event(self, event_loop: &ActiveEventLoop, event: T) -> Result<Self, Self::Error> {
        let _ = (event_loop, event);
        Ok(self)
    }

    /// See [`winit::application::ApplicationHandler::window_event`].
    fn window_event(
        self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) -> Result<Self, Self::Error>;

    /// See [`winit::application::ApplicationHandler::device_event`].
    fn device_event(
        self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) -> Result<Self, Self::Error> {
        let _ = (event_loop, device_id, event);
        Ok(self)
    }

    /// See [`winit::application::ApplicationHandler::about_to_wait`].
    fn about_to_wait(self, event_loop: &ActiveEventLoop) -> Result<Self, Self::Error> {
        let _ = event_loop;
        Ok(self)
    }

    /// See [`winit::application::ApplicationHandler::suspended`].
    fn suspended(self, event_loop: &ActiveEventLoop) -> Result<Self, Self::Error> {
        let _ = event_loop;
        Ok(self)
    }

    /// See [`winit::application::ApplicationHandler::exiting`].
    fn exiting(self, event_loop: &ActiveEventLoop) -> Result<Self, Self::Error> {
        let _ = event_loop;
        Ok(self)
    }

    /// See [`winit::application::ApplicationHandler::memory_warning`].
    fn memory_warning(self, event_loop: &ActiveEventLoop) -> Result<Self, Self::Error> {
        let _ = event_loop;
        Ok(self)
    }
}

#[cold]
const fn panic_poison() -> ! {
    panic!("application handler re-used after a panic occured during a transition")
}

enum Takeable<T> {
    Item(T),
    Poison,
}

impl<T> Takeable<T> {
    pub fn transition(&mut self, f: impl FnOnce(T) -> T) {
        *self = Self::Item(f(std::mem::replace(self, Takeable::Poison).into_inner()))
    }

    pub fn into_inner(self) -> T {
        match self {
            Self::Item(item) => item,
            Self::Poison => panic_poison(),
        }
    }
}

struct ApplicationHandlerWrapper<T, E> {
    state: Takeable<Result<T, E>>,
}

impl<T, E> ApplicationHandlerWrapper<T, E> {
    fn new(state: T) -> Self {
        Self {
            state: Takeable::Item(Ok(state)),
        }
    }

    fn into_inner(self) -> Result<T, E> {
        self.state.into_inner()
    }

    fn transition(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        f: impl FnOnce(T) -> Result<T, E>,
    ) {
        self.state
            .transition(|state| state.and_then(|state| f(state).inspect_err(|_| event_loop.exit())))
    }
}

impl<Handler, UserEvent> winit::application::ApplicationHandler<UserEvent>
    for ApplicationHandlerWrapper<Handler, Handler::Error>
where
    Handler: ApplicationHandlerFallibleOwned<UserEvent>,
    UserEvent: 'static,
{
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        self.transition(event_loop, |state| state.new_events(event_loop, cause));
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.transition(event_loop, |state| state.resumed(event_loop));
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        self.transition(event_loop, |state| state.user_event(event_loop, event));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.transition(event_loop, |state| {
            state.window_event(event_loop, window_id, event)
        });
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        self.transition(event_loop, |state| {
            state.device_event(event_loop, device_id, event)
        });
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.transition(event_loop, |state| state.about_to_wait(event_loop));
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.transition(event_loop, |state| state.suspended(event_loop));
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.transition(event_loop, |state| state.exiting(event_loop));
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        self.transition(event_loop, |state| state.memory_warning(event_loop));
    }
}

#[extend::ext(name = EventLoopExt)]
pub impl<UserEvent: 'static> winit::event_loop::EventLoop<UserEvent> {
    fn run_app_fallible_owned<Handler: ApplicationHandlerFallibleOwned<UserEvent>>(
        self,
        handler: Handler,
    ) -> Result<Result<Handler, Handler::Error>, winit::error::EventLoopError> {
        let mut wrapper = ApplicationHandlerWrapper::new(handler);
        self.run_app(&mut wrapper)?;
        Ok(wrapper.into_inner())
    }
}

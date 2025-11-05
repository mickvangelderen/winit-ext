mod takeable;

use crate::takeable::Takeable;

pub trait Application<TUserEvent: 'static = ()>: Sized {
    type Uninitialized: ApplicationUninitialized<TUserEvent, Application = Self>;
    type Resumed: ApplicationResumed<TUserEvent, Application = Self>;
    type Suspended: ApplicationSuspended<TUserEvent, Application = Self>;
    type Exited;
    type Error;
}

pub trait ApplicationUninitialized<TUserEvent: 'static = ()>: Sized {
    type Application: Application<TUserEvent, Uninitialized = Self>;

    fn initialize(
        self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<
        <Self::Application as Application<TUserEvent>>::Resumed,
        <Self::Application as Application<TUserEvent>>::Error,
    >;
}

pub trait ApplicationResumed<TUserEvent: 'static = ()>: Sized {
    type Application: Application<TUserEvent, Resumed = Self>;

    fn handle(
        self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: EventResumed<TUserEvent>,
    ) -> Result<Self, <Self::Application as Application<TUserEvent>>::Error>;

    fn suspend(
        self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<
        <Self::Application as Application<TUserEvent>>::Suspended,
        <Self::Application as Application<TUserEvent>>::Error,
    >;

    fn exit(
        self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<
        <Self::Application as Application<TUserEvent>>::Exited,
        <Self::Application as Application<TUserEvent>>::Error,
    >;
}

pub trait ApplicationSuspended<TUserEvent: 'static = ()>: Sized {
    type Application: Application<TUserEvent, Suspended = Self>;

    fn handle(
        self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: EventSuspended<TUserEvent>,
    ) -> Result<Self, <Self::Application as Application<TUserEvent>>::Error>;

    fn resume(
        self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<
        <Self::Application as Application<TUserEvent>>::Resumed,
        <Self::Application as Application<TUserEvent>>::Error,
    >;

    fn exit(
        self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) -> Result<
        <Self::Application as Application<TUserEvent>>::Exited,
        <Self::Application as Application<TUserEvent>>::Error,
    >;
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventResumed<TUserEvent: 'static = ()> {
    NewEvents(winit::event::StartCause),
    WindowEvent {
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    },
    DeviceEvent {
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    },
    UserEvent(TUserEvent),
    AboutToWait,
    MemoryWarning,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventSuspended<TUserEvent: 'static = ()> {
    NewEvents(winit::event::StartCause),
    DeviceEvent {
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    },
    UserEvent(TUserEvent),
    AboutToWait,
    MemoryWarning,
}

enum State<TApplicationState: Application<TUserEvent>, TUserEvent: 'static> {
    Uninitialized(TApplicationState::Uninitialized),
    Resumed(TApplicationState::Resumed),
    Suspended(TApplicationState::Suspended),
    Exited(TApplicationState::Exited),
}

fn invalid_transition() -> ! {
    unreachable!("invalid transition")
}

struct Adapter<TApplication: Application<TUserEvent>, TUserEvent: 'static>(
    Takeable<Result<State<TApplication, TUserEvent>, TApplication::Error>>,
);

impl<TApplication: Application<TUserEvent>, TUserEvent> Adapter<TApplication, TUserEvent> {
    fn new(state: TApplication::Uninitialized) -> Self {
        Self(Takeable::new(Ok(State::Uninitialized(state))))
    }

    fn exit(self) -> Result<TApplication::Exited, TApplication::Error> {
        Ok(match self.0.get()? {
            State::Uninitialized(_) => invalid_transition(),
            State::Resumed(_) => invalid_transition(),
            State::Suspended(_) => invalid_transition(),
            State::Exited(state) => state,
        })
    }

    fn transition<
        F: FnOnce(
            State<TApplication, TUserEvent>,
        ) -> Result<State<TApplication, TUserEvent>, TApplication::Error>,
    >(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        f: F,
    ) {
        self.0.transition(|fallible_state| {
            fallible_state.and_then(|state| f(state).inspect_err(|_| event_loop.exit()))
        })
    }
}

impl<TApplicationState: Application<TUserEvent>, TUserEvent>
    winit::application::ApplicationHandler<TUserEvent> for Adapter<TApplicationState, TUserEvent>
{
    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        self.transition(event_loop, |state| {
            Ok(match state {
                State::Uninitialized(state) => match cause {
                    winit::event::StartCause::Init => State::Resumed(state.initialize(event_loop)?),
                    _ => invalid_transition(),
                },
                State::Resumed(state) => {
                    State::Resumed(state.handle(event_loop, EventResumed::NewEvents(cause))?)
                }
                State::Suspended(state) => {
                    State::Suspended(state.handle(event_loop, EventSuspended::NewEvents(cause))?)
                }
                State::Exited(_) => invalid_transition(),
            })
        })
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: TUserEvent) {
        self.transition(event_loop, |state| {
            Ok(match state {
                State::Uninitialized(_) => invalid_transition(),
                State::Resumed(state) => {
                    State::Resumed(state.handle(event_loop, EventResumed::UserEvent(event))?)
                }
                State::Suspended(state) => {
                    State::Suspended(state.handle(event_loop, EventSuspended::UserEvent(event))?)
                }
                State::Exited(_) => invalid_transition(),
            })
        })
    }

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        self.transition(event_loop, |state| {
            Ok(match state {
                State::Uninitialized(_) => invalid_transition(),
                State::Resumed(state) => State::Resumed(
                    state.handle(event_loop, EventResumed::DeviceEvent { device_id, event })?,
                ),
                State::Suspended(state) => State::Suspended(
                    state.handle(event_loop, EventSuspended::DeviceEvent { device_id, event })?,
                ),
                State::Exited(_) => invalid_transition(),
            })
        })
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.transition(event_loop, |state| {
            Ok(match state {
                State::Uninitialized(_) => invalid_transition(),
                State::Resumed(state) => {
                    State::Resumed(state.handle(event_loop, EventResumed::AboutToWait)?)
                }
                State::Suspended(state) => {
                    State::Suspended(state.handle(event_loop, EventSuspended::AboutToWait)?)
                }
                State::Exited(_) => invalid_transition(),
            })
        })
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.transition(event_loop, |state| {
            Ok(match state {
                State::Uninitialized(_) => invalid_transition(),
                State::Resumed(state) => State::Suspended(state.suspend(event_loop)?),
                State::Suspended(state) => State::Suspended(state),
                State::Exited(_) => invalid_transition(),
            })
        })
    }

    fn exiting(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.transition(event_loop, |state| {
            Ok(match state {
                State::Uninitialized(_) => invalid_transition(),
                State::Resumed(state) => State::Exited(state.exit(event_loop)?),
                State::Suspended(state) => State::Exited(state.exit(event_loop)?),
                State::Exited(_) => invalid_transition(),
            })
        })
    }

    fn memory_warning(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.transition(event_loop, |state| {
            Ok(match state {
                State::Uninitialized(_) => invalid_transition(),
                State::Resumed(state) => {
                    State::Resumed(state.handle(event_loop, EventResumed::MemoryWarning)?)
                }
                State::Suspended(state) => {
                    State::Suspended(state.handle(event_loop, EventSuspended::MemoryWarning)?)
                }
                State::Exited(_) => invalid_transition(),
            })
        })
    }

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.transition(event_loop, |state| {
            Ok(match state {
                State::Uninitialized(_) => invalid_transition(),
                State::Resumed(state) => State::Resumed(state),
                State::Suspended(state) => State::Resumed(state.resume(event_loop)?),
                State::Exited(_) => invalid_transition(),
            })
        })
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        self.transition(event_loop, |state| {
            Ok(match state {
                State::Uninitialized(_) => invalid_transition(),
                State::Resumed(state) => State::Resumed(
                    state.handle(event_loop, EventResumed::WindowEvent { window_id, event })?,
                ),
                State::Suspended(_) => {
                    // TODO: Can we receive window events while suspended?
                    invalid_transition()
                }
                State::Exited(_) => invalid_transition(),
            })
        })
    }
}

type ApplicationResult<TApplication, TUserEvent> = Result<
    <TApplication as Application<TUserEvent>>::Exited,
    <TApplication as Application<TUserEvent>>::Error,
>;
type EventLoopResult<T> = Result<T, winit::error::EventLoopError>;

 pub fn run_app<TUserEvent, TApplicationUninitialized: ApplicationUninitialized<TUserEvent>>(event_loop: winit::event_loop::EventLoop<TUserEvent>, app: TApplicationUninitialized) -> EventLoopResult<ApplicationResult<TApplicationUninitialized::Application, TUserEvent>> {
    let mut app = Adapter::<TApplicationUninitialized::Application, TUserEvent>::new(app);
    event_loop.run_app(&mut app)?;
    Ok(app.exit())
}

This crate provides `ApplicationHandlerFallibleOwned` which is an alternative `winit::application:ApplicationHandler` trait that provides `Self` by value and accepts a `Result<Self, Self::Error>` in the event handler methods.
We also provide `EventLoopExt` which is an extension trait to `winit::event_loop::EventLoop` that provides `EventLoop::run_app_fallible_owned` accepting a type that implements `ApplicationHandlerFallibleOwned`.

Obtaining `Self` by value is useful if you want to represent your applications state through an enum.
Being able to return errors with `?` is useful. When an error occurs, the event loop is automatically asked to exit, the application is dropped and the error is returned to the `run_app_fallible_owned` caller. 

See the [examples](./examples/) for a demonstration:

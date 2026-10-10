use std::fmt;
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};
use tauri_runtime::{
    AppCounter, Error, ExitOrAlterExecution, RunEvent, Runtime, RuntimeHandle, UserEvent,
};

use crate::embedder::ServoEvent;

pub struct ServoRuntime {
    event_loop: EventLoop<ServoEvent>,
}

impl fmt::Debug for ServoRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServoRuntime").finish_non_exhaustive()
    }
}

// Minimal implementation of tauri_runtime::Runtime for the Servo engine
impl<T: UserEvent> Runtime<T> for ServoRuntime {
    type EventLoopProxy = ServoProxy<T>;
    type Handle = ServoHandle<T>;
    type Window = ServoWin<T>;

    fn new() -> Result<Self, Error> {
        let event_loop = EventLoopBuilder::<ServoEvent>::with_user_event().build();
        Ok(Self { event_loop })
    }

    fn create_window<F>(
        &self,
        _handle: Self::Handle,
        _pending: tauri_runtime::window::PendingWindow<T, Self>,
    ) -> Result<Self::Window, Error> {
        Err(Error::CreateWebview("Pending implementation".into()))
    }

    fn run<F>(self, mut callback: F)
    where
        F: FnMut(RunEvent<T>) -> ExitOrAlterExecution + 'static,
    {
        self.event_loop.run(move |_event, _, control_flow| {
            let action = callback(RunEvent::Ready);
            if let ExitOrAlterExecution::Exit = action {
                *control_flow = ControlFlow::Exit;
            }
        });
    }
}

// Stub structures for Runtime traits (to be expanded)
#[derive(Clone)]
pub struct ServoProxy<T: UserEvent>(EventLoopProxy<ServoEvent>, std::marker::PhantomData<T>);
impl<T: UserEvent> fmt::Debug for ServoProxy<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServoProxy").finish()
    }
}
impl<T: UserEvent> tauri_runtime::EventLoopProxy<T> for ServoProxy<T> {
    fn send_event(&self, _event: T) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct ServoHandle<T: UserEvent>(std::marker::PhantomData<T>);
impl<T: UserEvent> fmt::Debug for ServoHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServoHandle").finish()
    }
}
impl<T: UserEvent> RuntimeHandle<T> for ServoHandle<T> {}

pub struct ServoWin<T: UserEvent>(std::marker::PhantomData<T>);
impl<T: UserEvent> fmt::Debug for ServoWin<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServoWin").finish()
    }
}

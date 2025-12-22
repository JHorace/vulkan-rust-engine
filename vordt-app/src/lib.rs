use vordt_engine::VulkanEngine;
use winit::error::EventLoopError;
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::{
    application::ApplicationHandler, event::Event::UserEvent, event::WindowEvent,
    event_loop::ActiveEventLoop, event_loop::EventLoop, window::Window, window::WindowAttributes,
    window::WindowId,
};

//Vulkan engine app that uses a window
//Window is an Option because it is not guaranteed to exist until the resumed event is emitted.
//Engine is an Option because we can't create a windowed engine without a window.
pub struct EngineApplication {
    window: Option<Window>,
    engine: Option<VulkanEngine>,
}

impl ApplicationHandler for EngineApplication {
    //Emitted when the application has been resumed.
    //It is recommended that applications only initialize their graphics context and create a window
    //after a resumed event has been first emitted.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.window = event_loop.create_window(WindowAttributes::default()).ok();

            let display_handle = self
                .window
                .as_ref()
                .unwrap()
                .display_handle()
                .unwrap()
                .as_raw();

            let window_handle = self
                .window
                .as_ref()
                .unwrap()
                .window_handle()
                .unwrap()
                .as_raw();

            let display_window_handles = (display_handle, window_handle);

            self.engine = Some(
                VulkanEngine::new(true, Some(display_window_handles))
                    .expect("Failed to create engine"),
            );
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Window close requested, closing...");
                event_loop.exit();
            }
            _ => println!("Window event: {:?}", event),
        }
    }
}

impl EngineApplication {
    //When running tests, the event loop may be created on any thread.
    pub fn run(&mut self) {
        let event_loop = EventLoop::builder()
            .with_any_thread(true)
            .build()
            .expect("vordt-app: winit could not create an event loop");

        event_loop
            .run_app(self)
            .expect("vordt-app: winit could not run the event loop");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform::x11;
    use vordt_engine::VulkanEngine;
    use winit::event::WindowEvent::CloseRequested;
    use winit::platform;
    use winit::platform::wayland::EventLoopBuilderExtWayland;

    struct TestAppEmpty;

    impl ApplicationHandler for TestAppEmpty {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            event_loop.exit()
        }

        fn window_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _: WindowId,
            _event: WindowEvent,
        ) {
        }
    }

    //Simple test that creates an app, runs it, and immediately exits.
    #[test]
    fn test_create_xlib_app() {
        //EventLoop is intended to run on the main thread - but tests run in parallel on arbitrary threads
        //So we use the with_any_thread method to specify that we want to run on any thread.
        let event_loop =
            x11::EventLoopBuilderExtX11::with_any_thread(&mut EventLoop::builder(), true)
                .build()
                .expect("Failed to create event loop");

        let mut app = TestAppEmpty {};
        event_loop.run_app(&mut app).expect("Failed to run app")
    }

    #[test]
    fn test_create_wayland_app() {
        let event_loop =
            EventLoopBuilderExtWayland::with_any_thread(&mut EventLoop::builder(), true)
                .build()
                .expect("Failed to create event loop");

        let mut app = TestAppEmpty {};
        event_loop.run_app(&mut app).expect("Failed to run app")
    }

    #[test]
    fn test_create_vordt_app() {
        let mut app = EngineApplication {
            window: None,
            engine: None,
        };
        app.run();
    }
}

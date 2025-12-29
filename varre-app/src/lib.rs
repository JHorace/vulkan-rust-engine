use std::cell::RefCell;
use varre_engine::VulkanEngine;
use winit::error::EventLoopError;
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
};

use winit::event_loop::{EventLoopProxy, EventLoopBuilder, ActiveEventLoop};

use winit::{
    application::ApplicationHandler, event::WindowEvent,
    window::Window, window::WindowAttributes,
    window::WindowId,
};
use winit::dpi::Size;
use winit::platform::wayland::ActiveEventLoopExtWayland;

//Vulkan engine app that uses a window
//Window is an Option because it is not guaranteed to exist until the resumed event is emitted.
//Engine is an Option because we can't create a windowed engine without a window.
#[derive(Default)]
pub struct EngineApplication {
    window: Option<Box<dyn Window>>,
    engine: Option<VulkanEngine>,
}


impl ApplicationHandler for EngineApplication {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.window = Some(event_loop.create_window(WindowAttributes::default()).expect("Failed to create window"));
    }

    fn window_event(&mut self,
                    event_loop: &dyn ActiveEventLoop,
                    id: WindowId,
                    event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Window close requested, closing...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => println!("Window event: {:?}", event),
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use platform::x11;
    use varre_engine::VulkanEngine;
    use winit::event::WindowEvent::CloseRequested;
    use winit::event_loop::ControlFlow;
    use winit::platform;
    use winit::platform::wayland::EventLoopBuilderExtWayland;

    #[derive(Default)]
    struct TestAppEmpty;

    impl ApplicationHandler for TestAppEmpty {
        fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
            event_loop.exit()
        }

        fn window_event(
            &mut self,
            _event_loop: &dyn ActiveEventLoop,
            _: WindowId,
            _event: WindowEvent,
        ) {
        }
    }

    //Simple test that creates an app, runs it, and immediately exits.
    #[test]
    fn test_create_xlib_app() {
        let event_loop = EventLoopBuilderExtX11::with_any_thread(EventLoopBuilder::default().with_x11(), true)
            .build()
            .expect("varre-app: winit could not create an event loop");

        event_loop.run_app(TestAppEmpty::default()).expect("Failed to run app")
    }

    #[test]
    fn test_create_wayland_app() {
        let event_loop = EventLoopBuilderExtWayland::with_any_thread(EventLoopBuilder::default().with_wayland(), true)
            .build()
            .expect("varre-app: winit could not create an event loop");

        event_loop.run_app(TestAppEmpty::default()).expect("Failed to run app")
    }

    #[test]
    fn test_create_varre_app() {
        let event_loop = EventLoopBuilderExtWayland::with_any_thread(EventLoopBuilder::default().with_wayland(), true)
            .build()
            .expect("varre-app: winit could not create an event loop");

        event_loop.set_control_flow(ControlFlow::Poll);

        let app = EngineApplication::default();

        event_loop.run_app(app).expect("Failed to run app");
    }

    #[test]
    fn test_clear_color() {
        let event_loop = EventLoopBuilderExtWayland::with_any_thread(EventLoopBuilder::default().with_wayland(), true)
            .build()
            .expect("varre-app: winit could not create an event loop");

        event_loop.set_control_flow(ControlFlow::Poll);

        let app = EngineApplication::default();

        event_loop.run_app(app).expect("Failed to run app")
    }

}

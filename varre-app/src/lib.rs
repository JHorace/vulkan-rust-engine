use varre_engine::VulkanEngine;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use winit::{
    event_loop::ActiveEventLoop,
    application::ApplicationHandler,
    event::WindowEvent,
    window::{Window, WindowAttributes, WindowId},
};

// @NOTE The relationship between VarreApplicationCore, VarreApplicationImpl, and the winit event loop
//       is confusing, and their design is driven by my inexperience with rust and winit. I'll document
//       the design decisions and limitations in the comments here:
//       - The overall goal is to create a base class that implements window event handling behavior that is shared between
//         all applications. This includes creating the initial window, adding it to the VulkanEngine, and notifying the engine
//         if the window is resized. Other apps then don't need to reimplement this behavior, but can define custom event handling
//         such as additional keyboard input responses.
//       - EventLoop::run_app moves its ApplicationHandler argument, so a struct that implements ApplicationHandler
//         cannot create its own event loop.
pub trait VarreApplicationImpl {

    fn on_window_event(&mut self, event: &WindowEvent, engine: &mut VulkanEngine) -> bool {
        // Return true if the event was handled, false to use default handling
        false
    }
}

pub struct VarreApplicationCore {
    window: Option<Box<dyn Window>>,
    engine: Option<VulkanEngine>,
    app_impl: Option<Box<dyn VarreApplicationImpl>>,
}

impl VarreApplicationCore {

    pub fn new(app_impl: Box<dyn VarreApplicationImpl>) -> Self {
        Self {
            window: None,
            engine: None,
            app_impl: Some(app_impl),
        }
    }
    pub fn start(&self) {
        
    }
}

//Vulkan engine app that uses a window
//Window is an Option because it is not guaranteed to exist until the resumed event is emitted.
//Engine is an Option because we can't create a windowed engine without a window.
impl ApplicationHandler for VarreApplicationCore {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let window = event_loop
            .create_window(WindowAttributes::default())
            .expect("Failed to create window");
        let display_handle = event_loop.display_handle().unwrap().as_raw();
        let window_handle = window.window_handle().unwrap().as_raw();
        self.engine = Some(
            VulkanEngine::new(true, Some(display_handle)).expect("Failed to create VulkanEngine"),
        );
        self.engine.as_mut().unwrap().add_window(
            display_handle,
            window_handle,
            window.surface_size().width,
            window.surface_size().height,
        );
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, id: WindowId, event: WindowEvent) {
        // Let the app implementation handle the event first if present
        let handled = self.app_impl.as_mut().map_or(false, |app| {
            app.on_window_event(&event, self.engine.as_mut().unwrap())
        });

        if !handled {
            // Default handling if the app didn't handle it
            match event {
                WindowEvent::CloseRequested => {
                    println!("Window close requested, closing...");
                    event_loop.exit();
                }
                WindowEvent::RedrawRequested => {
                    //self.window.as_ref().unwrap().request_redraw();
                }
                WindowEvent::SurfaceResized(size) => {
                    self.engine.as_mut().unwrap().recreate_swapchain(
                        size.width,
                        size.height,
                    );
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::event_loop::{ControlFlow, EventLoopBuilder};
    use winit::platform::wayland::EventLoopBuilderExtWayland;
    use winit::platform::x11::EventLoopBuilderExtX11;

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
        let event_loop =
            EventLoopBuilderExtX11::with_any_thread(EventLoopBuilder::default().with_x11(), true)
                .build()
                .expect("varre-app: winit could not create an event loop");

        event_loop
            .run_app(TestAppEmpty::default())
            .expect("Failed to run app")
    }

    #[test]
    fn test_create_wayland_app() {
        let event_loop = EventLoopBuilderExtWayland::with_any_thread(
            EventLoopBuilder::default().with_wayland(),
            true,
        )
        .build()
        .expect("varre-app: winit could not create an event loop");

        event_loop
            .run_app(TestAppEmpty::default())
            .expect("Failed to run app")
    }
}

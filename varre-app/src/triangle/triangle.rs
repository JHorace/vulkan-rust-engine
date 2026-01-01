use winit::event::WindowEvent;
use winit::event_loop::{EventLoopBuilder};
use winit::platform::wayland::EventLoopBuilderExtWayland;
use varre_app::*;
use varre_engine::VulkanEngine;

struct TriangleApp;

impl VarreApplicationImpl for TriangleApp {
    fn on_window_event(&mut self, event: &WindowEvent, engine: &mut VulkanEngine) -> bool {
        match event {
            WindowEvent::RedrawRequested => {
                engine.draw();
                return true;
            },
            _ => false,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoopBuilderExtWayland::with_any_thread(
        EventLoopBuilder::default().with_wayland(),
        true,
    )
        .build()
        .expect("varre-app: winit could not create an event loop");
     // let event_loop = EventLoop::builder().with_wayland()
     //     .build().expect("Failed to create event loop");

    let app = VarreApplicationCore::new(Box::new(TriangleApp));

    event_loop.run_app(app).expect("Failed to run app");


Ok(())
}
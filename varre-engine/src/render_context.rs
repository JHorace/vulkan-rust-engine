pub mod triangle;
pub mod mesh_simple;

use ash::vk;
use crate::DeviceContext;

pub enum RenderContextType {
    Triangle,
    MeshSimple,
}

pub trait RenderContext {
    fn on_swapchain_resized(&self, new_size: vk::Extent2D) {
        
    }
    fn record_setup(&self, device_context: &DeviceContext, cmd : vk::CommandBuffer);
    fn record_draw(&self, device_context: &DeviceContext, cmd : vk::CommandBuffer, img: vk::Image, img_view: vk::ImageView, depth_img: vk::Image, depth_view: vk::ImageView, area: vk::Rect2D);
}
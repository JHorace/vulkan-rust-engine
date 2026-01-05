pub mod triangle;

use ash::vk;
use crate::DeviceContext;

pub enum RenderContextType {
    Triangle,
}

pub trait RenderContext {
    fn record_setup(&self, device_context: &DeviceContext, cmd : vk::CommandBuffer);
    fn record_draw(&self, device_context: &DeviceContext, cmd : vk::CommandBuffer, img: vk::Image, img_view: vk::ImageView, area: vk::Rect2D);
}
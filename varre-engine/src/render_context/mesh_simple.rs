use crate::DeviceContext;
use crate::command_buffers::record_image_layout_transition;
use crate::mesh_utils::VulkanMesh;
use crate::render_context::RenderContext;
use crate::shader_utils::create_shader_object;
use ash::vk;
use ash::vk::{CommandBuffer, Extent2D, Image, ImageView, Rect2D};
use varre_assets::{ModelID, ShaderID};

pub struct MeshSimpleRenderContext {
    vertex_shader: vk::ShaderEXT,
    fragment_shader: vk::ShaderEXT,
    depth_image: Option<vk::Image>,
}

impl MeshSimpleRenderContext {
    pub fn new(device_context: &DeviceContext) -> Self {
        let vert_shader_data = ShaderID::BASIC_MODEL_VERTEX.shader();
        let vertex_shader = create_shader_object(device_context, vert_shader_data);

        let frag_shader_data = ShaderID::BASIC_MODEL_FRAGMENT.shader();
        let fragment_shader = create_shader_object(device_context, frag_shader_data);

        let model = ModelID::CUBE.load();
        let mesh = VulkanMesh::from_model(device_context, &model);

        Self {
            vertex_shader,
            fragment_shader,
            depth_image: None,
        }
    }
}

impl RenderContext for MeshSimpleRenderContext {
    fn on_swapchain_resized(&self, new_size: Extent2D) {

    }

    fn record_setup(&self, device_context: &DeviceContext, cmd: CommandBuffer) {
        todo!()
    }

    fn record_draw(
        &self,
        device_context: &DeviceContext,
        cmd: CommandBuffer,
        img: Image,
        img_view: ImageView,
        area: Rect2D,
    ) {
        unsafe {
            record_image_layout_transition(
                &device_context.device,
                cmd,
                img,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::AccessFlags2::NONE,
                vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            );
        }
    }
}

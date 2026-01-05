use ash::vk;
use ash::vk::{CommandBuffer, SampleCountFlags};
use varre_assets::ShaderID;
use crate::command_buffers::record_image_layout_transition;
use crate::DeviceContext;
use crate::render_context::RenderContext;
use crate::shader_utils::create_shader_object;

pub struct TriangleRenderContext {
    triangle_vert: vk::ShaderEXT,
    triangle_frag: vk::ShaderEXT,
}

impl TriangleRenderContext {
    pub fn new(device_context: &DeviceContext) -> Self {
      
        let vert_shader = ShaderID::SHADER_TRIANGLE_VERTEX.shader();
        let triangle_vert = create_shader_object(device_context.shader_object_loader.as_ref().unwrap(), vert_shader);
        
        let frag_shader = ShaderID::SHADER_TRIANGLE_FRAGMENT.shader();
        let triangle_frag = create_shader_object(device_context.shader_object_loader.as_ref().unwrap(), frag_shader);
        
        Self{
           triangle_vert, triangle_frag
        }
    }
}

impl RenderContext for TriangleRenderContext {
    fn record_setup(&self, device_context: &DeviceContext, cmd: CommandBuffer) {
    }

    fn record_draw(&self, device_context: &DeviceContext, cmd : vk::CommandBuffer, img: vk::Image, img_view: vk::ImageView, area: vk::Rect2D) {
        unsafe {

            // Transition color target to write
            record_image_layout_transition(&device_context.device, cmd, img, vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::AccessFlags2::NONE, vk::AccessFlags2::COLOR_ATTACHMENT_WRITE, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT);

            // Begin rendering
            {
                let clear_color =  vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } };

                let attachment_info = [vk::RenderingAttachmentInfo::default()
                    .image_view(img_view)
                    .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(clear_color)];

                let rendering_info = vk::RenderingInfo::default()
                    .render_area(area)
                    .layer_count(1)
                    .color_attachments(&attachment_info);

                device_context.device.cmd_begin_rendering(cmd, &rendering_info);
            }

            // Set render state
            {
                let shaders = [self.triangle_vert, self.triangle_frag];
                let stages = [vk::ShaderStageFlags::VERTEX, vk::ShaderStageFlags::FRAGMENT];
                let shader_object_loader = device_context.shader_object_loader.as_ref()
                    .expect("shader_object_loader not available");
                shader_object_loader.cmd_bind_shaders(cmd, &stages, &shaders);

                // Setting viewport, scissor, and rasterizer discard is required before draw w/ shader object.
                let viewport = [vk::Viewport::default().width(area.extent.width as f32).height(area.extent.height as f32)];
                device_context.device.cmd_set_viewport_with_count(cmd, &viewport);
                let scissor = [vk::Rect2D::default().extent(area.extent)];
                device_context.device.cmd_set_scissor_with_count(cmd, &scissor);
                device_context.device.cmd_set_rasterizer_discard_enable(cmd, false);

                // Setting vertex input, primitive topology, primitive restart, and polygon mode is required before draw w/ shader object, if a vertex shader is bound.
                shader_object_loader.cmd_set_vertex_input(cmd, &[], &[]);
                shader_object_loader.cmd_set_primitive_topology(cmd, vk::PrimitiveTopology::TRIANGLE_LIST);
                shader_object_loader.cmd_set_primitive_restart_enable(cmd, false);

                // Required w/ shader object if rasterizer discard is disabled.
                shader_object_loader.cmd_set_rasterization_samples(cmd, vk::SampleCountFlags::TYPE_1);
                let sample_mask = [0x1];
                shader_object_loader.cmd_set_sample_mask(cmd, SampleCountFlags::TYPE_1, &sample_mask);
                shader_object_loader.cmd_set_alpha_to_coverage_enable(cmd, false);
                shader_object_loader.cmd_set_polygon_mode(cmd, vk::PolygonMode::FILL);
                device_context.device.cmd_set_line_width(cmd, 1.0);
                shader_object_loader.cmd_set_cull_mode(cmd, vk::CullModeFlags::BACK);
                shader_object_loader.cmd_set_front_face(cmd, vk::FrontFace::CLOCKWISE);
                shader_object_loader.cmd_set_depth_test_enable(cmd, false);
                shader_object_loader.cmd_set_depth_bounds_test_enable(cmd, false);
                shader_object_loader.cmd_set_depth_bias_enable(cmd, false);
                shader_object_loader.cmd_set_stencil_test_enable(cmd, false);

                // Required per bound color target
                let color_blend_enable = [vk::FALSE];
                shader_object_loader.cmd_set_color_blend_enable(cmd, 0, &color_blend_enable);
                let color_write_mask = [vk::ColorComponentFlags::RGBA];
                shader_object_loader.cmd_set_color_write_mask(cmd, 0, &color_write_mask);
            }

            device_context.device.cmd_draw(cmd, 3, 1, 0, 0);

            device_context.device.cmd_end_rendering(cmd);
            // Transition color target to don't care
            record_image_layout_transition(&device_context.device, cmd, img, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR, vk::AccessFlags2::COLOR_ATTACHMENT_WRITE, vk::AccessFlags2::NONE, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, vk::PipelineStageFlags2::BOTTOM_OF_PIPE);
        }
    }
}

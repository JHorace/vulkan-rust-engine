use ash::{vk, ext::{shader_object}, Device};
use ash::vk::SampleCountFlags;
use crate::{DeviceContext, VulkanEngine};
use crate::mesh_utils::VulkanMesh;

pub fn record_image_layout_transition(device: &Device, cmd : vk::CommandBuffer, img : vk::Image, old_layout : vk::ImageLayout, new_layout : vk::ImageLayout,
                                  src_access_mask : vk::AccessFlags2, dst_access_mask : vk::AccessFlags2,
                                  src_stage_mask : vk::PipelineStageFlags2, dst_stage_mask : vk::PipelineStageFlags2, subresource_range : vk::ImageSubresourceRange) {
   unsafe {
       let img_barrier = [vk::ImageMemoryBarrier2::default()
           .src_access_mask(src_access_mask)
           .dst_access_mask(dst_access_mask)
           .src_stage_mask(src_stage_mask)
           .dst_stage_mask(dst_stage_mask)
           .old_layout(old_layout)
           .new_layout(new_layout)
           .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
           .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
           .image(img)
           .subresource_range(subresource_range)];

       let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&img_barrier);

       device.cmd_pipeline_barrier2(cmd, &dependency_info);
   }
}

pub(crate) fn record_mesh_draw_setup(device_context: &DeviceContext, cmd: &vk::CommandBuffer, mesh: &VulkanMesh) {
   // Setup vertex and index buffers

}


pub(crate) fn record_draw(device_context: &DeviceContext, cmd : vk::CommandBuffer, img : vk::Image, img_view : vk::ImageView, area : vk::Rect2D, triangle_vert: vk::ShaderEXT, triangle_frag: vk::ShaderEXT) {
    unsafe {

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
            let shaders = [triangle_vert, triangle_frag];
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
    }
}
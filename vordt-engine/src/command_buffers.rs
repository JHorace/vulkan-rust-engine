use ash::{vk, ext::{shader_object}};
use ash::vk::SampleCountFlags;
use crate::VulkanEngine;

impl VulkanEngine {

    fn record_image_layout_transition(&self, cmd : vk::CommandBuffer, img : vk::Image, old_layout : vk::ImageLayout, new_layout : vk::ImageLayout,
    src_access_mask : vk::AccessFlags2, dst_access_mask : vk::AccessFlags2,
    src_stage_mask : vk::PipelineStageFlags2, dst_stage_mask : vk::PipelineStageFlags2) {
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
               .subresource_range(vk::ImageSubresourceRange::default()
                   .aspect_mask(vk::ImageAspectFlags::COLOR)
                   .base_mip_level(0)
                   .level_count(1)
                   .base_array_layer(0)
                   .layer_count(1))];

           let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&img_barrier);

           self.device.cmd_pipeline_barrier2(cmd, &dependency_info);
       }
    }
    fn record_draw(&self, cmd : vk::CommandBuffer, img : vk::Image, img_view : vk::ImageView, area : vk::Rect2D) {
        unsafe {

            // Transition color target to write
            self.record_image_layout_transition(cmd, img, vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::AccessFlags2::NONE, vk::AccessFlags2::COLOR_ATTACHMENT_WRITE, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT);

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

                self.device.cmd_begin_rendering(cmd, &rendering_info);
            }

            // Set render state
            {
                // Setting viewport, scissor, and rasterizer discard is required before draw w/ shader object.
                let viewport = [vk::Viewport::default().width(area.extent.width as f32).height(area.extent.height as f32)];
                self.device.cmd_set_viewport_with_count(cmd,  &viewport);
                let scissor = [vk::Rect2D::default().extent(area.extent)];
                self.device.cmd_set_scissor_with_count(cmd,  &scissor);
                self.device.cmd_set_rasterizer_discard_enable(cmd, false);

                // Setting vertex input, primitive topology, primitive restart, and polygon mode is required before draw w/ shader object, if a vertex shader is bound.
                self.shader_object_loader.cmd_set_vertex_input(cmd, &[], &[]);
                self.shader_object_loader.cmd_set_primitive_topology(cmd, vk::PrimitiveTopology::TRIANGLE_LIST);
                self.shader_object_loader.cmd_set_primitive_restart_enable(cmd, false);

                // Required w/ shader object if rasterizer discard is disabled.
                self.shader_object_loader.cmd_set_rasterization_samples(cmd, vk::SampleCountFlags::TYPE_1);
                let sample_mask = [0x1];
                self.shader_object_loader.cmd_set_sample_mask(cmd, SampleCountFlags::TYPE_1, &sample_mask);
                self.shader_object_loader.cmd_set_alpha_to_coverage_enable(cmd, false);
                self.shader_object_loader.cmd_set_polygon_mode(cmd, vk::PolygonMode::FILL);
                self.device.cmd_set_line_width(cmd, 1.0);
                self.shader_object_loader.cmd_set_cull_mode(cmd, vk::CullModeFlags::BACK);
                self.shader_object_loader.cmd_set_front_face(cmd, vk::FrontFace::CLOCKWISE);
                self.shader_object_loader.cmd_set_depth_test_enable(cmd, false);
                self.shader_object_loader.cmd_set_depth_bounds_test_enable(cmd, false);
                self.shader_object_loader.cmd_set_depth_bias_enable(cmd, false);
                self.shader_object_loader.cmd_set_stencil_test_enable(cmd, false);

                // Required per bound color target
                let color_blend_enable = [vk::FALSE];
                self.shader_object_loader.cmd_set_color_blend_enable(cmd, 0, &color_blend_enable);
                let color_write_mask = [vk::ColorComponentFlags::RGBA];
                self.shader_object_loader.cmd_set_color_write_mask(cmd, 0, &color_write_mask);
            }

            self.device.cmd_draw(cmd, 3, 1, 0, 0);

            // Transition color target to don't care
            self.record_image_layout_transition(cmd, img, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR, vk::AccessFlags2::COLOR_ATTACHMENT_WRITE, vk::AccessFlags2::NONE, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, vk::PipelineStageFlags2::BOTTOM_OF_PIPE);
        }
    }
}
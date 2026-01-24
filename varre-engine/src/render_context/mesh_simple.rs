use std::ptr::null;
use crate::DeviceContext;
use crate::command_buffers::record_image_layout_transition;
use crate::mesh_utils::VulkanMesh;
use crate::render_context::RenderContext;
use crate::shader_utils::{create_shader_object, make_descriptor_set_layouts};
use ash::vk;
use ash::vk::{CommandBuffer, Extent2D, Image, ImageView, PipelineBindPoint, Rect2D, SampleCountFlags};
use glam::Vec3;
use varre_assets::{ModelID, ShaderID};
use crate::memory_utils::{create_buffer, record_copy_buffer};

struct UBO {
    model: glam::Mat4,
    view: glam::Mat4,
    proj: glam::Mat4,
}

pub struct MeshSimpleRenderContext {
    vertex_shader: vk::ShaderEXT,
    fragment_shader: vk::ShaderEXT,
    mesh: VulkanMesh,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
    pipeline_layout: vk::PipelineLayout,
    uniform_buffer: vk::Buffer,
    uniform_buffer_memory: vk::DeviceMemory,
}

impl MeshSimpleRenderContext {
    pub fn new(device_context: &DeviceContext) -> Self {
        unsafe {

            let vert_shader_data = ShaderID::BASIC_MODEL_VERTEX.shader();
            let frag_shader_data = ShaderID::BASIC_MODEL_FRAGMENT.shader();
            let descriptor_set_layouts = make_descriptor_set_layouts(device_context, &[vert_shader_data, frag_shader_data]);
            
            let vertex_shader = create_shader_object(device_context, vert_shader_data, &descriptor_set_layouts);

            let fragment_shader = create_shader_object(device_context, frag_shader_data, &descriptor_set_layouts);

            let model = ModelID::CUBE.load();
            let mesh = VulkanMesh::from_model(device_context, &model);

            let pool_sizes = [vk::DescriptorPoolSize::default()
                .descriptor_count(32)
                .ty(vk::DescriptorType::UNIFORM_BUFFER)];

            let pool_create_info = vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(&pool_sizes)
                .max_sets(32);



            let descriptor_pool = device_context.device.create_descriptor_pool(&pool_create_info, None).unwrap();


            let descriptor_set_alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&descriptor_set_layouts);

            let descriptor_sets = device_context.device.allocate_descriptor_sets(&descriptor_set_alloc_info).unwrap();

            let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&descriptor_set_layouts);

            let pipeline_layout = device_context.device.create_pipeline_layout(&pipeline_layout_create_info, None).unwrap();

            let (uniform_buffer, uniform_buffer_memory) = create_buffer(device_context, size_of::<UBO>() as vk::DeviceSize, vk::BufferUsageFlags::UNIFORM_BUFFER, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);

            Self {
                vertex_shader,
                fragment_shader,
                mesh,
                descriptor_pool,
                descriptor_set: descriptor_sets[0],
                pipeline_layout,
                uniform_buffer,
                uniform_buffer_memory,
            }
        }
    }
}

impl RenderContext for MeshSimpleRenderContext {
    fn on_swapchain_resized(&self, new_size: Extent2D) {}

    fn record_setup(&self, device_context: &DeviceContext, cmd: CommandBuffer) {
        unsafe {
            record_copy_buffer(device_context, cmd, self.mesh.vertex_staging_buffer, self.mesh.vertex_buffer, self.mesh.vertex_buffer_size);
            record_copy_buffer(device_context, cmd, self.mesh.index_staging_buffer, self.mesh.index_buffer, self.mesh.index_buffer_size);

            let uboData_c= device_context.device.map_memory(self.uniform_buffer_memory, 0, size_of::<UBO>() as vk::DeviceSize, vk::MemoryMapFlags::empty()).unwrap();

            let uboData = &mut *(uboData_c as *mut UBO);

            uboData.model = glam::Mat4::IDENTITY;
            uboData.view = glam::Mat4::look_at_lh(glam::Vec3::new(2.0, 2.0, 2.0), glam::Vec3::new(0.0, 0.0, 0.0), glam::Vec3::new(0.0, 0.0, 1.0));
            uboData.proj = glam::Mat4::perspective_lh(f32::to_radians(45.0), 1920 as f32 / 1080 as f32, 0.1, 10.0);
            uboData.proj.col_mut(1).y *= -1.0;

            device_context.device.unmap_memory(self.uniform_buffer_memory);

            let ubo_descriptor = [vk::DescriptorBufferInfo { buffer: self.uniform_buffer, offset: 0, range: size_of::<UBO>() as vk::DeviceSize }];

            let write_descriptor_sets = [
                vk::WriteDescriptorSet::default()
                    .dst_set(self.descriptor_set)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&ubo_descriptor),
        ];

        device_context.device.update_descriptor_sets(&write_descriptor_sets, &[]);
        }

    }

    fn record_draw(
        &self,
        device_context: &DeviceContext,
        cmd: CommandBuffer,
        img: Image,
        img_view: ImageView,
        depth_img: Image,
        depth_view: ImageView,
        area: Rect2D,
    ) {
        unsafe {

            // Begin rendering
            {
                let clear_color = vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                };

                let attachment_info = [vk::RenderingAttachmentInfo::default()
                    .image_view(img_view)
                    .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(clear_color)];

                let depth_attachment_info = vk::RenderingAttachmentInfo::default()
                    .image_view(depth_view)
                    .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::DONT_CARE)
                    .clear_value(clear_color);

                let rendering_info = vk::RenderingInfo::default()
                    .render_area(area)
                    .layer_count(1)
                    .color_attachments(&attachment_info)
                    .depth_attachment(&depth_attachment_info);

                device_context
                    .device
                    .cmd_begin_rendering(cmd, &rendering_info);
            }

            // Set render state
            {
                let shaders = [self.vertex_shader, self.fragment_shader];
                let stages = [vk::ShaderStageFlags::VERTEX, vk::ShaderStageFlags::FRAGMENT];
                let shader_object_loader = device_context
                    .shader_object_loader
                    .as_ref()
                    .expect("shader_object_loader not available");
                shader_object_loader.cmd_bind_shaders(cmd, &stages, &shaders);

                // Setting viewport, scissor, and rasterizer discard is required before draw w/ shader object.
                let viewport = [vk::Viewport::default()
                    .width(area.extent.width as f32)
                    .height(area.extent.height as f32)
                    .min_depth(0.0)
                    .max_depth(1.0)];

                device_context
                    .device
                    .cmd_set_viewport_with_count(cmd, &viewport);
                let scissor = [vk::Rect2D::default().extent(area.extent)];
                device_context
                    .device
                    .cmd_set_scissor_with_count(cmd, &scissor);
                device_context
                    .device
                    .cmd_set_rasterizer_discard_enable(cmd, false);

                let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription2EXT::default()
                    .binding(0)
                    .stride(std::mem::size_of::<Vec3>() as u32)
                    .input_rate(vk::VertexInputRate::VERTEX)
                    .divisor(1)];

                let vertex_attribute_descriptions = [vk::VertexInputAttributeDescription2EXT::default()
                    .binding(0)
                    .location(0)
                    .format(vk::Format::R32G32B32_SFLOAT)
                    .offset(0)];

                // Setting vertex input, primitive topology, primitive restart, and polygon mode is required before draw w/ shader object, if a vertex shader is bound.
                shader_object_loader.cmd_set_vertex_input(cmd, &vertex_input_binding_descriptions, &vertex_attribute_descriptions);
                shader_object_loader
                    .cmd_set_primitive_topology(cmd, vk::PrimitiveTopology::TRIANGLE_LIST);
                shader_object_loader.cmd_set_primitive_restart_enable(cmd, false);

                // Required w/ shader object if rasterizer discard is disabled.
                shader_object_loader
                    .cmd_set_rasterization_samples(cmd, vk::SampleCountFlags::TYPE_1);
                let sample_mask = [0x1];
                shader_object_loader.cmd_set_sample_mask(
                    cmd,
                    SampleCountFlags::TYPE_1,
                    &sample_mask,
                );
                shader_object_loader.cmd_set_alpha_to_coverage_enable(cmd, false);
                shader_object_loader.cmd_set_polygon_mode(cmd, vk::PolygonMode::FILL);
                device_context.device.cmd_set_line_width(cmd, 1.0);
                shader_object_loader.cmd_set_cull_mode(cmd, vk::CullModeFlags::BACK);
                shader_object_loader.cmd_set_front_face(cmd, vk::FrontFace::CLOCKWISE);
                shader_object_loader.cmd_set_depth_test_enable(cmd, true);
                shader_object_loader.cmd_set_depth_bounds_test_enable(cmd, false);
                shader_object_loader.cmd_set_depth_bias_enable(cmd, false);
                shader_object_loader.cmd_set_stencil_test_enable(cmd, false);
                shader_object_loader.cmd_set_depth_compare_op(cmd, vk::CompareOp::GREATER);

                shader_object_loader.cmd_set_depth_write_enable(cmd, true);

                // Required per bound color target
                let color_blend_enable = [vk::FALSE];
                shader_object_loader.cmd_set_color_blend_enable(cmd, 0, &color_blend_enable);
                let color_write_mask = [vk::ColorComponentFlags::RGBA];
                shader_object_loader.cmd_set_color_write_mask(cmd, 0, &color_write_mask);

                let vertex_buffers = [self.mesh.vertex_buffer];
                let offsets = [0];
                let dynamic_offsets : &[u32] = &[];

                shader_object_loader.cmd_bind_vertex_buffers2(cmd, 0, &vertex_buffers, &offsets, None, None);
                device_context.device.cmd_bind_index_buffer(cmd, self.mesh.index_buffer, 0, vk::IndexType::UINT32);
                device_context.device.cmd_bind_descriptor_sets(cmd, PipelineBindPoint::GRAPHICS, self.pipeline_layout, 0, &[self.descriptor_set], &dynamic_offsets);

            }

            device_context.device.cmd_draw_indexed(cmd, self.mesh.index_count, 1, 0, 0, 0);

            device_context.device.cmd_end_rendering(cmd);
        }
    }
}

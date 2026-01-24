use crate::DeviceContext;
use crate::command_buffers::record_image_layout_transition;
use crate::physical_device_utils::find_memorytype_index;
use crate::render_context::RenderContext;
use ash::vk::Semaphore;
use ash::{
    khr::{surface, swapchain},
    vk,
};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

//Swapchain is created by Device, owns multiple Device-created images, and uses device-level
//functions. As such, it must not outlive the device. It holds an additional Instance reference
//for convenience. This does not require a second explicit lifetime as the lifetime of the instance
//should exceed the device.
pub struct VulkanWindow {
    pub vk_surface: vk::SurfaceKHR,
    pub vk_swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_image_views: Vec<vk::ImageView>,
    pub depth_image: vk::Image,
    pub depth_image_view: vk::ImageView,
    pub depth_image_memory: vk::DeviceMemory,
    pub present_complete_semaphores: [vk::Semaphore; 3],
    pub rendering_complete_semaphores: [vk::Semaphore; 3],
    pub frame_fences: [vk::Fence; 3],
    frame_index: usize,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
}

impl VulkanWindow {
    pub fn new(
        device_context: &crate::DeviceContext,
        display_window_handle: (RawDisplayHandle, RawWindowHandle),
        extent: vk::Extent2D,
    ) -> Self {
        unsafe {
            let swapchain_loader = &device_context.swapchain_loader;
            let (display_handle, window_handle) = display_window_handle;

            let surface = ash_window::create_surface(
                &device_context.entry,
                &device_context.instance,
                display_handle,
                window_handle,
                None,
            )
            .unwrap();

            let surface_format = device_context
                .surface_loader
                .get_physical_device_surface_formats(device_context.physical_device, surface)
                .unwrap()[0];

            let swapchain =
                create_swapchain(device_context, surface, surface_format.format, extent);

            let swapchain_images = get_swapchain_images(device_context, swapchain);

            let swapchain_image_views =
                get_swapchain_image_views(device_context, &swapchain_images, surface_format.format);

            let (depth_image, depth_image_view, depth_image_memory) = create_depth_resources(device_context, extent);

            let present_complete_semaphores: [Semaphore; 3] = unsafe {
                std::array::from_fn(|_| {
                    device_context
                        .device
                        .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                        .expect("failed to create semaphore")
                })
            };

            let rendering_complete_semaphores: [Semaphore; 3] = unsafe {
                std::array::from_fn(|_| {
                    device_context
                        .device
                        .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                        .expect("failed to create semaphore")
                })
            };

            let fence_create_info =
                vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

            let frame_fences = unsafe {
                std::array::from_fn(|_| {
                    device_context
                        .device
                        .create_fence(&fence_create_info, None)
                        .expect("failed to create fence")
                })
            };

            VulkanWindow {
                vk_surface: surface,
                vk_swapchain: swapchain,
                swapchain_images,
                swapchain_image_views,
                depth_image,
                depth_image_view,
                depth_image_memory,
                rendering_complete_semaphores,
                present_complete_semaphores,
                frame_fences,
                frame_index: 0,
                format: surface_format.format,
                extent,
            }
        }
    }

    pub fn initialize_images(&self, device_context: &crate::DeviceContext, cmd: vk::CommandBuffer) {
        record_image_layout_transition(
            &device_context.device,
            cmd,
            self.depth_image.clone(),
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            vk::AccessFlags2::NONE,
            vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
            vk::PipelineStageFlags2::TOP_OF_PIPE,
            vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
            vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::DEPTH)
                .layer_count(1)
                .level_count(1),
        );
    }

    pub fn on_window_resized(
        &mut self,
        device_context: &crate::DeviceContext,
        new_extent: vk::Extent2D,
    ) {
        unsafe {
            device_context.device.device_wait_idle().unwrap();
            for view in self.swapchain_image_views.iter() {
                device_context.device.destroy_image_view(*view, None);
            }
            device_context
                .swapchain_loader
                .destroy_swapchain(self.vk_swapchain, None);

            device_context.device.destroy_image_view(self.depth_image_view, None);
            device_context.device.destroy_image(self.depth_image, None);
            device_context.device.free_memory(self.depth_image_memory, None);

            let (depth_image, depth_image_view, depth_image_memory) = create_depth_resources(device_context, new_extent);

            self.vk_swapchain =
                create_swapchain(device_context, self.vk_surface, self.format, new_extent);
            self.swapchain_images = get_swapchain_images(device_context, self.vk_swapchain);
            self.swapchain_image_views =
                get_swapchain_image_views(device_context, &self.swapchain_images, self.format);
            self.depth_image = depth_image;
            self.depth_image_view = depth_image_view;
            self.depth_image_memory = depth_image_memory;
            self.extent = new_extent;
        };
    }

    pub fn render_frame(
        &mut self,
        device_context: &DeviceContext,
        cmd: vk::CommandBuffer,
        render_context: &Box<dyn RenderContext>,
    ) {
        unsafe {
            let frame_fence = self.frame_fences[self.frame_index];
            let present_complete_semaphore = self.present_complete_semaphores[self.frame_index];
            device_context
                .device
                .wait_for_fences(&[frame_fence], true, u64::MAX)
                .unwrap();
            device_context.device.reset_fences(&[frame_fence]).unwrap();

            let (present_index, suboptimal) = device_context
                .swapchain_loader
                .acquire_next_image(
                    self.vk_swapchain,
                    u64::MAX,
                    present_complete_semaphore,
                    vk::Fence::null(),
                )
                .unwrap();

            if suboptimal {
                println!("suboptimal swapchain image acquired");
            }

            device_context
                .device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::RELEASE_RESOURCES)
                .expect("failed to reset command buffer");

            let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            device_context
                .device
                .begin_command_buffer(cmd, &command_buffer_begin_info)
                .expect("failed to begin command buffer");

            record_image_layout_transition(
                &device_context.device,
                cmd,
                self.swapchain_images[present_index as usize],
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::AccessFlags2::NONE,
                vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );

            record_image_layout_transition(
                &device_context.device,
                cmd,
                self.depth_image.clone(),
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                vk::AccessFlags2::NONE,
                vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ
                    | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                vk::PipelineStageFlags2::TOP_OF_PIPE,
                vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::DEPTH)
                    .layer_count(1)
                    .level_count(1),
            );

            render_context.record_draw(device_context, cmd, self.swapchain_images[present_index as usize], self.swapchain_image_views[present_index as usize], self.depth_image, self.depth_image_view, vk::Rect2D::default().extent(self.extent));

            record_image_layout_transition(&device_context.device, cmd, self.swapchain_images[present_index as usize], vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR, vk::AccessFlags2::COLOR_ATTACHMENT_WRITE, vk::AccessFlags2::NONE, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1));

            device_context
                .device
                .end_command_buffer(cmd)
                .expect("failed to end recording command buffer");


            let rendering_complete_semaphore =
                self.rendering_complete_semaphores[present_index as usize];

            let command_buffers = vec![cmd];
            let wait_semaphores = vec![present_complete_semaphore];
            let signal_semaphores = vec![rendering_complete_semaphore];
            let wait_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

            let submit_info = vk::SubmitInfo::default()
                .command_buffers(&command_buffers)
                .wait_semaphores(&wait_semaphores)
                .signal_semaphores(&signal_semaphores)
                .wait_dst_stage_mask(&wait_stage_mask);

            device_context
                .device
                .queue_submit(device_context.graphics_queue, &[submit_info], frame_fence)
                .expect("failed to submit command buffer");

            let swapchains = vec![self.vk_swapchain];

            let image_indices = vec![present_index];
            let wait_semaphores = vec![rendering_complete_semaphore];

            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&wait_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            device_context
                .swapchain_loader
                .queue_present(device_context.graphics_queue, &present_info)
                .unwrap();

            self.frame_index = (self.frame_index + 1) % 3;
        }
    }
}

fn create_swapchain(
    device_context: &crate::DeviceContext,
    surface: vk::SurfaceKHR,
    format: vk::Format,
    extent: vk::Extent2D,
) -> vk::SwapchainKHR {
    let surface_capabilities = unsafe {
        device_context
            .surface_loader
            .get_physical_device_surface_capabilities(device_context.physical_device, surface)
            .unwrap()
    };

    let present_modes = unsafe {
        device_context
            .surface_loader
            .get_physical_device_surface_present_modes(device_context.physical_device, surface)
            .unwrap()
    };
    // Use swapchain loader from device context
    let swapchain_loader = &device_context.swapchain_loader;

    let surface_resolution = match surface_capabilities.current_extent.width {
        u32::MAX => extent,
        _ => surface_capabilities.current_extent,
    };

    let pre_transform = if surface_capabilities
        .supported_transforms
        .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
    {
        vk::SurfaceTransformFlagsKHR::IDENTITY
    } else {
        surface_capabilities.current_transform
    };

    let present_mode = present_modes
        .iter()
        .cloned()
        .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO);

    let mut desired_image_count = surface_capabilities.min_image_count + 1;

    if (surface_capabilities.max_image_count > 0
        && desired_image_count > surface_capabilities.max_image_count)
    {
        desired_image_count = surface_capabilities.max_image_count;
    }

    let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(desired_image_count)
        .image_format(format)
        .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
        .image_extent(surface_resolution)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(pre_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .image_array_layers(1);

    unsafe {
        swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .expect("Failed to create swapchain")
    }
}

fn create_depth_resources(device_context: &DeviceContext, extent: vk::Extent2D) -> (vk::Image, vk::ImageView, vk::DeviceMemory) {
   unsafe {

       let depth_image_create_info = vk::ImageCreateInfo::default()
       .image_type(vk::ImageType::TYPE_2D)
       .extent(vk::Extent3D {
           width: extent.width,
           height: extent.height,
           depth: 1,
       })
       .mip_levels(1)
       .array_layers(1)
       .format(vk::Format::D32_SFLOAT)
       .tiling(vk::ImageTiling::OPTIMAL)
       .initial_layout(vk::ImageLayout::UNDEFINED)
       .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
       .samples(vk::SampleCountFlags::TYPE_1)
       .sharing_mode(vk::SharingMode::EXCLUSIVE);

       let depth_image = device_context
           .device
           .create_image(&depth_image_create_info, None)
           .unwrap();

       let memory_requirements = device_context
           .device
           .get_image_memory_requirements(depth_image);
       let memory_allocate_info = vk::MemoryAllocateInfo::default()
           .allocation_size(memory_requirements.size)
           .memory_type_index(
               find_memorytype_index(
                   &device_context,
                   &memory_requirements,
                   vk::MemoryPropertyFlags::DEVICE_LOCAL,
               )
                   .expect("Could not find suitable memory type for depth image"),
           );

       let depth_image_memory = device_context
           .device
           .allocate_memory(&memory_allocate_info, None)
           .unwrap();

       device_context
           .device
           .bind_image_memory(depth_image, depth_image_memory, 0)
           .unwrap();

       let depth_image_view_info = vk::ImageViewCreateInfo::default()
           .subresource_range(
               vk::ImageSubresourceRange::default()
                   .aspect_mask(vk::ImageAspectFlags::DEPTH)
                   .level_count(1)
                   .layer_count(1),
           )
           .image(depth_image)
           .format(depth_image_create_info.format)
           .view_type(vk::ImageViewType::TYPE_2D);

       let depth_image_view = device_context
           .device
           .create_image_view(&depth_image_view_info, None)
           .unwrap();

       (depth_image, depth_image_view, depth_image_memory)
   }
}

fn get_swapchain_images(
    device_context: &crate::DeviceContext,
    swapchain: vk::SwapchainKHR,
) -> Vec<vk::Image> {
    unsafe {
        device_context
            .swapchain_loader
            .get_swapchain_images(swapchain)
            .expect("Failed to get swapchain images")
    }
}

fn get_swapchain_image_views(
    device_context: &DeviceContext,
    images: &[vk::Image],
    format: vk::Format,
) -> Vec<vk::ImageView> {
    unsafe {
        images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo::default()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);
                device_context
                    .device
                    .create_image_view(&create_view_info, None)
                    .unwrap()
            })
            .collect()
    }
}

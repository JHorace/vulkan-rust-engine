use ash::{
    khr::{surface, swapchain},
    vk,
};
use ash::vk::Semaphore;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use crate::DeviceContext;

//Swapchain is created by Device, owns multiple Device-created images, and uses device-level 
//functions. As such, it must not outlive the device. It holds an additional Instance reference
//for convenience. This does not require a second explicit lifetime as the lifetime of the instance
//should exceed the device.
pub struct VulkanWindow {
    pub vk_surface: vk::SurfaceKHR,
    pub vk_swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_image_views: Vec<vk::ImageView>,
    pub present_complete_semaphores: [vk::Semaphore; 3],
    pub rendering_complete_semaphores: [vk::Semaphore; 3],
    pub frame_fences: [vk::Fence; 3],
    frame_index: usize,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
}

impl VulkanWindow {
    
    pub fn new(device_context: &crate::DeviceContext, display_window_handle: (RawDisplayHandle, RawWindowHandle), extent: vk::Extent2D) -> Self {
 
        unsafe {
            
            let swapchain_loader = &device_context.swapchain_loader;
        let (display_handle, window_handle) = display_window_handle;
        
        let surface =
            ash_window::create_surface(
                &device_context.entry,
                &device_context.instance,
                display_handle,
                window_handle,
                None,
            ).unwrap();

            let surface_format = unsafe {
                device_context.surface_loader
                    .get_physical_device_surface_formats(device_context.physical_device, surface)
                    .unwrap()[0]
            };
            
            let swapchain = create_swapchain(device_context, surface, surface_format.format, extent);
            
            let swapchain_images = get_swapchain_images(device_context, swapchain);
            
            let swapchain_image_views = get_swapchain_image_views(device_context, &swapchain_images, surface_format.format);

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

            VulkanWindow {
                vk_surface: surface,
                vk_swapchain: swapchain,
                swapchain_images,
                swapchain_image_views,
                rendering_complete_semaphores,
                present_complete_semaphores,
                format: surface_format.format,
                extent,
            }
        }
    }
   
    pub fn on_window_resized(&mut self, device_context: &crate::DeviceContext, new_extent: vk::Extent2D) {
        unsafe {
            device_context.device.device_wait_idle().unwrap();
            for view in self.swapchain_image_views.iter() {
            device_context.device.destroy_image_view(*view, None);
            }
            device_context.swapchain_loader.destroy_swapchain(self.vk_swapchain, None);
            
            self.vk_swapchain = create_swapchain(device_context, self.vk_surface, self.format, new_extent);
            self.swapchain_images = get_swapchain_images(device_context, self.vk_swapchain);
            self.swapchain_image_views = get_swapchain_image_views(device_context, &self.swapchain_images, self.format);
            self.extent = new_extent;
        };
    }

    pub fn render_frame(&self, device_context: &DeviceContext, cmd: vk::CommandBuffer)
    {
        unsafe {
            let draw_command_buffer_fence = self.frame_fences[self.frame_index];
            let present_complete_semaphore = self.present_complete_semaphores[self.frame_index];
            device_context
                .device
                .wait_for_fences(&[draw_command_buffer_fence], true, u64::MAX)
                .unwrap();
            device_context
                .device
                .reset_fences(&[draw_command_buffer_fence])
                .unwrap();

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
        }
    }
  
}

fn create_swapchain(device_context: &crate::DeviceContext, surface: vk::SurfaceKHR, format: vk::Format, extent: vk::Extent2D) -> vk::SwapchainKHR {


    let surface_capabilities = unsafe {
        device_context.surface_loader
            .get_physical_device_surface_capabilities(device_context.physical_device, surface)
            .unwrap()
    };

    let present_modes = unsafe {
        device_context.surface_loader
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
        .contains(vk::SurfaceTransformFlagsKHR::IDENTITY) {
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

    if(surface_capabilities.max_image_count > 0 &&
        desired_image_count > surface_capabilities.max_image_count)
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

fn get_swapchain_images(device_context: &crate::DeviceContext, swapchain: vk::SwapchainKHR) -> Vec<vk::Image> {
    unsafe {
        device_context.swapchain_loader
            .get_swapchain_images(swapchain)
            .expect("Failed to get swapchain images")
    }
}

fn get_swapchain_image_views(device_context: &DeviceContext, images: &[vk::Image], format: vk::Format) -> Vec<vk::ImageView> {
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
                device_context.device.create_image_view(&create_view_info, None).unwrap()
            })
            .collect()
    }
}
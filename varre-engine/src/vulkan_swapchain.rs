use ash::{
    khr::{surface, swapchain},
    vk,
};

//Swapchain is created by Device, owns multiple Device-created images, and uses device-level 
//functions. As such, it must not outlive the device. It holds an additional Instance reference
//for convenience. This does not require a second explicit lifetime as the lifetime of the instance
//should exceed the device.
pub struct Swapchain {
    pub vk_swapchain: vk::SwapchainKHR,
    pub loader: swapchain::Device,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
    pub extent: vk::Extent2D,
}

impl Swapchain {
    pub fn new(
        instance: &ash::Instance,
        device:   &ash::Device,
        surface: vk::SurfaceKHR,
        window_size: vk::Extent2D,
        surface_capabilities: vk::SurfaceCapabilitiesKHR,
        present_modes: Vec<vk::PresentModeKHR>,
        format: vk::SurfaceFormatKHR,
    ) -> Self {
        // Load swapchain functions
        let swapchain_loader = swapchain::Device::new(&instance, &device);

    let surface_resolution = match surface_capabilities.current_extent.width {
        u32::MAX => window_size,
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
            .image_format(format.format)
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_extent(surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);




        let swapchain : vk::SwapchainKHR = unsafe {
            swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .expect("Failed to create swapchain")
        };

        let present_images = unsafe {
           swapchain_loader.get_swapchain_images(swapchain)
               .expect("Failed to get swapchain images")
        };

        let present_image_views: Vec<vk::ImageView> = unsafe {
            present_images
                .iter()
                .map(|&image| {
                    let create_view_info = vk::ImageViewCreateInfo::default()
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(format.format)
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
                    device.create_image_view(&create_view_info, None).unwrap()
                })
                .collect()
        };

        Swapchain {
            vk_swapchain: swapchain,
            loader: swapchain_loader,
            images: present_images,
            image_views: present_image_views,
            extent: surface_resolution,
        }
    }
}

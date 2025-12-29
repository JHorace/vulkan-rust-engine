use ash::vk;

#[derive(Copy, Clone)]
pub struct QueueFamilyIndices {
    pub graphics_general: Option<u32>,
    pub async_compute: Option<u32>,
    pub transfer: Option<u32>,
}

impl QueueFamilyIndices {
    //TODO: This is inefficient

    pub fn new(queue_family_properties: &Vec<vk::QueueFamilyProperties>) -> Self {
        Self {
            //Find the first queue family that supports both graphics and compute.
            graphics_general: queue_family_properties.iter().enumerate().find_map(
                |(index, info)| {
                    (info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                        && info.queue_flags.contains(vk::QueueFlags::COMPUTE))
                        .then_some(index as u32)
                },
            ),
            //Find the first dedicated compute queue family - that does not support graphics.
            async_compute: queue_family_properties
                .iter()
                .enumerate()
                .find_map(|(index, info)| {
                    (info.queue_flags.contains(vk::QueueFlags::COMPUTE)
                        && !info.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                        .then_some(index as u32)
                }),
            //Find the first dedicated transfer queue family - that does not support graphics or compute.
            transfer: queue_family_properties
                .iter()
                .enumerate()
                .find_map(|(index, info)| {
                    (info.queue_flags.contains(vk::QueueFlags::TRANSFER)
                        && !info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                        && !info.queue_flags.contains(vk::QueueFlags::COMPUTE))
                        .then_some(index as u32)
                }),
        }
    }
}

pub fn get_physical_devices_supporting_surface(physical_devices: Vec<vk::PhysicalDevice>, instance: &ash::Instance, surface: vk::SurfaceKHR, surface_loader: &ash::khr::surface::Instance) -> Vec<vk::PhysicalDevice>
{
    unsafe {
        physical_devices
            .into_iter()
            .filter(|&physical_device| {
                instance
                    .get_physical_device_queue_family_properties(physical_device)
                    .iter()
                    .enumerate()
                    .any(|(index, _)| {
                        surface_loader
                            .get_physical_device_surface_support(
                                physical_device,
                                index as u32,
                                surface,
                            )
                            .unwrap()
                    })
            })
            .collect()
    }
}

pub fn select_physical_device(physical_devices: Vec<vk::PhysicalDevice>, instance: &ash::Instance) -> vk::PhysicalDevice {
    unsafe {
        physical_devices
            .into_iter()
            .find(|&physical_device| {
                instance
                    .get_physical_device_properties(physical_device)
                    .device_type
                    == vk::PhysicalDeviceType::DISCRETE_GPU
            })
            .expect("failed to find a suitable GPU")
    }
}
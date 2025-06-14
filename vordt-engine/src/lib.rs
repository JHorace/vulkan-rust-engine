use ash::{Entry, vk};
use std::borrow::Cow;
use std::{error::Error, ffi, os::raw::c_char};
use ash::ext::debug_utils;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    p_user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    //TODO: It should be possible to make this function safe if we can determine the maximum length
    //      of the message - If so we can check if the string in null terminated.

    let callback_data = unsafe { *p_callback_data };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        unsafe { ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy() }
    };

    println!("{message_severity:?}:\n{message_type:?} : {message}\n", );

    vk::FALSE
}

//TODO: I could support multiple window handles here, or a headless mode.
fn create_instance(
    enable_validation: bool,
    display_handle: Option<&RawDisplayHandle>,
    loader: &Entry,
) -> Result<ash::Instance, Box<dyn Error>> {
    //todo: app name, version, etc.
    let application_info = vk::ApplicationInfo::default()
        .application_name(c"vordt-engine")
        .application_version(0)
        .engine_name(c"vordt-engine")
        .engine_version(0)
        .api_version(vk::make_api_version(0, 1, 3, 0));

    let layer_names = if enable_validation {
        vec![c"VK_LAYER_KHRONOS_validation"]
    } else {
        vec![]
    };
    let layer_names_raw: Vec<*const c_char> =
        layer_names.iter().map(|name| name.as_ptr()).collect();

    let mut extension_names = if display_handle.is_some() {
        ash_window::enumerate_required_extensions(*display_handle.unwrap())
            .expect("failed to enumerate required extensions")
            .to_vec()
    } else {
        vec![]
    };


    enable_validation.then(|| {
        extension_names.push(debug_utils::NAME.as_ptr());
    });

    let instance_create_flags = vk::InstanceCreateFlags::default();

    let instance_create_info = vk::InstanceCreateInfo::default()
        .application_info(&application_info)
        .enabled_extension_names(&extension_names)
        .enabled_layer_names(&layer_names_raw)
        .flags(instance_create_flags);

    unsafe {
        loader
            .create_instance(&instance_create_info, None)
            .map_err(|e| e.into())
    }
}

struct QueueFamilyIndices {
    graphics_general: Option<u32>,
    async_compute: Option<u32>,
    transfer: Option<u32>,
}

impl QueueFamilyIndices {
    //TODO: This is inefficient

    fn new(queue_family_properties: &Vec<vk::QueueFamilyProperties>) -> Self {
        Self {
            //Find the first queue family that supports both graphics and compute.
            graphics_general: queue_family_properties
                .iter()
                .enumerate()
                .find_map(|(index, info)| {
                    (info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                        && info.queue_flags.contains(vk::QueueFlags::COMPUTE))
                        .then_some(index as u32)
                }),
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

fn create_device(
    instance: &ash::Instance,
    display_handle: RawDisplayHandle,
    window_handle: RawWindowHandle,
    enable_validation: bool,
    loader: &Entry,
) -> Result<ash::Device, Box<dyn Error>> {


    //Create a surface early so we can determine whether physical devices support it
    let surface = unsafe {
        ash_window::create_surface(loader, instance, display_handle, window_handle, None)
            .expect("failed to create surface")
    };

    let surface_loader = unsafe { ash::khr::surface::Instance::new(loader, instance) };

    //Get the list of available physical devices
    let physical_devices = unsafe { instance.enumerate_physical_devices().expect("failed to enumerate physical devices") };

    //Filter the list to only include physical devices that support the surface
    let surface_supported_devices: Vec<ash::vk::PhysicalDevice> = unsafe {
        physical_devices.into_iter()
            .filter(|&physical_device|
                instance.get_physical_device_queue_family_properties(physical_device)
                    .iter()
                    .enumerate()
                    .any(|(index, queue_family_props)|
                        surface_loader.get_physical_device_surface_support(physical_device, index as u32, surface)
                            .unwrap()))
            .collect()
    };

    //Find the first physical device that is a discrete GPU
    let selected_device = unsafe {
        surface_supported_devices
            .into_iter()
            .find(|&physical_device|
                instance.get_physical_device_properties(physical_device)
                    .device_type == vk::PhysicalDeviceType::DISCRETE_GPU)
            .expect("failed to find a suitable GPU")
    };

    //Choose queue family indices for the device
    let queue_family_indices = unsafe {
        QueueFamilyIndices::new(
            &instance.get_physical_device_queue_family_properties(selected_device))
    };

    let mut queue_create_infos : Vec<vk::DeviceQueueCreateInfo> = vec![];

    if let Some(graphics_index) = queue_family_indices.graphics_general {
        queue_create_infos.push(vk::DeviceQueueCreateInfo::default()
            .queue_family_index(graphics_index));
    }

    if let Some(compute_index) = queue_family_indices.async_compute {
        queue_create_infos.push(vk::DeviceQueueCreateInfo::default()
            .queue_family_index(compute_index));
    }

    if let Some(transfer_index) = queue_family_indices.transfer {
        queue_create_infos.push(vk::DeviceQueueCreateInfo::default()
            .queue_family_index(transfer_index));
    }
    
    let device_extension_names_raw = [
        ash::khr::swapchain::NAME.as_ptr(),
    ];

    enable_validation.then(|| {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vulkan_debug_callback));

        let debug_utils_loader = debug_utils::Instance::new(loader, &instance);
        let debug_callback = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap()
        };
    });

    let device_create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&device_extension_names_raw);
    
    unsafe {
        instance
            .create_device(selected_device, &device_create_info, None)
            .map_err(|e| e.into())
    }
}

pub struct VulkanEngine {}

impl VulkanEngine {
    pub fn new(enable_validation: bool, display_handle: Option<RawDisplayHandle>,
               window_handle: Option<RawWindowHandle>) -> Result<Self, Box<dyn Error>> {
        //Load entry point
        //'linked' here means compile-time static linkage against vulkan development libraries.
        let entry = unsafe { Entry::load()? };

        let instance = { create_instance(enable_validation, display_handle.as_ref(), &entry)? };

        Ok(VulkanEngine {})
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_instance() {
        let entry = unsafe { Entry::load().expect("failed to load vulkan module") };

        create_instance(true, None, &entry).expect("Failed to create VordtEngine instance");
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

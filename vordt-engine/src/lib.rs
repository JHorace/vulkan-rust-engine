mod swapchain;

use ash::{Entry, ext::debug_utils, vk};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::borrow::Cow;
use std::{error::Error, ffi, os::raw::c_char};

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    //TODO: It should be possible to make this function safe if we can determine the maximum length
    //      of the message - If so we can check if the string in null terminated.

    let callback_data = unsafe { *p_callback_data };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        unsafe { ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy() }
    };

    println!("{message_severity:?}:\n{message_type:?} : {message}\n",);

    vk::FALSE
}

//TODO: I could support multiple window handles here, or a headless mode.
//Debated whether to pass in a list of window/surface extensions or a display handle.
//I went with the display handle - conceptually the app is saying "I'm going to create these types
//of windows," and the engine is determining what it needs to do that.
//I don't think it is at all likely we'd need to support multiple display types - though xlib and
//wayland may be possible.
//TODO: Convert display_handle to a vector of display handles.
fn create_instance(
    enable_validation: bool,
    display_handle: Option<RawDisplayHandle>,
    loader: &Entry,
) -> Result<ash::Instance, Box<dyn Error>> {
    //todo: app name, version, etc.
    let application_info = vk::ApplicationInfo::default()
        .application_name(c"vordt-engine")
        .application_version(0)
        .engine_name(c"vordt-engine")
        .engine_version(0)
        .api_version(vk::make_api_version(0, 1, 3, 0));

    let layer_names_raw: Vec<*const c_char> = if enable_validation {
        vec![c"VK_LAYER_KHRONOS_validation"]
            .iter()
            .map(|name| name.as_ptr())
            .collect()
    } else {
        Vec::new()
    };

    let mut extension_names = display_handle.map_or(Vec::new(), |handle| {
        ash_window::enumerate_required_extensions(handle)
            .expect("failed to enumerate required extensions")
            .to_vec()
    });

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

//If display_window_handles is None, then we're in a headless state.
//The engine should always be able to render to something, so it should always have a device.
fn create_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    extension_names: Vec<*const c_char>,
    enable_validation: bool,
    loader: &Entry,
) -> Result<ash::Device, Box<dyn Error>> {
    //Choose queue family indices for the device
    let queue_family_indices = unsafe {
        QueueFamilyIndices::new(
            &instance.get_physical_device_queue_family_properties(physical_device),
        )
    };

    let mut queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = vec![];
    let queue_priorities = [1.0];

    if let Some(graphics_index) = queue_family_indices.graphics_general {
        queue_create_infos.push(
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(graphics_index)
                .queue_priorities(&queue_priorities),
        );
    }

    if let Some(compute_index) = queue_family_indices.async_compute {
        queue_create_infos.push(
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(compute_index)
                .queue_priorities(&queue_priorities),
        );
    }

    if let Some(transfer_index) = queue_family_indices.transfer {
        queue_create_infos.push(
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(transfer_index)
                .queue_priorities(&queue_priorities),
        );
    }

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
        .enabled_extension_names(&extension_names);

    unsafe {
        instance
            .create_device(physical_device, &device_create_info, None)
            .map_err(|e| e.into())
    }
}

pub struct VulkanEngine {
    entry: Entry,
    instance: ash::Instance,
    device: ash::Device,
    surface_loader: ash::khr::surface::Instance,
    swapchain: Option<swapchain::Swapchain>,
}

impl VulkanEngine {
    pub fn new(
        enable_validation: bool,
        display_window_handles: Option<(RawDisplayHandle, RawWindowHandle)>,
    ) -> Result<Self, Box<dyn Error>> {
        //Load entry point
        //'linked' here means compile-time static linkage against vulkan development libraries.
        let entry = Entry::linked();

        let (display_handle, window_handle) = display_window_handles
            .map(|(d, w)| (Some(d), Some(w)))
            .unwrap_or((None, None));

        //TODO: should we panic if this fails? Depends on whether there is anything the engine or the
        //      user can do to fix instance creation (update paths to missing layers, etc.)
        let instance = create_instance(enable_validation, display_handle, &entry)?;

        //We'll load surface functions regardless of whether we have a surface or not.
        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

        //Get the list of available physical devices
        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .expect("failed to enumerate physical devices")
        };

        //Create a surface if we have a display and window handle
        //If we don't then we're headless. This is a valid state in which surface will be None.
        //If we do but surface creation fails, then we're in a bad state and surface should be Some(err)
        let maybe_surface: Option<vk::SurfaceKHR> = unsafe {
            display_window_handles
                .map(|(display_handle, window_handle)| {
                    ash_window::create_surface(
                        &entry,
                        &instance,
                        display_handle,
                        window_handle,
                        None,
                    )
                })
                .transpose()?
        };

        let device_extension_names_raw: Vec<*const c_char> =
            maybe_surface.map_or_else(Vec::new, |_| vec![ash::khr::swapchain::NAME.as_ptr()]);

        //If we have a surface, filter the supported physical devices to only those that support it.
        //If we don't, then we're in a headless state and we can (probably) use all physical devices.
        let supported_physical_devices: Vec<vk::PhysicalDevice> = unsafe {
            if let Some(surface) = maybe_surface {
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
            } else {
                physical_devices
            }
        };

        //Find the first physical device that is a discrete GPU
        let selected_device = unsafe {
            supported_physical_devices
                .into_iter()
                .find(|&physical_device| {
                    instance
                        .get_physical_device_properties(physical_device)
                        .device_type
                        == vk::PhysicalDeviceType::DISCRETE_GPU
                })
                .expect("failed to find a suitable GPU")
        };

        //Device creation could fail without a panic if the automatically selected physical device
        // is unsuitable, but a suitable device can be enumerated.
        let device = create_device(
            &instance,
            selected_device,
            device_extension_names_raw,
            enable_validation,
            &entry,
        )?;

        let swapchain = display_window_handles.map(|(display_handle, window_handle)| {
            let surface = unsafe {
                ash_window::create_surface(&entry, &instance, display_handle, window_handle, None)
                    .expect("failed to create surface")
            };

            let surface_format = unsafe {
                surface_loader
                    .get_physical_device_surface_formats(selected_device, surface)
                    .unwrap()[0]
            };
            let surface_capabilities = unsafe {
                surface_loader
                    .get_physical_device_surface_capabilities(selected_device, surface)
                    .unwrap()
            };
            swapchain::Swapchain::new(
                &instance,
                &device,
                surface,
                surface_format,
                surface_capabilities.current_extent,
                surface_capabilities.min_image_count,
            )
        });

        Ok(VulkanEngine {
            entry,
            instance,
            device,
            surface_loader,
            swapchain,
        })
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::{event_loop::EventLoop, window::Window};

    #[test]
    fn test_create_instance() {
        let entry = unsafe { Entry::load().expect("failed to load vulkan module") };

        create_instance(true, None, &entry).expect("Failed to create VordtEngine instance");
    }

    #[test]
    fn test_create_device() {
        let entry = unsafe { Entry::load().expect("failed to load vulkan module") };

        let instance =
            create_instance(true, None, &entry).expect("Failed to create VordtEngine instance");

        create_device(&instance, None, false, &entry).expect("Failed to create VordtEngine device");
    }
}

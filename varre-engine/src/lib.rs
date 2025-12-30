mod vulkan_swapchain;
mod physical_device_utils;
mod command_buffers;

use ash::{
    ext::{debug_utils, shader_object},
    khr::{surface, swapchain},
    vk, Device, Entry, Instance,
};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::borrow::Cow;
use std::{error::Error, ffi, os::raw::c_char};

use physical_device_utils::*;

pub const NUM_FRAMES_IN_FLIGHT: usize = 3;

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
) -> Result<Instance, Box<dyn Error>> {
    //todo: app name, version, etc.4
    let application_info = vk::ApplicationInfo::default()
        .application_name(c"varre-engine")
        .application_version(0)
        .engine_name(c"varre-engine")
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


//If display_window_handles is None, then we're in a headless state.
//The engine should always be able to render to something, so it should always have a device.
fn create_device(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    queue_family_indices: QueueFamilyIndices,
    headless: bool,
) -> Result<Device, Box<dyn Error>> {
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

    let device_extension_names_raw: Vec<*const c_char> = [shader_object::NAME.as_ptr()]
        .into_iter()
        .chain((!headless).then_some(swapchain::NAME.as_ptr()))
        .collect();

    let mut shader_object_features = vk::PhysicalDeviceShaderObjectFeaturesEXT::default();

    let device_create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&device_extension_names_raw)
        .push_next(&mut shader_object_features);

    unsafe {
        instance
            .create_device(physical_device, &device_create_info, None)
            .map_err(|e| e.into())
    }
}

pub struct VulkanEngine {
    entry: Entry,
    instance: Instance,
    physical_device: vk::PhysicalDevice,
    device: Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    one_time_command_buffer: vk::CommandBuffer,
    surface_loader: surface::Instance,
    shader_object_loader: shader_object::Device,
    swapchain: Option<vulkan_swapchain::Swapchain>,
    debug_utils: Option<(debug_utils::Instance, vk::DebugUtilsMessengerEXT)>,
}

impl VulkanEngine {
    pub fn new(
        enable_validation: bool,
        display_handle: Option<RawDisplayHandle>,
    ) -> Result<Self, Box<dyn Error>> {
        //Load entry point
        //'linked' here means compile-time static linkage against vulkan development libraries.
        let entry = Entry::linked();

        //TODO: should we panic if this fails? Depends on whether there is anything the engine or the
        //      user can do to fix instance creation (update paths to missing layers, etc.)
        let instance = create_instance(enable_validation, display_handle, &entry)?;

        //We'll load surface functions regardless of whether we have a surface or not.
        let surface_loader = surface::Instance::new(&entry, &instance);

        //Get the list of available physical devices
        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .expect("failed to enumerate physical devices")
        };

        //Find the first physical device that is a discrete GPU
        let physical_device = select_physical_device(physical_devices, &instance);

        let queue_family_properties = unsafe {instance.get_physical_device_queue_family_properties(physical_device)};

        let queue_family_indices = QueueFamilyIndices::new(
            &queue_family_properties
        );

        let debug_utils = enable_validation.then(|| {
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

            let debug_utils_loader = debug_utils::Instance::new(&entry, &instance);
            let debug_callback = unsafe {
                debug_utils_loader
                    .create_debug_utils_messenger(&debug_info, None)
                    .unwrap()
            };

            (debug_utils_loader, debug_callback)
        });

        //Device creation could fail without a panic if the automatically selected physical device
        // is unsuitable, but a suitable device can be enumerated.
        let device = create_device(
            &instance,
            physical_device,
            queue_family_indices,
            display_handle.is_none(),
        )?;

        let queue = unsafe {Device::get_device_queue(&device, queue_family_indices.graphics_general.unwrap(), 0)};

        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_indices.graphics_general.unwrap());

        let command_pool = unsafe {device.create_command_pool(&command_pool_create_info, None).expect("failed to create command pool")};

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_buffer_count(1)
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let one_time_command_buffer = unsafe { device.allocate_command_buffers(&command_buffer_allocate_info)?[0] };

        let shader_object_loader = shader_object::Device::new(&instance, &device);

        Ok(VulkanEngine {
            entry,
            instance,
            physical_device,
            device,
            queue,
            command_pool,
            one_time_command_buffer,
            surface_loader,
            shader_object_loader,
            swapchain: None,
            debug_utils,
        })
    }

    pub fn add_window(&mut self, display_handle: RawDisplayHandle, window_handle: RawWindowHandle, window_width: u32, window_height: u32) {

            let surface = unsafe {
                ash_window::create_surface(&self.entry, &self.instance, display_handle, window_handle, None)
                    .expect("failed to create surface")
            };

            let surface_format = unsafe {
                self.surface_loader
                    .get_physical_device_surface_formats(self.physical_device, surface)
                    .unwrap()[0]
            };

            let surface_capabilities = unsafe {
                self.surface_loader
                    .get_physical_device_surface_capabilities(self.physical_device, surface)
                    .unwrap()
            };

            let present_modes = unsafe {
                self.surface_loader.get_physical_device_surface_present_modes(self.physical_device, surface)
                    .unwrap()
            };

            let swapchain = vulkan_swapchain::Swapchain::new(
                &self.instance,
                &self.device,
                surface,
                vk::Extent2D { width: window_width, height: window_height },
                surface_capabilities,
                present_modes,
                surface_format
            );

            self.swapchain = Some(swapchain);
    }

    pub fn record_and_submit_one_time_command_buffer<F: FnOnce(&Device, vk::CommandBuffer)>(&self, f: F) {
        unsafe {
            self.record_command_buffer(self.one_time_command_buffer, f);

            self.device.queue_wait_idle(self.queue).expect("failed to wait for queue idle");

            self.submit_command_buffer(self.one_time_command_buffer);

            self.device.queue_wait_idle(self.queue).expect("failed to wait for queue idle");
        }
    }

    pub fn record_command_buffer<F: FnOnce(&Device, vk::CommandBuffer)>(&self, command_buffer: vk::CommandBuffer, f: F) {
        unsafe {
            self.device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::RELEASE_RESOURCES)
                .expect("failed to reset command buffer");

            let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            self.device.begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .expect("failed to begin command buffer");

            f(&self.device, command_buffer);

            self.device.end_command_buffer(command_buffer)
                .expect("failed to end command buffer");
        }
    }

    pub fn submit_command_buffer(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            let command_buffers = vec![command_buffer];

            let submit_info = vk::SubmitInfo::default()
                .command_buffers(&command_buffers);

            self.device.queue_submit(self.queue, &[submit_info], vk::Fence::null())
                .expect("failed to submit command buffer");
        }
    }

    fn make_shader_objects(&self) {
        // @TODO - For now we will use hardcoded paths and stage info, but we should be able to read
        //         these from SPIR-V, and automatically make objects for every shader asset present.
        
        let vertex_shader_bytes: &[u8] = varre_assets::shaders::SHADER_TRIANGLE_VERTEX;
        let shader_create_info = vk::ShaderCreateInfoEXT::default()
            .stage(vk::ShaderStageFlags::VERTEX)
    }
}

impl Drop for VulkanEngine {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            if let Some((ref debug_loader, debug_callback)) = self.debug_utils {
                debug_loader.destroy_debug_utils_messenger(debug_callback, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_instance() {
        let entry = unsafe { Entry::load().expect("failed to load vulkan module") };

        create_instance(true, None, &entry).expect("Failed to create VarreEngine instance");
    }

    #[test]
    fn test_create_device() {
        let entry = unsafe { Entry::load().expect("failed to load vulkan module") };

        let instance =
            create_instance(true, None, &entry).expect("Failed to create VarreEngine instance");

        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .expect("failed to enumerate physical devices")
        };

        let physical_device = select_physical_device(physical_devices, &instance);

        let queue_family_properties = unsafe {instance.get_physical_device_queue_family_properties(physical_device)};

        let queue_family_indices = QueueFamilyIndices::new(
            &queue_family_properties
        );

        create_device(&instance, physical_device, queue_family_indices, false).expect("Failed to create VarreEngine device");
    }

    #[test]
    fn test_create_engine() {
        VulkanEngine::new(true, None).expect("Failed to create VarreEngine");
    }

    #[test]
    fn test_submit_command_buffer() {
        let engine = VulkanEngine::new(true, None).expect("Failed to create VarreEngine");
        engine.record_and_submit_one_time_command_buffer(|_, _| {});
    }
}

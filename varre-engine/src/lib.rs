mod command_buffers;
mod geometry;
mod memory_utils;
mod mesh_utils;
mod physical_device_utils;
mod render_context;
mod shader_utils;
mod vulkan_window;
mod extensions;

use crate::mesh_utils::VulkanMesh;
use crate::render_context::mesh_simple::MeshSimpleRenderContext;
use ash::vk::SurfaceKHR;
use ash::{
    Device, Entry, Instance,
    ext::{debug_utils, shader_object},
    khr::{surface, swapchain},
    vk,
};
use physical_device_utils::*;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use render_context::RenderContext;
pub use render_context::RenderContextType;
use render_context::triangle::TriangleRenderContext;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::CStr;
use std::{error::Error, ffi, os::raw::c_char};
use varre_assets::{ModelID, ShaderID};
use crate::extensions::unified_image_layouts;

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

    let device_extension_names_raw: Vec<*const c_char> = [shader_object::NAME.as_ptr(), unified_image_layouts::NAME.as_ptr()]
        .into_iter()
        .chain((!headless).then_some(swapchain::NAME.as_ptr()))
        .collect();

    let mut shader_object_features =
        vk::PhysicalDeviceShaderObjectFeaturesEXT::default().shader_object(true);

    let mut unified_image_layouts_features =
        unified_image_layouts::PhysicalDeviceUnifiedImageLayoutsFeaturesKHR::default().unified_image_layouts(true);

    let mut vulkan11_features =
        vk::PhysicalDeviceVulkan11Features::default().shader_draw_parameters(true);

    let mut vulkan13_features = vk::PhysicalDeviceVulkan13Features::default()
        .synchronization2(true)
        .dynamic_rendering(true);

    let device_create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&device_extension_names_raw)
        .push_next(&mut shader_object_features)
        .push_next(&mut unified_image_layouts_features)
        .push_next(&mut vulkan11_features)
        .push_next(&mut vulkan13_features);

    unsafe {
        instance
            .create_device(physical_device, &device_create_info, None)
            .map_err(|e| e.into())
    }
}

// @NOTE I could consider adding bindings to extension functions in the impl for DeviceContext and
//       making its handles private. This seems autogen-able, but potentially fraught with errors.
pub struct DeviceContext {
    pub entry: Entry,
    pub instance: Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: Device,
    pub graphics_queue: vk::Queue,
    pub surface_loader: surface::Instance,
    pub swapchain_loader: swapchain::Device,
    pub shader_object_loader: Option<shader_object::Device>,
}

pub struct VulkanEngine {
    device_context: DeviceContext,

    command_pool: vk::CommandPool,
    draw_command_buffers: [vk::CommandBuffer; 3],
    one_time_command_buffer: vk::CommandBuffer,

    surface: Option<SurfaceKHR>,
    window: Option<vulkan_window::VulkanWindow>,

    frame_index: usize,
    debug_utils: Option<(debug_utils::Instance, vk::DebugUtilsMessengerEXT)>,

    render_context: Option<Box<dyn RenderContext>>,
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

        //Get the list of available physical devices
        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .expect("failed to enumerate physical devices")
        };

        //Find the first physical device that is a discrete GPU
        let physical_device = select_physical_device(physical_devices, &instance);

        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let queue_family_indices = QueueFamilyIndices::new(&queue_family_properties);

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

        // Create DeviceContext
        let surface_loader = surface::Instance::new(&entry, &instance);
        let swapchain_loader = swapchain::Device::new(&instance, &device);
        let shader_object_loader = Some(shader_object::Device::new(&instance, &device));

        let graphics_queue = unsafe {
            Device::get_device_queue(
                &device,
                queue_family_indices.graphics_general.unwrap(),
                0,
            )
        };

        let device_context = DeviceContext {
            entry,
            instance,
            physical_device,
            device,
            graphics_queue,
            surface_loader,
            swapchain_loader,
            shader_object_loader,
        };


        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_indices.graphics_general.unwrap());

        let command_pool = unsafe {
            device_context
                .device
                .create_command_pool(&command_pool_create_info, None)
                .expect("failed to create command pool")
        };

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_buffer_count(4)
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffers = unsafe {
            device_context
                .device
                .allocate_command_buffers(&command_buffer_allocate_info)?
        };

        let one_time_command_buffer = command_buffers[0];
        let draw_command_buffers = command_buffers[1..][..3].try_into()?;

        Ok(VulkanEngine {
            device_context,

            command_pool,
            draw_command_buffers,
            one_time_command_buffer,
            surface: None,
            window: None,
            frame_index: 0,
            debug_utils,
            render_context: None,
        })
    }

    pub fn set_render_context(&mut self, context_type: RenderContextType) {
        match context_type {
            RenderContextType::Triangle => {
                self.render_context =
                    Some(Box::new(TriangleRenderContext::new(&self.device_context)));
            }
            RenderContextType::MeshSimple => {
                self.render_context =
                    Some(Box::new(MeshSimpleRenderContext::new(&self.device_context)));
            }
        }

        self.setup_render_context();
    }

    pub fn add_window(
        &mut self,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
        window_width: u32,
        window_height: u32,
    ) {
        self.window = Some(vulkan_window::VulkanWindow::new(
            &self.device_context,
            (display_handle, window_handle),
            vk::Extent2D {
                width: window_width,
                height: window_height,
            },
        ));

        unsafe {
            let cmd = self.one_time_command_buffer;
            self.device_context
                .device
                .reset_command_buffer(
                    cmd,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .expect("failed to reset command buffer");

            let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device_context.device.begin_command_buffer(cmd, &command_buffer_begin_info).unwrap();
            self.window.as_ref().unwrap().initialize_images(&self.device_context, self.one_time_command_buffer.clone());
            self.device_context.device.end_command_buffer(cmd).unwrap();

            let command_buffers = vec![cmd];

            let submit_info = vk::SubmitInfo::default()
                .command_buffers(&command_buffers);

            self.device_context
                .device
                .queue_submit(self.device_context.graphics_queue, &[submit_info], vk::Fence::null())
                .expect("failed to submit command buffer");

            self.device_context.device.device_wait_idle().unwrap();
        }
    }

    fn get_next_command_buffer(&mut self) -> vk::CommandBuffer {
        let idx = self.frame_index.clone();
        self.frame_index = (idx + 1) % NUM_FRAMES_IN_FLIGHT;
        self.draw_command_buffers[idx].clone()
    }

    pub fn on_window_resized(&mut self, window_width: u32, window_height: u32) {
        unsafe {
            self.window.as_mut().unwrap().on_window_resized(
                &self.device_context,
                vk::Extent2D {
                    width: window_width,
                    height: window_height,
                },
            )
        }
    }

    pub fn draw(&mut self) {
        let cmd = self.get_next_command_buffer();
        self.window.as_mut().unwrap().render_frame(&self.device_context, cmd, self.render_context.as_mut().unwrap())
    }

    pub fn setup_render_context(&mut self) {
        unsafe {
            let cmd = self.one_time_command_buffer;
            self.device_context
                .device
                .reset_command_buffer(
                    cmd,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .expect("failed to reset command buffer");

            let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device_context.device.begin_command_buffer(cmd, &command_buffer_begin_info).unwrap();
            self.render_context.as_mut().unwrap().record_setup(&self.device_context, self.one_time_command_buffer.clone());
            self.device_context.device.end_command_buffer(cmd).unwrap();

            let command_buffers = vec![cmd];

            let submit_info = vk::SubmitInfo::default()
                .command_buffers(&command_buffers);

            self.device_context
                .device
                .queue_submit(self.device_context.graphics_queue, &[submit_info], vk::Fence::null())
                .expect("failed to submit command buffer");

            self.device_context.device.device_wait_idle().unwrap();
        }
    }
}

impl Drop for VulkanEngine {
    fn drop(&mut self) {
        unsafe {
            self.device_context.device.device_wait_idle().unwrap();

            // Destroy swapchain and related resources
            if let Some(swapchain) = self.window.take() {
                for view in swapchain.swapchain_image_views.iter() {
                    self.device_context.device.destroy_image_view(*view, None);
                }
                self.device_context
                    .swapchain_loader
                    .destroy_swapchain(swapchain.vk_swapchain, None);
            }

            // Destroy command pool (this also frees command buffers)
            self.device_context
                .device
                .destroy_command_pool(self.command_pool, None);

            // Destroy device
            self.device_context.device.destroy_device(None);

            // Destroy surface
            if let Some(surface) = self.surface {
                self.device_context
                    .surface_loader
                    .destroy_surface(surface, None);
            }

            // Destroy debug utils
            if let Some((ref debug_loader, debug_callback)) = self.debug_utils {
                debug_loader.destroy_debug_utils_messenger(debug_callback, None);
            }

            // Destroy instance
            self.device_context.instance.destroy_instance(None);
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

        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let queue_family_indices = QueueFamilyIndices::new(&queue_family_properties);

        create_device(&instance, physical_device, queue_family_indices, false)
            .expect("Failed to create VarreEngine device");
    }

    #[test]
    fn test_create_engine() {
        VulkanEngine::new(true, None).expect("Failed to create VarreEngine");
    }

}

use std::{error::Error, os::raw::c_char};

use ash::{vk, Entry};


fn create_instance(enable_validation: bool, loader: &Entry) -> Result<ash::Instance, Box<dyn Error>> {

    //todo: app name, version, etc.
    let application_info = vk::ApplicationInfo::default()
      .application_name(c"vordt-engine")
      .application_version(0)
      .engine_name(c"vordt-engine")
      .engine_version(0)
      .api_version(vk::make_api_version(0, 1, 3, 0));

    let layer_names = if enable_validation { vec![c"VK_LAYER_KHRONOS_validation"] } else { vec![] };
    let layer_names_raw:Vec<*const c_char> = layer_names.iter().map(|name| name.as_ptr()).collect();
    //let mut extension_names = vec![];

    let instance_create_flags = vk::InstanceCreateFlags::default();

    let instance_create_info = vk::InstanceCreateInfo::default()
      .application_info(&application_info)
      .enabled_layer_names(&layer_names_raw)
      .flags(instance_create_flags);

  
  unsafe {loader.create_instance(&instance_create_info, None).map_err(|e| e.into())}
  
}

pub struct VulkanEngine {}

impl VulkanEngine {
  pub fn new(enable_validation: bool) -> Result<Self, Box<dyn Error>> {
    
    //Load entry point
    //'linked' here means compile-time static linkage against vulkan development libraries.
    let entry = unsafe { Entry::load()? };

    let instance = { create_instance(enable_validation, &entry)? };
    
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

    create_instance(true, &entry).expect("Failed to create VordtEngine instance");
  }

  #[test]
  fn it_works() {
    let result = add(2, 2);
    assert_eq!(result, 4);
  }
}

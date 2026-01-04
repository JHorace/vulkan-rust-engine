use ash::vk;

fn find_memory_type_index(
    memory_requirements: vk::MemoryRequirements,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
   memory_properties.memory_types[..memory_properties.memory_type_count as _]
       .iter()
       .enumerate()
       .find(|(index, memory_type)| {
           (1 << index) as u32 & memory_requirements.memory_type_bits != 0
            && memory_type.property_flags.contains(flags)
       })
       .map(|(index, _memory_type)| index as _)
}

pub fn create_buffer(device_context: &crate::DeviceContext, size: vk::DeviceSize, usage: vk::BufferUsageFlags, memory_properties: vk::MemoryPropertyFlags) -> (vk::Buffer, vk::DeviceMemory) {
    let buffer_create_info = vk::BufferCreateInfo::default()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = unsafe { device_context.device.create_buffer(&buffer_create_info, None).expect("failed to create buffer!") };

    let memory_reqs = unsafe { device_context.device.get_buffer_memory_requirements(buffer) };
    let physical_device_memory_properties = unsafe { device_context.instance.get_physical_device_memory_properties(device_context.physical_device) };
    let memory_type_index = find_memory_type_index(memory_reqs, physical_device_memory_properties, memory_properties).expect("failed to find suitable memory type");

    let memory_allocate_info = vk::MemoryAllocateInfo::default()
        .allocation_size(memory_reqs.size)
        .memory_type_index(memory_type_index);

    let device_memory = unsafe { device_context.device.allocate_memory(&memory_allocate_info, None).expect("failed to allocate memory!") };

    unsafe { device_context.device.bind_buffer_memory(buffer, device_memory, 0).expect("failed to bind buffer memory!") };

    (buffer, device_memory)
}
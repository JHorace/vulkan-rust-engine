use ash::util::Align;
use ash::vk;
use glam::Vec3;
use crate::memory_utils::create_buffer;

pub struct VulkanMesh {
    pub vertex_staging_buffer: vk::Buffer,
    vertex_staging_buffer_memory: vk::DeviceMemory,
    pub vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    pub vertex_buffer_size: vk::DeviceSize,
    pub index_staging_buffer: vk::Buffer,
    index_staging_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    pub index_buffer_size: vk::DeviceSize,
    pub index_count: u32,
}

impl VulkanMesh {

    pub fn from_model(device_context: &crate::DeviceContext, model: &varre_assets::Model) -> Self {
        unsafe {

            let vertex_buffer_size = (model.verts.len() * std::mem::size_of::<Vec3>()) as vk::DeviceSize;
            let index_buffer_size = (model.indices.len() * std::mem::size_of::<u32>()) as vk::DeviceSize;

            let (vertex_staging_buffer, vertex_staging_buffer_memory) = create_buffer(device_context, vertex_buffer_size, vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::VERTEX_BUFFER, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);
            let (index_staging_buffer, index_staging_buffer_memory) = create_buffer(device_context, index_buffer_size, vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::INDEX_BUFFER, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);
            let (vertex_buffer, vertex_buffer_memory) = create_buffer(device_context, vertex_buffer_size, vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER, vk::MemoryPropertyFlags::DEVICE_LOCAL);
            let (index_buffer, index_buffer_memory) = create_buffer(device_context, index_buffer_size, vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER, vk::MemoryPropertyFlags::DEVICE_LOCAL);

            let vertex_ptr = device_context.device.map_memory(vertex_staging_buffer_memory, 0, vertex_buffer_size, vk::MemoryMapFlags::empty()).unwrap();

            let mut vertex_slice = Align::new(vertex_ptr,
            align_of::<Vec3>() as u64,
            vertex_buffer_size as u64);

            vertex_slice.copy_from_slice(&model.verts);

            device_context.device.unmap_memory(vertex_staging_buffer_memory);

            let index_ptr = device_context.device.map_memory(index_staging_buffer_memory, 0, index_buffer_size, vk::MemoryMapFlags::empty()).unwrap();

            let mut index_slice = Align::new(index_ptr,
            align_of::<u32>() as u64,
            index_buffer_size as u64);

            index_slice.copy_from_slice(&model.indices);

            device_context.device.unmap_memory(index_staging_buffer_memory);

            Self {
                vertex_staging_buffer,
                vertex_staging_buffer_memory,
                vertex_buffer,
                vertex_buffer_memory,
                vertex_buffer_size,
                index_staging_buffer,
                index_staging_buffer_memory,
                index_buffer,
                index_buffer_memory,
                index_buffer_size,
                index_count: model.indices.len() as u32
            }
        }
    }
}

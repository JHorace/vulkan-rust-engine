use std::collections::HashMap;
use ash::vk;
use glam::Vec3;
use varre_assets::ModelID;
use crate::memory_utils::create_buffer;

pub struct VulkanMesh {
    vertex_staging_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    index_staging_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
}

impl VulkanMesh {

    pub fn from_model(device_context: &crate::DeviceContext, model: &varre_assets::Model) -> Self {
        let vertex_buffer_size = (model.verts.len() * std::mem::size_of::<Vec3>()) as vk::DeviceSize;
        let index_buffer_size = (model.indices.len() * std::mem::size_of::<u32>()) as vk::DeviceSize;

        let (vertex_staging_buffer, vertex_buffer_memory) = create_buffer(device_context, vertex_buffer_size, vk::BufferUsageFlags::VERTEX_BUFFER, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);
        let (index_staging_buffer, index_buffer_memory) = create_buffer(device_context, index_buffer_size, vk::BufferUsageFlags::INDEX_BUFFER, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);

        Self {
            vertex_staging_buffer,
            vertex_buffer_memory,
            index_staging_buffer,
            index_buffer_memory,
        }
    }
}

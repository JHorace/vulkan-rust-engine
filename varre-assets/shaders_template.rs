// Template for generated shaders.rs
// This file is read by build.rs and prepended to the generated shader constants

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex,
    TessellationControl,
    TessellationEvaluation,
    Geometry,
    Fragment,
    Compute,
    Task,
    Mesh,
    Raygen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderID {
    // ShaderID variants will be generated here by build.rs
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VkDescriptorSetLayoutBinding {
    pub set: u32,
    pub binding: u32,
    pub descriptor_type: u32,
    pub descriptor_count: u32,
    pub stage_flags: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Shader {
    pub id: ShaderID,
    pub spv: &'static [u8],
    pub stage: ShaderStage,
    pub entry_point: &'static str,
    pub descriptor_set_layout_bindings: &'static [VkDescriptorSetLayoutBinding],
}

pub mod shaders {
    use super::{Shader, ShaderStage};
    // Shader constants will be generated here by build.rs
}

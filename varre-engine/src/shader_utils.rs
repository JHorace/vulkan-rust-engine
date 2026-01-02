use std::collections::HashMap;
use ash::vk;
use ash::ext::shader_object;
use std::ffi::CStr;
use varre_assets::{Shader, ShaderID};

// Helper trait for converting ShaderStage to Vulkan flags
pub trait ToVkShaderStage {
    fn to_vk(&self) -> vk::ShaderStageFlags;
}

impl ToVkShaderStage for varre_assets::ShaderStage {
    fn to_vk(&self) -> vk::ShaderStageFlags {
        match self {
            varre_assets::ShaderStage::Vertex => vk::ShaderStageFlags::VERTEX,
            varre_assets::ShaderStage::Fragment => vk::ShaderStageFlags::FRAGMENT,
            varre_assets::ShaderStage::Compute => vk::ShaderStageFlags::COMPUTE,
            varre_assets::ShaderStage::Geometry => vk::ShaderStageFlags::GEOMETRY,
            varre_assets::ShaderStage::TessellationControl => vk::ShaderStageFlags::TESSELLATION_CONTROL,
            varre_assets::ShaderStage::TessellationEvaluation => vk::ShaderStageFlags::TESSELLATION_EVALUATION,
            varre_assets::ShaderStage::Task => vk::ShaderStageFlags::TASK_EXT,
            varre_assets::ShaderStage::Mesh => vk::ShaderStageFlags::MESH_EXT,
            varre_assets::ShaderStage::Raygen => vk::ShaderStageFlags::RAYGEN_KHR,
        }
    }
}

pub struct VulkanShader {
    pub stage: vk::ShaderStageFlags,
    pub shader: vk::ShaderEXT,
}

impl VulkanShader {
    /// Create a VulkanShader from a varre_assets::Shader
    pub fn from_shader(
        shader_object_loader: &shader_object::Device,
        shader: &varre_assets::Shader,
    ) -> Self {
        let stage = shader.stage.to_vk();
        let shader_ext = create_shader_object(shader_object_loader, shader);

        Self {
            stage,
            shader: shader_ext,
        }
    }
}

pub struct ShaderManager {
    shaders: HashMap<ShaderID, vk::ShaderEXT>,
    shader_object_loader: shader_object::Device,
}

impl ShaderManager {

    pub fn new(shader_object_loader: shader_object::Device) -> Self {
        ShaderManager { shaders: HashMap::new(), shader_object_loader }
    }

    /// Load all shaders from varre-assets into the shader manager
    pub fn load_shaders(&mut self) {
        for shader_id in ShaderID::all() {
            let shader = shader_id.shader();
            self.add_shader(shader);
        }
    }

    pub fn add_shader(&mut self, shader: &varre_assets::Shader)
    {
        let vulkan_shader = VulkanShader::from_shader(&self.shader_object_loader,
                                                      shader);
        self.shaders.insert(shader.id, vulkan_shader.shader);
    }

    /// Get a shader by its ID
    pub fn get_shader(&self, id: ShaderID) -> Option<&vk::ShaderEXT> {
        self.shaders.get(&id)
    }

    /// Get a mutable reference to a shader by its ID
    pub fn get_shader_mut(&mut self, id: ShaderID) -> Option<&mut vk::ShaderEXT> {
        self.shaders.get_mut(&id)
    }

    /// Get a reference to all shaders
    pub fn shaders(&self) -> &HashMap<ShaderID, vk::ShaderEXT> {
        &self.shaders
    }

    /// Get a mutable reference to all shaders
    pub fn shaders_mut(&mut self) -> &mut HashMap<ShaderID, vk::ShaderEXT> {
        &mut self.shaders
    }
}

impl Drop for ShaderManager {
    fn drop(&mut self) {
        unsafe {
            // Destroy all shader objects
            for (_id, shader) in self.shaders.drain() {
                self.shader_object_loader.destroy_shader(shader, None);
            }
        }
    }
}

/// Determine valid next stages for a given shader stage in the graphics pipeline
fn get_next_stages(stage: vk::ShaderStageFlags) -> vk::ShaderStageFlags {
    match stage {
        vk::ShaderStageFlags::VERTEX => {
            // After vertex: tessellation control, geometry, or fragment
            vk::ShaderStageFlags::TESSELLATION_CONTROL
                | vk::ShaderStageFlags::GEOMETRY
                | vk::ShaderStageFlags::FRAGMENT
        }
        vk::ShaderStageFlags::TESSELLATION_CONTROL => {
            // After tessellation control: tessellation evaluation (required)
            vk::ShaderStageFlags::TESSELLATION_EVALUATION
        }
        vk::ShaderStageFlags::TESSELLATION_EVALUATION => {
            // After tessellation evaluation: geometry or fragment
            vk::ShaderStageFlags::GEOMETRY | vk::ShaderStageFlags::FRAGMENT
        }
        vk::ShaderStageFlags::GEOMETRY => {
            // After geometry: fragment
            vk::ShaderStageFlags::FRAGMENT
        }
        vk::ShaderStageFlags::TASK_EXT => {
            // After task: mesh (required for mesh shading pipeline)
            vk::ShaderStageFlags::MESH_EXT
        }
        vk::ShaderStageFlags::MESH_EXT => {
            // After mesh: fragment
            vk::ShaderStageFlags::FRAGMENT
        }
        vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::COMPUTE | vk::ShaderStageFlags::RAYGEN_KHR => {
            // Terminal stages: no next stage
            vk::ShaderStageFlags::empty()
        }
        _ => {
            // Unknown or unsupported stage
            vk::ShaderStageFlags::empty()
        }
    }
}

pub fn create_shader_object(
    shader_object_loader: &shader_object::Device,
    shader: &varre_assets::Shader,
) -> vk::ShaderEXT {
    let stage = shader.stage.to_vk();
    let next_stage = get_next_stages(stage);

    // Convert entry point to CString (owned, null-terminated)
    let entry_point_string = format!("{}\0", shader.entry_point);
    let entry_point = CStr::from_bytes_with_nul(entry_point_string.as_bytes())
        .expect("Invalid entry point");

    unsafe {
        let shader_create_info = [vk::ShaderCreateInfoEXT::default()
            .stage(stage)
            .code_type(vk::ShaderCodeTypeEXT::SPIRV)
            .code(shader.spv)
            .name(entry_point)
            .next_stage(next_stage)];

        shader_object_loader
            .create_shaders(&shader_create_info, None)
            .expect("failed to create shaders")[0]
    }
}
use ash::vk;
use ash::ext::shader_object;
use std::ffi::CStr;

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
        device_context: &crate::DeviceContext,
        shader: &varre_assets::Shader,
    ) -> Self {
        let shader_object_loader = device_context.shader_object_loader.as_ref()
            .expect("shader_object_loader not available");
        let stage = shader.stage.to_vk();
        let shader_ext = create_shader_object(shader_object_loader, shader);

        Self {
            stage,
            shader: shader_ext,
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
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use regex::Regex;
use russimp::scene::{PostProcess, Scene};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();


    process_shaders(&out_dir);
    process_models(&out_dir);
}

fn process_shaders(out_dir: &str) {
    let shader_dir = Path::new("shaders");
    let out_shader_dir = Path::new(out_dir).join("shaders");

    // Create output directory
    fs::create_dir_all(&out_shader_dir).expect("Failed to create output shader directory");

    // Tell cargo to rerun if shaders change or if the compiler path changes
    println!("cargo:rerun-if-changed=shaders");
    println!("cargo:rerun-if-env-changed=SLANGC_PATH");

    if !shader_dir.exists() {
        return;
    }

    // Determine which slangc to use:
    // 1. Check SLANGC_PATH environment variable
    // 2. Fallback to "slangc" (looking in system PATH)
    let slangc_command = env::var("SLANGC_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("slangc"));

    // Find all .slang files
    let slang_files: Vec<_> = fs::read_dir(shader_dir)
        .expect("Failed to read shaders directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "slang" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    // Regex to match [shader("stage")] followed by a function definition
    // Matches: [shader("vertex")] ... functionName(
    let shader_attr_regex = Regex::new(r#"\[shader\("([^"]+)"\)][^(]*\s(\w+)\s*\("#)
        .expect("Invalid regex");

    for shader_path in slang_files {
        let file_name = shader_path.file_name().unwrap().to_str().unwrap();
        let base_name = shader_path.file_stem().unwrap().to_str().unwrap();

        // Read the shader file to detect entry points
        let shader_content = fs::read_to_string(&shader_path)
            .expect(&format!("Failed to read shader file: {}", file_name));

        // Extract all shader entry points with their stages
        let mut entry_points = Vec::new();
        for cap in shader_attr_regex.captures_iter(&shader_content) {
            let stage_name = cap.get(1).unwrap().as_str();
            let function_name = cap.get(2).unwrap().as_str();
            entry_points.push((function_name.to_string(), stage_name.to_string()));
        }

        if entry_points.is_empty() {
            println!("cargo:warning=No shader entry points found in {}", file_name);
            continue;
        }

        println!("cargo:info=Found {} entry point(s) in {}", entry_points.len(), file_name);

        for (entry_point, stage_name) in entry_points {
            // Replace hyphens with underscores for the output filename
            let safe_base_name = base_name.replace('-', "_");
            let output_path = out_shader_dir.join(format!("{}.{}.spv", safe_base_name, stage_name));

            println!("cargo:info=Compiling shader: {} (entry: {}, stage: {})",
                     file_name, entry_point, stage_name);

            let status = Command::new(&slangc_command)
                .arg(&shader_path)
                .arg("-target")
                .arg("spirv")
                .arg("-entry")
                .arg(&entry_point)
                .arg("-o")
                .arg(&output_path)
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!("cargo:info=Successfully compiled {} ({})", file_name, entry_point);
                }
                Ok(s) => {
                    panic!("Failed to compile shader {} (entry {}): exit code {:?}",
                           file_name, entry_point, s.code());
                }
                Err(e) => {
                    let error_msg = if e.kind() == std::io::ErrorKind::NotFound {
                        format!(
                            "Executable '{:?}' not found. \n\
                            Please install slangc and ensure it's in your PATH, \n\
                            or set the SLANGC_PATH environment variable to the full path of the binary.",
                            slangc_command
                        )
                    } else {
                        format!("Failed to run slangc: {}", e)
                    };
                    panic!("Failed to run slangc for {} (entry {}): {}", file_name, entry_point, error_msg);
                }
            }
        }

        generate_shader_module(out_dir, &out_shader_dir);
    }
}

fn generate_shader_module(out_dir: &str, out_shader_dir: &Path) {
    // Read the template file
    let template_path = Path::new("shaders_template.rs");
    let template = fs::read_to_string(template_path)
        .expect("Failed to read shaders_template.rs");

    // Split the template to insert ShaderID enum variants
    let (before_shader_id, after_shader_id) = template.split_once("    // ShaderID variants will be generated here by build.rs")
        .expect("Template missing ShaderID placeholder comment");

    let (shader_id_suffix, after_shader_struct) = after_shader_id.split_once("    // Shader constants will be generated here by build.rs")
        .expect("Template missing Shader constants placeholder comment");

    let mut generated_code = String::from(before_shader_id);

    // Helper function to map stage names to ShaderStage enum variants
    fn stage_to_enum(stage: &str) -> &str {
        match stage {
            "vertex" => "ShaderStage::Vertex",
            "fragment" => "ShaderStage::Fragment",
            "compute" => "ShaderStage::Compute",
            "geometry" => "ShaderStage::Geometry",
            "tesscontrol" => "ShaderStage::TessellationControl",
            "tesseval" => "ShaderStage::TessellationEvaluation",
            _ => panic!("Unknown shader stage: {}", stage),
        }
    }

    // First pass: collect all shader names for the enum
    let mut shader_names = Vec::new();
    let entries: Vec<_> = fs::read_dir(out_shader_dir)
        .expect("Failed to read output shader directory")
        .filter_map(|e| e.ok())
        .collect();

    for entry in &entries {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "spv") {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            let parts: Vec<&str> = file_name.trim_end_matches(".spv").split('.').collect();
            if parts.len() < 2 {
                continue;
            }

            let var_name = file_name
                .replace(".spv", "")
                .replace('.', "_")
                .replace('-', "_")
                .to_uppercase();

            shader_names.push(var_name);
        }
    }

    // Generate ShaderID enum variants
    for name in &shader_names {
        generated_code.push_str(&format!("    {},\n", name));
    }
    generated_code.push_str(shader_id_suffix);

    // Second pass: generate shader constants with id field
    for entry in &entries {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "spv") {
            let file_name = path.file_name().unwrap().to_str().unwrap();

            // Parse filename: "name.stage.spv" -> extract stage and entry point name
            let parts: Vec<&str> = file_name.trim_end_matches(".spv").split('.').collect();
            if parts.len() < 2 {
                println!("cargo:warning=Skipping malformed shader filename: {}", file_name);
                continue;
            }

            let base_name = parts[0..parts.len()-1].join("_");
            let stage = parts[parts.len()-1];

            // Transform "name.stage.spv" into a valid Rust identifier "NAME_STAGE"
            let var_name = file_name
                .replace(".spv", "")
                .replace('.', "_")
                .replace('-', "_")
                .to_uppercase();

            // We use a path relative to OUT_DIR for the include_bytes! macro
            let rel_path = format!("shaders/{}", file_name);

            // Generate Shader struct constant
            generated_code.push_str(&format!(
                "    pub const {}: Shader = Shader {{\n",
                var_name
            ));
            generated_code.push_str(&format!(
                "        id: ShaderID::{},\n",
                var_name
            ));
            generated_code.push_str(&format!(
                "        spv: ::include_bytes_aligned::include_bytes_aligned!(4, concat!(env!(\"OUT_DIR\"), \"/{}\")),\n",
                rel_path
            ));
            generated_code.push_str(&format!(
                "        stage: {},\n",
                stage_to_enum(stage)
            ));
            generated_code.push_str(&format!(
                "        entry_point: \"{}\",\n",
                base_name
            ));
            generated_code.push_str("    };\n\n");
        }
    }

    // Close the shaders module
    generated_code.push_str("}\n\n");

    // Generate helper methods for ShaderID
    generated_code.push_str("impl ShaderID {\n");
    generated_code.push_str("    /// Get all shader IDs\n");
    generated_code.push_str("    pub const fn all() -> &'static [ShaderID] {\n");
    generated_code.push_str("        &[\n");
    for name in &shader_names {
        generated_code.push_str(&format!("            ShaderID::{},\n", name));
    }
    generated_code.push_str("        ]\n");
    generated_code.push_str("    }\n\n");

    generated_code.push_str("    /// Get the shader reference for this ID\n");
    generated_code.push_str("    pub const fn shader(&self) -> &'static Shader {\n");
    generated_code.push_str("        match self {\n");
    for name in &shader_names {
        generated_code.push_str(&format!("            ShaderID::{} => &shaders::{},\n", name, name));
    }
    generated_code.push_str("        }\n");
    generated_code.push_str("    }\n");
    generated_code.push_str("}\n");

    let dest_path = Path::new(out_dir).join("shaders.rs");
    fs::write(dest_path, generated_code).expect("Failed to write generated shaders.rs");
}
fn process_models(out_dir: &str) {
    let models_dir = Path::new("models");
    let out_models_dir = Path::new(out_dir).join("models");

    // Create output directory for binary model files
    fs::create_dir_all(&out_models_dir).expect("Failed to create output models directory");

    // Tell cargo to rerun if models change
    println!("cargo:rerun-if-changed=models");

    if !models_dir.exists() {
        return;
    }

    // Find all .obj files recursively
    fn find_obj_files(dir: &Path, obj_files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    find_obj_files(&path, obj_files);
                } else if path.extension().map_or(false, |ext| ext == "obj") {
                    obj_files.push(path);
                }
            }
        }
    }

    let mut obj_files = Vec::new();
    find_obj_files(models_dir, &mut obj_files);

    if obj_files.is_empty() {
        return;
    }

    // Read the template file
    let template_path = Path::new("models_template.rs");
    let template = fs::read_to_string(template_path)
        .expect("Failed to read models_template.rs");

    // Split the template to insert ModelID enum variants
    let (before_model_id, after_model_id) = template.split_once("    // ModelID variants will be generated here by build.rs")
        .expect("Template missing ModelID placeholder comment");

    let (model_id_suffix, after_model_struct) = after_model_id.split_once("    // Model constants will be generated here by build.rs")
        .expect("Template missing Model constants placeholder comment");

    let mut models_code = String::from(before_model_id);

    // First pass: collect all model names for the enum
    let mut model_names = Vec::new();
    for model_path in &obj_files {
        let base_name = model_path.file_stem().unwrap().to_str().unwrap();
        let var_name = base_name
            .replace('-', "_")
            .replace('.', "_")
            .to_uppercase();
        model_names.push(var_name);
    }

    // Generate ModelID enum variants
    for name in &model_names {
        models_code.push_str(&format!("    {},\n", name));
    }
    models_code.push_str(model_id_suffix);

    // Second pass: generate model constants with id field
    for model_path in obj_files {
        let file_name = model_path.file_name().unwrap().to_str().unwrap();
        let base_name = model_path.file_stem().unwrap().to_str().unwrap();

        println!("cargo:info=Loading model: {}", file_name);

        // Load the model using russimp
        let scene = Scene::from_file(
            model_path.to_str().unwrap(),
            vec![
                PostProcess::Triangulate,
                PostProcess::JoinIdenticalVertices,
                PostProcess::GenerateNormals,
            ],
        ).expect(&format!("Failed to load model: {}", file_name));

        // Extract vertices, indices, and UVs from the first mesh
        if scene.meshes.is_empty() {
            println!("cargo:warning=No meshes found in {}", file_name);
            continue;
        }

        let mesh = &scene.meshes[0];

        // Flatten indices
        let mut indices = Vec::new();
        for face in &mesh.faces {
            for index in &face.0 {
                indices.push(*index);
            }
        }

        // Flatten UVs (u, v for each vertex)
        let mut uvs = Vec::new();
        if !mesh.texture_coords.is_empty() && mesh.texture_coords[0].is_some() {
            if let Some(tex_coords) = &mesh.texture_coords[0] {
                for uv in tex_coords {
                    uvs.push(uv.x);
                    uvs.push(uv.y);
                }
            }
        }

        // Serialize to binary format:
        // [vert_count: u32][vertices: f32*3*n][index_count: u32][indices: u32*n][uv_count: u32][uvs: f32*n]
        let mut binary_data = Vec::new();

        // Write vertex count
        binary_data.extend_from_slice(&(mesh.vertices.len() as u32).to_le_bytes());

        // Write vertices
        for vertex in &mesh.vertices {
            binary_data.extend_from_slice(&vertex.x.to_le_bytes());
            binary_data.extend_from_slice(&vertex.y.to_le_bytes());
            binary_data.extend_from_slice(&vertex.z.to_le_bytes());
        }

        // Write index count
        binary_data.extend_from_slice(&(indices.len() as u32).to_le_bytes());

        // Write indices
        for index in &indices {
            binary_data.extend_from_slice(&index.to_le_bytes());
        }

        // Write UV count
        binary_data.extend_from_slice(&(uvs.len() as u32).to_le_bytes());

        // Write UVs
        for uv in &uvs {
            binary_data.extend_from_slice(&uv.to_le_bytes());
        }

        // Write binary file to OUT_DIR/models/
        let bin_filename = format!("{}.bin", base_name.replace('-', "_"));
        let bin_path = out_models_dir.join(&bin_filename);
        fs::write(&bin_path, binary_data).expect("Failed to write binary model file");

        // Generate a valid Rust identifier
        let var_name = base_name
            .replace('-', "_")
            .replace('.', "_")
            .to_uppercase();

        // Generate constant with include_bytes!
        models_code.push_str(&format!(
            "    pub const {}_DATA: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/models/{}\"));\n\n",
            var_name, bin_filename
        ));

        println!("cargo:info=Serialized model {} with {} vertices, {} indices ({} bytes)",
                 file_name, mesh.vertices.len(), indices.len(), bin_path.metadata().unwrap().len());
    }

    // Close the models module
    models_code.push_str("}\n\n");

    // Generate helper functions to load models by ID
    models_code.push_str("impl ModelID {\n");
    models_code.push_str("    /// Get the binary data for this model\n");
    models_code.push_str("    pub fn data(&self) -> &'static [u8] {\n");
    models_code.push_str("        match self {\n");
    for name in &model_names {
        models_code.push_str(&format!("            ModelID::{} => models::{}_DATA,\n", name, name));
    }
    models_code.push_str("        }\n");
    models_code.push_str("    }\n\n");
    models_code.push_str("    /// Load and decode this model\n");
    models_code.push_str("    pub fn load(&self) -> Model {\n");
    models_code.push_str("        Model::decode(*self, self.data())\n");
    models_code.push_str("    }\n");
    models_code.push_str("}\n");

    let dest_path = Path::new(out_dir).join("models.rs");
    fs::write(dest_path, models_code).expect("Failed to write generated models.rs");
}
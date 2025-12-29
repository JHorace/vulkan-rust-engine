use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let shader_dir = Path::new("shaders");
    let out_shader_dir = Path::new(&out_dir).join("shaders");

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

    for shader_path in slang_files {
        let file_name = shader_path.file_name().unwrap().to_str().unwrap();
        let output_path = out_shader_dir.join(format!("{}.spv", file_name));

        println!("cargo:warning=Compiling shader: {}", file_name);

        let status = Command::new(&slangc_command)
            .arg(&shader_path)
            .arg("-target")
            .arg("spirv")
            .arg("-o")
            .arg(&output_path)
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("cargo:warning=Successfully compiled {}", file_name);
            }
            Ok(s) => {
                panic!("Failed to compile shader {}: exit code {:?}", file_name, s.code());
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
                panic!("Failed to run slangc for {}: {}", file_name, error_msg);
            }
        }
    }
}

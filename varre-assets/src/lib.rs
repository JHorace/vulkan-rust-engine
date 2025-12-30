// Compiled shaders will be available at compile time via include_bytes!
// Example usage:
// pub const VERTEX_SHADER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders/shader.vert.spv"));
include!(concat!(env!("OUT_DIR"), "/shaders.rs"));

// Compiled shaders will be available at compile time via include_bytes!
// Example usage:
// pub const VERTEX_SHADER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders/shader.vert.spv"));

use include_bytes_aligned::include_bytes_aligned;
use glam::Vec3;

include!(concat!(env!("OUT_DIR"), "/shaders.rs"));
include!(concat!(env!("OUT_DIR"), "/models.rs"));
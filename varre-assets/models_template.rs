// Template for generated models.rs
// This file is read by build.rs and prepended to the generated model constants

use glam::Vec3;

#[derive(Debug, Clone)]
pub struct Model {
    pub verts: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub uvs: Vec<f32>,
}

impl Model {
    /// Load a model from binary data generated at build time
    pub const fn from_bytes(data: &'static [u8]) -> Self {
        Self {
            verts: Vec::new(),
            indices: Vec::new(),
            uvs: Vec::new(),
        }
    }

    /// Decode binary model data into runtime structures
    pub fn decode(data: &[u8]) -> Self {
        let mut offset = 0;

        // Read vertex count (u32)
        let vert_count = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]) as usize;
        offset += 4;

        // Read vertices (3 f32s per vertex)
        let mut verts = Vec::with_capacity(vert_count);
        for _ in 0..vert_count {
            let x = f32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            let y = f32::from_le_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]);
            let z = f32::from_le_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]);
            verts.push(Vec3::new(x, y, z));
            offset += 12;
        }

        // Read index count (u32)
        let index_count = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]) as usize;
        offset += 4;

        // Read indices (u32s)
        let mut indices = Vec::with_capacity(index_count);
        for _ in 0..index_count {
            let idx = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            indices.push(idx);
            offset += 4;
        }

        // Read UV count (u32)
        let uv_count = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]) as usize;
        offset += 4;

        // Read UVs (f32s)
        let mut uvs = Vec::with_capacity(uv_count);
        for _ in 0..uv_count {
            let uv = f32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
            uvs.push(uv);
            offset += 4;
        }

        Self { verts, indices, uvs }
    }
}

pub mod models {

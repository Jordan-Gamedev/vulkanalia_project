use crate::engine::Mesh;
use crate::engine::MeshBufferLayout;
use crate::engine::QuantizedModelMatrix;
use crate::engine::QuantizedVertex;
use crate::engine::UniformBufferObject;
use crate::engine::buffers::Buffer;
use crate::resources::AssetId;
use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct ModelHandle {
    pub vertex_buffer: Buffer<QuantizedVertex>,
    pub index_buffer: Buffer<u32>,
    pub uniform_buffers: Vec<Buffer<UniformBufferObject>>,
    pub dyn_model_matrix_buffer: Buffer<QuantizedModelMatrix>,
    pub static_model_matrix_buffer: Buffer<QuantizedModelMatrix>,
    pub loaded_meshes: HashMap<AssetId, Mesh>,
    pub mesh_uniform_buffer: Buffer<MeshBufferLayout>,
}

use crate::engine::InFlightBuffers;
use crate::engine::Mesh;
use crate::engine::MeshBufferLayout;
use crate::engine::QuantizedModelMatrix;
use crate::engine::QuantizedVertex;
use crate::engine::UniformBufferObject;
use crate::engine::buffer::Buffer;
use crate::resources::AssetId;
use std::collections::HashMap;

#[derive(Default)]
pub struct ModelHandle {
    pub vertex_buffer: Buffer<QuantizedVertex>,
    pub index_buffer: Buffer<u32>,
    pub uniform_buffers: InFlightBuffers<UniformBufferObject>,
    pub model_matrix_buffers: InFlightBuffers<QuantizedModelMatrix>,
    pub loaded_meshes: HashMap<AssetId, Mesh>,
    pub mesh_uniform_buffer: InFlightBuffers<MeshBufferLayout>,
}

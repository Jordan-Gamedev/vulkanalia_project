use crate::engine::InFlightBuffers;
use crate::engine::Mesh;
use crate::engine::MeshMetadata;
use crate::engine::QuantizedModelMatrix;
use crate::engine::UniformBufferObject;
use crate::resources::AssetId;
use std::collections::HashMap;
use vulkanalia::prelude::v1_0::*;
use vulkanalia_vma::*;

pub struct ModelHandle {
    pub uniform_buffers: InFlightBuffers<UniformBufferObject>,
    pub model_matrix_buffers: InFlightBuffers<QuantizedModelMatrix>,
    pub loaded_meshes: HashMap<AssetId, Mesh>,
    pub mesh_metadata_buffers: InFlightBuffers<MeshMetadata>,

    // Vertex Buffer
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_allocation: Allocation,
    pub vertex_buffer_base_device_address: vk::DeviceAddress,
    pub vertex_buffer_virtual_block: VirtualBlock,

    // Index Buffer
    pub index_buffer: vk::Buffer,
    pub index_buffer_allocation: Allocation,
    pub index_buffer_base_device_address: vk::DeviceAddress,
    pub index_buffer_virtual_block: VirtualBlock,

    // Staging Buffer
    pub staging_buffer: vk::Buffer,
    pub staging_buffer_allocation: Allocation,
    pub staging_buffer_mapped_memory: *mut u8,
}

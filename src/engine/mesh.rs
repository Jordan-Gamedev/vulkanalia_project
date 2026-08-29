use crate::engine::MeshMetadata;
use crate::resources::AssetId;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mesh {
    pub mesh_asset_id: AssetId,
    pub metadata: MeshMetadata,
    pub usage_count: u32,
    pub vertex_offset: u32,
    pub index_offset: u32,
    pub lod0_vertex_length: u32,
    pub lod0_index_length: u32,
    pub lod1_vertex_length: u32,
    pub lod1_index_length: u32,
    pub lod2_vertex_length: u32,
    pub lod2_index_length: u32,
    pub lod3_vertex_length: u32,
    pub lod3_index_length: u32,
}

impl Eq for Mesh {}
unsafe impl Sync for Mesh {}
unsafe impl Send for Mesh {}

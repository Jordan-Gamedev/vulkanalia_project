use crate::engine::{MeshAllocation, MeshMetadata};

#[derive(Default, PartialEq)]
pub struct Mesh {
    pub mesh_allocations: Vec<MeshAllocation>,
    pub metadata: MeshMetadata,
    pub usage_count: u32,
    pub lifetime: u32,
    pub metadata_index: u16,
}

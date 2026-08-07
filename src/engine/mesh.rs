use crate::engine::IndirectDrawData;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Mesh {
    pub vertex_offset: u32,
    pub vertex_length: u32,
    pub index_offset: u32,
    pub index_length: u32,
    pub indirect_draw_data_ptr: *mut IndirectDrawData,
}

unsafe impl Sync for Mesh {}
unsafe impl Send for Mesh {}

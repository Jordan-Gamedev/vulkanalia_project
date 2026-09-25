use vulkanalia::prelude::v1_0::*;
use vulkanalia_vma::*;

pub struct MeshAllocation {
    // Vertex Buffer
    pub vertex_virtual_allocation: VirtualAllocation,
    pub vertex_device_address: vk::DeviceAddress,
    pub vertex_offset: i32,
    pub vertex_count: i32,

    // Index Buffer
    pub index_virtual_allocation: VirtualAllocation,
    pub index_device_address: vk::DeviceAddress,
    pub index_offset: u32,
    pub index_count: u32,
}

impl PartialEq for MeshAllocation {
    fn eq(&self, other: &Self) -> bool {
        self.vertex_device_address == other.vertex_device_address
            && self.index_device_address == other.index_device_address
    }
}

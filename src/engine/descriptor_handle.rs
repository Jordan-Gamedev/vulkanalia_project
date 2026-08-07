use vulkanalia::prelude::v1_0::*;

#[derive(Clone)]
pub struct DescriptorHandle {
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
}

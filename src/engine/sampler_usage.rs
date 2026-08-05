use vulkanalia::prelude::v1_0::*;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct SamplerUsage {
    pub slot_index: u32,
    pub sampler: vk::Sampler,
    pub instance_count: u32,
}

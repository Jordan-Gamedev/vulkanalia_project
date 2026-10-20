use vulkanalia::prelude::v1_0::*;

#[derive(Clone, Copy, Debug)]
pub struct Texture {
    pub image: vk::Image,
    pub allocation: vulkanalia_vma::Allocation,
    pub view: vk::ImageView,
}

impl Texture {
    pub fn destroy(&self, device: Device, allocator: &vulkanalia_vma::Allocator) {
        unsafe {
            device.destroy_image_view(self.view, None);
            allocator.destroy_image(self.image, self.allocation);
        }
    }
}

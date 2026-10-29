use crate::components::TransformComponent;
use crate::engine::animations::AnimationSampler;
use crate::resources::{AssetId, get_asset_from_id};

#[derive(Debug, Clone)]
pub struct Animation {
    start_time: f32,
    end_time: f32,
    current_time: f32,
    is_looping: bool,
    name: String,
    samplers: Vec<AnimationSampler>,
}

impl Animation {
    pub fn new(name: String, animation_asset_id: AssetId, is_looping: bool) -> Self {
        let _bytes = get_asset_from_id(animation_asset_id).0;

        let animation = Self {
            start_time: f32::MAX,
            end_time: f32::MIN,
            current_time: 0.0,
            is_looping: is_looping,
            name: name,
            samplers: Vec::new(),
        };

        animation
    }

    pub fn update(&mut self, transform_component: &mut TransformComponent, delta_time: f32) {
        // Advance time
        self.current_time += delta_time;

        // Animation end logic
        if self.current_time > self.end_time {
            if self.is_looping {
                self.current_time = self.start_time + (self.current_time % self.end_time);
            } else {
                self.current_time = self.end_time
            }
        }

        // Update the channels using the current animation time
        for sampler in &self.samplers {
            sampler.update(&self.samplers, transform_component, self.current_time);
        }
    }
}

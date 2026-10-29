use crate::components::TransformComponent;
use crate::engine::animations::Animation;
use bevy_ecs::component::Component;
use glam::Quat;
use glam::Vec3;
use log::warn;

#[derive(Component)]
#[require(TransformComponent::default())]
pub struct AnimatorComponent {
    pub animations: Vec<Animation>,
    pub saved_position: Vec3,
    pub saved_rotation: Quat,
    pub saved_scale: Vec3,
    playing_animation_index: usize,
    time_scale: f32,
}

impl AnimatorComponent {
    pub fn play(&mut self, anim_index: usize) {
        if self.playing_animation_index != anim_index {
            let animation_count = self.animations.len();
            if anim_index < animation_count {
                self.playing_animation_index = anim_index;
            } else {
                warn!(
                    "Animation Index: {{{}}} is outside of range of animation count: {{{}}}",
                    anim_index, animation_count
                );
                self.stop();
            }
        }
    }

    pub fn stop(&mut self) {
        self.playing_animation_index = usize::MAX;
    }

    pub fn is_playing(&self) -> bool {
        self.playing_animation_index != usize::MAX
    }

    pub fn get_playing_animation(&self) -> Option<&Animation> {
        self.animations.get(self.playing_animation_index)
    }

    pub fn get_playing_animation_mut(&mut self) -> Option<&mut Animation> {
        self.animations.get_mut(self.playing_animation_index)
    }

    pub fn get_time_scale(&self) -> f32 {
        self.time_scale
    }

    pub fn set_time_scale(&mut self, mult: f32) {
        self.time_scale = mult.clamp(0.0, 10000.0);
    }
}

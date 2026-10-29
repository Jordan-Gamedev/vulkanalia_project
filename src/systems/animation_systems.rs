use bevy_ecs::query::With;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_time::Time;

use crate::components::AnimatorComponent;
use crate::components::RenderComponent;
use crate::components::TransformComponent;

pub fn reset_animated_transforms(
    query: Query<(&AnimatorComponent, &mut TransformComponent), With<RenderComponent>>,
) {
    for (animator, mut transform) in query {
        transform.position = animator.saved_position;
        transform.rotation = animator.saved_rotation;
        transform.scale = animator.saved_scale;
    }
}

pub fn animate_system(
    time: Res<Time>,
    query: Query<(&mut AnimatorComponent, &mut TransformComponent), With<RenderComponent>>,
) {
    for (mut animator, mut transform) in query {
        let time_scale = animator.get_time_scale();
        animator.saved_position = transform.position;
        animator.saved_rotation = transform.rotation;
        animator.saved_scale = transform.scale;

        if let Some(animation) = animator.get_playing_animation_mut() {
            animation.update(transform.as_mut(), time_scale * time.delta_secs());
        }
    }
}

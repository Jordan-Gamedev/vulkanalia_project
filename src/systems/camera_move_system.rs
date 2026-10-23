use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_time::Time;
use glam::Quat;
use std::f32::consts::PI;

use crate::components::CameraComponent;
use crate::components::TransformComponent;
use crate::input_manager::InputManager;

const DEG_TO_RAD: f32 = PI / 180.0;

pub fn camera_move_system(
    input_manager: Res<InputManager>,
    time: Res<Time>,
    camera_query: Query<(&mut TransformComponent, &mut CameraComponent)>,
) {
    for (mut transform, mut cam) in camera_query {
        // Rotation

        let look_input = input_manager.look_axis_input * time.delta_secs();

        cam.rotation_eulers.x -= look_input.y;
        cam.rotation_eulers.y -= look_input.x;
        cam.rotation_eulers.z = 0.0;

        cam.rotation_eulers.x = cam.rotation_eulers.x.clamp(-90.0, 90.0);
        cam.rotation_eulers.y %= 360.0;

        transform.rotation = Quat::from_euler(
            glam::EulerRot::YXZ,
            cam.rotation_eulers.y * DEG_TO_RAD,
            cam.rotation_eulers.x * DEG_TO_RAD,
            cam.rotation_eulers.z * DEG_TO_RAD,
        );

        // Position

        let move_input = input_manager
            .move_axis_input
            .try_normalize()
            .unwrap_or_default()
            * time.delta_secs()
            * 5.0;

        let local_left = transform.left();
        let local_forward = transform.forward();
        transform.position += (move_input.x * local_left) + (move_input.y * local_forward);

        // println!(
        //     "cam pos: {:.2}  |  cam rot: {:.2}",
        //     transform.position, cam.rotation_eulers
        // );

        break;
    }
}

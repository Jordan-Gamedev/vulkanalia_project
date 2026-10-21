use bevy_ecs::{resource::Resource, system::Res};
use bevy_input::ButtonInput;
use glam::Vec2;
use winit::{dpi::PhysicalPosition, keyboard::KeyCode};

use crate::input_action::KeyboardInputAction;

#[derive(Default, Resource)]
pub struct InputManager {
    // Raw Inputs
    pub prev_mouse_pos: Vec2,
    pub mouse_pos: Vec2,
    pub mouse_delta: Vec2,

    // Settings
    pub look_speed: Vec2,

    // Sanitized Inputs
    pub move_forward_keys: Vec<KeyCode>,
    pub move_back_keys: Vec<KeyCode>,
    pub move_left_keys: Vec<KeyCode>,
    pub move_right_keys: Vec<KeyCode>,

    // Gameplay
    pub look_axis_input: Vec2,
    pub move_axis_input: Vec2,
    pub jump_input: KeyboardInputAction,
}

impl InputManager {
    pub fn new(look_speed: f32) -> Self {
        let mut input_manager = InputManager::default();
        input_manager.look_speed = Vec2::new(look_speed, look_speed);
        input_manager.move_forward_keys.push(KeyCode::KeyW);
        input_manager.move_forward_keys.push(KeyCode::ArrowUp);
        input_manager.move_back_keys.push(KeyCode::KeyS);
        input_manager.move_back_keys.push(KeyCode::ArrowDown);
        input_manager.move_left_keys.push(KeyCode::KeyA);
        input_manager.move_left_keys.push(KeyCode::ArrowLeft);
        input_manager.move_left_keys.push(KeyCode::KeyD);
        input_manager.move_right_keys.push(KeyCode::ArrowRight);
        input_manager.jump_input = KeyboardInputAction::new(vec![KeyCode::Space]);
        input_manager
    }

    pub fn clear_inputs(&mut self) {
        self.jump_input.clear();
    }

    pub fn update_keyboard_inputs(
        &mut self,
        keyboard: &Res<ButtonInput<KeyCode>>,
        time: &Res<bevy_time::Time>,
    ) {
        self.jump_input.update(&keyboard, &time);
    }

    pub fn update_mouse_move_inputs(&mut self, position: PhysicalPosition<f64>) {
        self.prev_mouse_pos = self.mouse_pos;
        self.mouse_pos = Vec2::new(position.x as f32, position.y as f32);
        self.mouse_delta = self.mouse_pos - self.prev_mouse_pos;
    }
}

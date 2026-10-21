use bevy_ecs::resource::Resource;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_input::ButtonInput;
use bevy_input::gamepad::Gamepad;
use bevy_input::gamepad::GamepadButton;
use glam::Vec2;
use winit::dpi::PhysicalPosition;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;
use winit::window::CursorGrabMode;
use winit::window::Window;

use crate::input_action::InputAction;

#[derive(Resource)]
pub struct InputManager {
    // Raw Inputs
    pub prev_mouse_pos: Vec2,
    pub mouse_pos: Vec2,
    pub mouse_delta: Vec2,
    pub is_cursor_visible: bool,

    // Settings
    pub look_speed: Vec2,

    // Inputs
    pub move_forward_input: InputAction,
    pub move_back_input: InputAction,
    pub move_left_input: InputAction,
    pub move_right_input: InputAction,
    pub jump_input: InputAction,
    pub aim_input: InputAction,
    pub shoot_input: InputAction,

    // Gameplay
    pub look_axis_input: Vec2,
    pub move_axis_input: Vec2,
}

impl InputManager {
    pub fn new(look_speed: f32) -> Self {
        Self {
            prev_mouse_pos: Vec2::ZERO,
            mouse_pos: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            is_cursor_visible: true,
            look_speed: Vec2::new(look_speed, look_speed),
            move_forward_input: InputAction::new(
                vec![KeyCode::KeyW, KeyCode::ArrowUp],
                vec![],
                vec![],
            ),
            move_back_input: InputAction::new(
                vec![KeyCode::KeyS, KeyCode::ArrowDown],
                vec![],
                vec![],
            ),
            move_left_input: InputAction::new(
                vec![KeyCode::KeyA, KeyCode::ArrowLeft],
                vec![],
                vec![],
            ),
            move_right_input: InputAction::new(
                vec![KeyCode::KeyD, KeyCode::ArrowRight],
                vec![],
                vec![],
            ),
            look_axis_input: Vec2::ZERO,
            move_axis_input: Vec2::ZERO,
            jump_input: InputAction::new(vec![KeyCode::Space], vec![GamepadButton::South], vec![]),
            aim_input: InputAction::new(
                vec![],
                vec![GamepadButton::LeftTrigger2],
                vec![MouseButton::Right],
            ),
            shoot_input: InputAction::new(
                vec![],
                vec![GamepadButton::RightTrigger2],
                vec![MouseButton::Left],
            ),
        }
    }

    pub fn clear_inputs(&mut self) {
        self.move_forward_input.clear();
        self.move_back_input.clear();
        self.move_left_input.clear();
        self.move_right_input.clear();
        self.jump_input.clear();
        self.aim_input.clear();
        self.shoot_input.clear();
    }

    pub fn update(
        &mut self,
        keyboard: &Res<ButtonInput<KeyCode>>,
        gamepads: &Query<&Gamepad>,
        mouse: &Res<ButtonInput<MouseButton>>,
        time: &Res<bevy_time::Time>,
        window: &Window,
    ) {
        let controlling_gamepad = gamepads.iter().next();

        if self.is_cursor_visible
            && (mouse.just_pressed(MouseButton::Left)
                || controlling_gamepad.is_some()
                    && controlling_gamepad
                        .unwrap()
                        .digital()
                        .any_just_pressed(GamepadButton::all()))
        {
            // Lock the cursor inside the window and make it invisible
            if let Err(_) = window.set_cursor_grab(CursorGrabMode::Locked) {
                window.set_cursor_grab(CursorGrabMode::Confined).unwrap();
            }
            window.set_cursor_visible(false);
            self.is_cursor_visible = false;
        } else if !self.is_cursor_visible && keyboard.just_pressed(KeyCode::Escape) {
            // Free cursor and make it visible again
            window.set_cursor_grab(CursorGrabMode::None).unwrap();
            window.set_cursor_visible(true);
            self.is_cursor_visible = true;
        }

        self.move_forward_input
            .update(&keyboard, gamepads, &mouse, &time);
        self.move_back_input
            .update(&keyboard, gamepads, &mouse, &time);
        self.move_left_input
            .update(&keyboard, gamepads, &mouse, &time);
        self.move_right_input
            .update(&keyboard, gamepads, &mouse, &time);
        self.jump_input.update(&keyboard, gamepads, &mouse, &time);
        self.aim_input.update(&keyboard, gamepads, &mouse, &time);
        self.shoot_input.update(&keyboard, gamepads, &mouse, &time);
    }

    pub fn update_mouse_move_inputs(&mut self, position: PhysicalPosition<f64>) {
        self.prev_mouse_pos = self.mouse_pos;
        self.mouse_pos = Vec2::new(position.x as f32, position.y as f32);
        self.mouse_delta = self.mouse_pos - self.prev_mouse_pos;
    }
}

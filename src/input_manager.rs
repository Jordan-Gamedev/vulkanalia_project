use bevy_ecs::resource::Resource;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_input::ButtonInput;
use bevy_input::gamepad::Gamepad;
use bevy_input::gamepad::GamepadAxis;
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
    pub mouse_position: Vec2,
    pub mouse_delta: Vec2,
    pub is_cursor_visible: bool,

    // Settings
    pub gamepad_look_speed: Vec2,
    pub mouse_look_speed: f32,
    pub gamepad_left_stick_deadzone: f32,
    pub gamepad_right_stick_deadzone: f32,

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
    pub fn new(
        gamepad_look_speed: Vec2,
        mouse_look_speed: f32,
        gamepad_left_stick_deadzone: f32,
        gamepad_right_stick_deadzone: f32,
    ) -> Self {
        Self {
            mouse_position: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            is_cursor_visible: true,
            gamepad_look_speed: gamepad_look_speed,
            mouse_look_speed: mouse_look_speed,
            gamepad_left_stick_deadzone: gamepad_left_stick_deadzone,
            gamepad_right_stick_deadzone: gamepad_right_stick_deadzone,
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
        self.look_axis_input = Vec2::ZERO;
        self.move_axis_input = Vec2::ZERO;
        self.mouse_delta = Vec2::ZERO;
    }

    pub fn update(
        &mut self,
        keyboard: &Res<ButtonInput<KeyCode>>,
        gamepads: &Query<&Gamepad>,
        mouse: &Res<ButtonInput<MouseButton>>,
        time: &Res<bevy_time::Time>,
        window: &Window,
    ) {
        let gamepad = gamepads.iter().next();

        // Update cursor

        if self.is_cursor_visible
            && (mouse.just_pressed(MouseButton::Left)
                || gamepad.is_some()
                    && gamepad
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

        // Update button inputs

        self.move_forward_input
            .update(&keyboard, &gamepad, &mouse, &time);
        self.move_back_input
            .update(&keyboard, &gamepad, &mouse, &time);
        self.move_left_input
            .update(&keyboard, &gamepad, &mouse, &time);
        self.move_right_input
            .update(&keyboard, &gamepad, &mouse, &time);
        self.jump_input.update(&keyboard, &gamepad, &mouse, &time);
        self.aim_input.update(&keyboard, &gamepad, &mouse, &time);
        self.shoot_input.update(&keyboard, &gamepad, &mouse, &time);

        // Update axis input

        if gamepad.is_some()
            && let Some(left_stick_x) = gamepad.unwrap().get(GamepadAxis::LeftStickX)
            && let Some(left_stick_y) = gamepad.unwrap().get(GamepadAxis::LeftStickY)
        {
            if left_stick_x.abs() > self.gamepad_left_stick_deadzone {
                self.move_axis_input.x = left_stick_x;
            }
            if left_stick_y.abs() > self.gamepad_left_stick_deadzone {
                self.move_axis_input.y = left_stick_y;
            }
        }

        if self.move_axis_input == Vec2::ZERO {
            let mut x_movement = 0;
            let mut y_movement = 0;
            if self.move_forward_input.is_held {
                y_movement += 1;
            }
            if self.move_back_input.is_held {
                y_movement -= 1;
            }
            if self.move_right_input.is_held {
                x_movement += 1;
            }
            if self.move_left_input.is_held {
                x_movement -= 1;
            }
            self.move_axis_input = Vec2::new(x_movement as f32, y_movement as f32);
        }

        if gamepad.is_some()
            && let Some(right_stick_x) = gamepad.unwrap().get(GamepadAxis::RightStickX)
            && let Some(right_stick_y) = gamepad.unwrap().get(GamepadAxis::RightStickY)
        {
            if right_stick_x.abs() > self.gamepad_right_stick_deadzone {
                self.look_axis_input.x = right_stick_x;
            }
            if right_stick_y.abs() > self.gamepad_right_stick_deadzone {
                self.look_axis_input.y = right_stick_y;
            }
        }

        if self.look_axis_input == Vec2::ZERO {
            self.look_axis_input = self.mouse_delta;
        }
    }

    pub fn update_mouse_delta(&mut self, dx: f64, dy: f64) {
        self.mouse_delta = Vec2::new(dx as f32, dy as f32);
    }

    pub fn update_mouse_position(&mut self, mouse_position: PhysicalPosition<f64>) {
        self.mouse_position = Vec2::new(mouse_position.x as f32, mouse_position.y as f32);
    }
}

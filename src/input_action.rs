use bevy_ecs::system::Query;
use bevy_input::ButtonInput;
use bevy_input::gamepad::Gamepad;
use bevy_input::gamepad::GamepadButton;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

pub struct InputAction {
    pub key_binds: Vec<KeyCode>,
    pub gamepad_binds: Vec<GamepadButton>,
    pub mouse_binds: Vec<MouseButton>,
    pub is_pressed: bool,
    pub is_just_released: bool,
    pub is_held: bool,
    pub hold_time: f32,
}

impl InputAction {
    pub fn new(
        key_binds: Vec<KeyCode>,
        gamepad_binds: Vec<GamepadButton>,
        mouse_binds: Vec<MouseButton>,
    ) -> Self {
        Self {
            key_binds: key_binds,
            gamepad_binds: gamepad_binds,
            mouse_binds: mouse_binds,
            is_pressed: false,
            is_just_released: false,
            is_held: false,
            hold_time: 0.0,
        }
    }

    pub fn update(
        &mut self,
        keyboard_input: &ButtonInput<KeyCode>,
        gamepads_input: &Query<&Gamepad>,
        mouse_input: &ButtonInput<MouseButton>,
        time: &bevy_time::Time,
    ) {
        let key_binds_count = self.key_binds.len();
        let gamepad_binds_count = if gamepads_input.is_empty() {
            0
        } else {
            self.gamepad_binds.len()
        };
        let mouse_binds_count = self.mouse_binds.len();

        let controlling_gamepad = gamepads_input.iter().next();

        if (key_binds_count > 0 && keyboard_input.any_just_pressed(self.key_binds.clone()))
            || (gamepad_binds_count > 0
                && controlling_gamepad
                    .unwrap()
                    .digital()
                    .any_just_pressed(self.gamepad_binds.clone()))
            || (mouse_binds_count > 0 && mouse_input.any_just_pressed(self.mouse_binds.clone()))
        {
            self.is_pressed = true;
        } else if (key_binds_count > 0 && keyboard_input.any_just_released(self.key_binds.clone()))
            || (gamepad_binds_count > 0
                && controlling_gamepad
                    .unwrap()
                    .digital()
                    .any_just_released(self.gamepad_binds.clone()))
            || (mouse_binds_count > 0 && mouse_input.any_just_released(self.mouse_binds.clone()))
        {
            self.is_just_released = true;
        }

        if self.is_held {
            self.hold_time += time.delta_secs()
        } else {
            self.hold_time = 0.0;
        }

        self.is_held = (key_binds_count > 0 && keyboard_input.any_pressed(self.key_binds.clone()))
            || (gamepad_binds_count > 0
                && controlling_gamepad
                    .unwrap()
                    .any_pressed(self.gamepad_binds.clone()))
            || (mouse_binds_count > 0 && mouse_input.any_pressed(self.mouse_binds.clone()));
    }

    pub fn clear(&mut self) {
        self.is_pressed = false;
        self.is_just_released = false;
    }
}

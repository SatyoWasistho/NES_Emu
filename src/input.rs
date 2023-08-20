use glium::glutin::event::{KeyboardInput, ElementState};

/*
Input:

On the original hardware, all controller inputs are registered
in CPU address space. Address $4016 holds the controller state
for port 1 and $4017 holds the controller state for port 2.

While the press state of every button on a controller can be stored
in a single byte, the controller read actually uses a bit-shift
mechanism to report each button state. One read returns the state
of a single button (0x01 for pressed, 0x00 for unpressed), and the
next read returns the state of a different button and so on, and
the cycle repeats.

The button sequence is as follows:
A -> B -> Select -> Start -> Up -> Down -> Left -> Right

The controller has a strobe flag as well. When in strobe mode,
the bit-shift is disabled and reset to the A button. Resetting
the strobe flag enables the bit-shift.
*/

#[derive(Clone)]
pub struct Controller {
    shift: u8,
    button_states: u8,
    strobe: bool
}

impl Controller {
    pub fn new() -> Self {
        Controller {
            shift: 0x80,
            button_states: 0x00,
            strobe: false
        }
    }
    pub fn read(&mut self) -> u8 {
        let mut res = 0;
        if self.button_states & self.shift != 0 {
            res |= 0x01;
        }
        if !self.strobe {
            self.shift = self.shift >> 1;
            if self.shift == 0 {
                self.shift = 0x80
            }
        }
        res
    }
    pub fn set_strobe(&mut self) {
        self.strobe = true;
        self.shift = 0x80;
    }
    pub fn reset_strobe(&mut self) {
        self.strobe = false;
    }
    pub fn keyboard_input(&mut self, key: KeyboardInput) {
        let mut button: u8 = 0x00;
        match key.scancode {
            22 => { //U
                button = 0x80;
            },
            23 => { //I
                button = 0x40;
            },
            47 => { //V
                button = 0x20;
            },
            48 => { //B
                button = 0x10;
            },
            17 => { //W
                button = 0x08;
            },
            31 => { //S
                button = 0x04;
            },
            30 => { //A
                button = 0x02;
            },
            32 => { //D
                button = 0x01;
            },
            _ => (),
        }
        if key.state == ElementState::Pressed {
            self.button_states |= button;
        } else {
            self.button_states &= !button;
        }
    }
}
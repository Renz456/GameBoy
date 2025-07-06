pub struct Joypad {
    p1: u8,           // Joypad register at 0xFF00
    right: bool,
    left: bool,
    up: bool,
    down: bool,
    a: bool,
    b: bool,
    select: bool,
    start: bool,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            p1: 0xFF,      // Default value with all bits set
            right: false,
            left: false,
            up: false,
            down: false,
            a: false,
            b: false,
            select: false,
            start: false,
        }
    }

    pub fn read_register(&self) -> u8 {
        // Bits 4-5 of P1 determine which buttons to read
        // Bit 4 = 0: Read D-pad buttons
        // Bit 5 = 0: Read action buttons
        // Bits 0-3 contain the button states (active low)
        let mut result = self.p1 & 0xF0; // Keep the selection bits

        if (self.p1 & 0x10) == 0 {
            // D-pad buttons selected
            let mut dpad = 0x0F;
            if self.right { dpad &= !0x01; }
            if self.left { dpad &= !0x02; }
            if self.up { dpad &= !0x04; }
            if self.down { dpad &= !0x08; }
            result |= dpad;
        }
        if (self.p1 & 0x20) == 0 {
            // Action buttons selected
            let mut action = 0x0F;
            if self.a { action &= !0x01; }
            if self.b { action &= !0x02; }
            if self.select { action &= !0x04; }
            if self.start { action &= !0x08; }
            result |= action;
        }

        result
    }

    pub fn write_register(&mut self, value: u8) {
        // Only bits 4-5 are writable
        self.p1 = (self.p1 & 0xCF) | (value & 0x30);
    }

    pub fn set_button_state(&mut self, button: Button, pressed: bool) -> bool {
        let old_state = match button {
            Button::Right => self.right,
            Button::Left => self.left,
            Button::Up => self.up,
            Button::Down => self.down,
            Button::A => self.a,
            Button::B => self.b,
            Button::Select => self.select,
            Button::Start => self.start,
        };

        // Update button state
        match button {
            Button::Right => self.right = pressed,
            Button::Left => self.left = pressed,
            Button::Up => self.up = pressed,
            Button::Down => self.down = pressed,
            Button::A => self.a = pressed,
            Button::B => self.b = pressed,
            Button::Select => self.select = pressed,
            Button::Start => self.start = pressed,
        }

        // Check if we need to trigger an interrupt
        // Interrupt is triggered when a button is pressed (goes from false to true)
        // and the corresponding button group is selected
        if !old_state && pressed {
            match button {
                Button::Right | Button::Left | Button::Up | Button::Down => {
                    if (self.p1 & 0x10) == 0 {
                        return true;
                    }
                }
                Button::A | Button::B | Button::Select | Button::Start => {
                    if (self.p1 & 0x20) == 0 {
                        return true;
                    }
                }
            }
        }
        
        false
    }
}

#[derive(Copy, Clone)]
pub enum Button {
    Right,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start,
}

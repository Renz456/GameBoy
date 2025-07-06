pub struct Serial {
    sb: u8,  // Serial transfer data (0xFF01)
    sc: u8,  // Serial transfer control (0xFF02)
    clock_cycles: u64,
    transfer_cycles: u32,
}

impl Serial {
    pub fn new() -> Self {
        Serial {
            sb: 0,
            sc: 0,
            clock_cycles: 0,
            transfer_cycles: 0,
        }
    }

    pub fn read_register(&self, address: u16) -> u8 {
        match address {
            0xFF01 => self.sb,
            0xFF02 => self.sc,
            _ => panic!("Invalid serial register address: {}", address),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        match address {
            0xFF01 => self.sb = value,
            0xFF02 => {
                self.sc = value;
                // If transfer is started (bit 7 is set)
                if (value & 0x80) != 0 {
                    self.transfer_cycles = 0;
                }
            }
            _ => panic!("Invalid serial register address: {}", address),
        }
    }

    pub fn do_cycle(&mut self, ticks: u32) -> bool {
        let mut interrupt_triggered = false;

        // Check if transfer is in progress (bit 7 of SC is set)
        if (self.sc & 0x80) != 0 {
            self.transfer_cycles += ticks;

            // Serial transfer takes 8 bits * 512 cycles per bit = 4096 cycles
            if self.transfer_cycles >= 4096 {
                // Transfer complete
                self.sc &= !0x80; // Clear transfer start bit
                interrupt_triggered = true;
            }
        }

        interrupt_triggered
    }
} 
pub struct Timer {
    div: u8, // Divider register at 0xFF04
    tima: u8, // Timer counter at 0xFF05
    tma: u8, // Timer modulo at 0xFF06
    tac: u8, // Timer control at 0xFF07
    pub clock_cycles: u64,
    internal_div: u16, // Internal counter for DIV
    internal_tima: u16, // Internal counter for TIMA
}

impl Timer {
    pub fn new() -> Self {
        Timer { 
            div: 0, 
            tima: 0, 
            tma: 0, 
            tac: 0, 
            clock_cycles: 0,
            internal_div: 0,
            internal_tima: 0,
        }
    }

    pub fn read_register(&self, address: u16) -> u8 {
        match address {
            0xFF04 => self.div,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac,
            _ => panic!("Invalid timer register address: {}", address),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        match address {
            0xFF04 => self.div = value,
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => self.tac = value,
            _ => panic!("Invalid timer register address: {}", address),
        }
    }

    pub fn do_cycle(&mut self, ticks: u32) -> bool {
        let mut interrupt_triggered = false;
        
        // Update DIV register (16384 Hz)
        self.internal_div = self.internal_div.wrapping_add(ticks as u16);
        while self.internal_div >= 256 {
            self.div = self.div.wrapping_add(1);
            self.internal_div -= 256;
        }

        // Update TIMA if timer is enabled
        if (self.tac & 0x04) != 0 {
            let tima_ticks = match self.tac & 0x03 {
                0 => 1024, // 4096 Hz
                1 => 16,   // 262144 Hz
                2 => 64,   // 65536 Hz
                3 => 256,  // 16384 Hz
                _ => unreachable!(),
            };

            self.internal_tima = self.internal_tima.wrapping_add(ticks as u16);
            while self.internal_tima >= tima_ticks {
                self.tima = self.tima.wrapping_add(1);
                if self.tima == 0 {
                    self.tima = self.tma;
                    // TODO: Trigger timer interrupt
                    interrupt_triggered = true;
                }
                self.internal_tima -= tima_ticks;
            }
        }

        interrupt_triggered
    }
}


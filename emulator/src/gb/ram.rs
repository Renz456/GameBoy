pub const INTERRUPT_ENABLE_ADDRESS: u16 = 0xFFFF;
pub const INTERRUPT_FLAGS_ADDRESS: u16 = 0xFF0F;

/*
const EXT_RAM_SIZE: usize = 8192;
const W_RAM_SIZE: usize = 8192;
const ECHO_RAM_SIZE: usize = 7679;
const H_RAM_SIZE: usize = 127;
const OAM_SIZE: usize = 159;
const IO_SIZE: usize = 127;

const USER_PROGRAM_AREA_ADDRESS: u16 = 0x100;
const VRAM_ADDRESS: u16 = 0x8000;
const EXT_RAM_ADDRESS: u16 = 0xA000;
const ECHO_RAM_ADDRESS: u16 = 0xE000;
const W_RAM_ADDRESS: u16 = 0xC000;
const OAM_ADDRESS: u16 = 0xFE00;
const IO_ADDRESS: u16 = 0xFF00;
const H_RAM_ADDR: u16 = 0xFF80;
const BG_PAL_ADDR: u16 = 0xFF47;

need to figure out what the above values/addresses are
*/


pub struct RAM {
    memory: [u8; 0xFFFF], // 65535 bytes (64KB) of memory
}

impl RAM {
    pub fn new() -> Self {
        RAM {
            memory: [0; 0xFFFF],
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }

    pub fn write(&mut self, address: u16, value: u8) {
        self.memory[address as usize] = value;
    }
}
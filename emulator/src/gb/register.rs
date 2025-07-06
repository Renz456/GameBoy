macro_rules! get_set {
    ($reg:ident, $get_name:ident, $set_name:ident, $size:ty) => {
        pub fn $get_name(&self) -> $size {
            self.$reg
        }

        pub fn $set_name(&mut self, val: $size) {
            self.$reg = val;
        }
    };
}

macro_rules! get_set_u16 {
    ($reg1:ident, $reg2:ident, $get_name:ident, $set_name:ident) => {
        pub fn $get_name(&self) -> u16 {
            ((self.$reg1 as u16) << 8) | (self.$reg2 as u16)
        }

        pub fn $set_name(&mut self, val: u16) {
            self.$reg1 = ((val & 0xFF00) >> 8) as u8;
            self.$reg2 = (val & 0xFF) as u8;
        }
    };
}

pub struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum FlagMasks {
    ZERO = 0b10000000,
    SUBTRACT = 0b01000000,
    HALF_CARRY = 0b00100000,
    CARRY = 0b00010000,
}
  
  
  
pub struct Flags {
    pub zero: bool,
    pub subtract: bool,
    pub half_carry: bool,
    pub carry: bool,
}

impl Flags {
    pub fn new() -> Self {
        Flags {
            zero: false,
            subtract: false,
            half_carry: false,
            carry: false,
        }
    }

    pub fn to_u8(&self) -> u8 {
        (if self.zero       { 1 } else { 0 }) << (FlagMasks::ZERO as u8).trailing_zeros() |
        (if self.subtract   { 1 } else { 0 }) << (FlagMasks::SUBTRACT as u8).trailing_zeros() |
        (if self.half_carry { 1 } else { 0 }) << (FlagMasks::HALF_CARRY as u8).trailing_zeros() |
        (if self.carry      { 1 } else { 0 }) << (FlagMasks::CARRY as u8).trailing_zeros()
    }

    pub fn from_u8(value: u8) -> Flags {
        Flags {
            zero: (value & FlagMasks::ZERO as u8) != 0,
            subtract: (value & FlagMasks::SUBTRACT as u8) != 0,
            half_carry: (value & FlagMasks::HALF_CARRY as u8) != 0,
            carry: (value & FlagMasks::CARRY as u8) != 0,
        }
    }
}

impl Registers {
    pub fn new() -> Self {
        Registers {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }

    get_set!(a, get_a, set_a, u8);
    get_set!(b, get_b, set_b, u8);
    get_set!(c, get_c, set_c, u8);
    get_set!(d, get_d, set_d, u8);
    get_set!(e, get_e, set_e, u8);
    // get_set!(f, get_f, set_f, u8);
    get_set!(h, get_h, set_h, u8);
    get_set!(l, get_l, set_l, u8);
    
    get_set!(sp, get_sp, set_sp, u16);
    get_set!(pc, get_pc, set_pc, u16);
    
    /*
    af, bc, de, hl
    */
    get_set_u16!(a, f, get_af, set_af);
    get_set_u16!(b, c, get_bc, set_bc);
    get_set_u16!(d, e, get_de, set_de);
    get_set_u16!(h, l, get_hl, set_hl); 

    pub fn get_f(&self) -> u8 {
        self.f
    }

    pub fn set_f(&mut self, value: u8) {
        let flags = Flags::from_u8(value);
        self.f = flags.to_u8();
        // self.a = flags.zero as u8; // TODO: Check if I need to do this
    }

    pub fn get_and_increment_pc(&mut self) -> u16 {
        let pc = self.pc;
        self.pc += 1;
        pc
    }
    
    pub fn increment_pc(&mut self) -> u16 {
        self.pc += 1;
        self.pc
    }
    

}
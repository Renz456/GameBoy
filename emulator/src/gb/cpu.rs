use crate::gb::register::Registers;
use crate::gb::register::Flags;
use crate::gb::register::FlagMasks;
use crate::gb::ram::RAM;

pub struct Code {
  pub opcode: u8,
  pub mnemonic: String,
  pub operands: Vec<String>,
  pub cycles: u8,
  pub size: u8,
}

enum Interrupt {
  VBLANK = 0x01,
  LCD_STAT = 0x02,
  TIMER = 0x04,
  SERIAL = 0x08,
  JOYPAD = 0x10,
}

enum InterruptHandlers {
  VBLANK = 0x40,
  LCD_STAT = 0x48,
  TIMER = 0x50,
  SERIAL = 0x58,
  JOYPAD = 0x60,
}


pub enum Instruction {
  ADD(ArithmeticTarget),
  ADC(ArithmeticTarget),
  SUB(ArithmeticTarget),
  SBC(ArithmeticTarget),
  AND(ArithmeticTarget),
  OR(ArithmeticTarget),
  XOR(ArithmeticTarget),
  CP(ArithmeticTarget),
  RL(bool),
  RR(bool),
  DAA,

  ADD_IMM(u8),
  ADC_IMM(u8),
  SUB_IMM(u8),
  SBC_IMM(u8),
  AND_IMM(u8),
  OR_IMM(u8),
  XOR_IMM(u8),
  CP_IMM(u8),

  ADD_MEM,
  ADC_MEM,
  SUB_MEM,
  SBC_MEM,
  AND_MEM,
  OR_MEM,
  XOR_MEM,
  CP_MEM,


  // NOP
  ADD_HL(ArithmeticTarget, ArithmeticTarget),
  // NOP
  NOP,

  INC(ArithmeticTarget),
  DEC(ArithmeticTarget),
  INC_16(ArithmeticTarget, ArithmeticTarget),
  DEC_16(ArithmeticTarget, ArithmeticTarget),

  // Enable/Disable Interrupts
  EI, 
  DI, 

  // Stack Operations
  PUSH(ArithmeticTarget, ArithmeticTarget),
  POP(ArithmeticTarget, ArithmeticTarget),

  // Jump Operations
  RET(bool, bool, bool),
  RET_N(bool, bool),
  RST(u8),
  CALL(u16, bool, bool, bool),
  JR(bool, bool, bool, i8),
  JP(bool, bool, bool, u16),
  JP_HL(),

  // Load/Store Operations
  LD_RR(ArithmeticTarget, ArithmeticTarget),
  LD_MEM_REG(ArithmeticTarget),
  LD_REG_MEM(ArithmeticTarget),
  LD_MEM_IMM(u8),
  LD_REG_IMM(ArithmeticTarget, u8),
  LD_MEM_INC(bool, bool),
  LD_BCDE(ArithmeticTarget, ArithmeticTarget, bool),
  LD_IMM_16(u16, bool),
  LD_IMM_8(u8, bool),
  LD_AC(bool),
  LD_REG_IMM_16(ArithmeticTarget, ArithmeticTarget, u16),
  MOD_MEM(bool),

  STORE_SP(u16),
  INC_SP(i8),
  LD_SP_HL,
  LD_HL_SP(i8),

  SCF,
  CCF,
  CPL,
  

}

pub enum ArithmeticTarget {
  A, B, C, D, E, H, L, F, SP
}

const INTERRUPT_ENABLE_ADDRESS: u16 = 0xFFFF;
const INTERRUPT_FLAG_ADDRESS: u16 = 0xFF0F;

pub struct CPU<'a> {
  pub registers: Registers,
  pub flags: Flags,
  pub ram: &'a mut RAM,
  pub interrupt_master_enable: bool,
  previous_ime: bool, // TODO: not sure if this is needed
  pub halted: bool,
  pub stopped: bool,
  pub clock_cycles: u64,
}

macro_rules! pop_16bit {
    ($self:ident, $sp:expr, $setter:ident) => {{
        let lower_half = $self.ram.read(*$sp);
        *$sp += 1;
        let upper_half = $self.ram.read(*$sp);
        *$sp += 1;
        $self.registers.$setter(((upper_half as u16) << 8) | lower_half as u16);
    }};
}

macro_rules! push_16bit {
    ($self:ident, $sp:expr, $getter:ident) => {{
        let value = $self.registers.$getter();
        *$sp -= 1;  
        $self.ram.write(*$sp, ((value >> 8) & 0xFF) as u8);
        *$sp -= 1;
        $self.ram.write(*$sp, (value & 0xFF) as u8);
    }};
}



macro_rules! arithmetic_op {
    ($self:ident, $target:ident, $op:ident) => {
        match $target {
            ArithmeticTarget::A => $self.$op($self.registers.get_a()),
            ArithmeticTarget::B => $self.$op($self.registers.get_b()),
            ArithmeticTarget::C => $self.$op($self.registers.get_c()),
            ArithmeticTarget::D => $self.$op($self.registers.get_d()),
            ArithmeticTarget::E => $self.$op($self.registers.get_e()),
            ArithmeticTarget::H => $self.$op($self.registers.get_h()),
            ArithmeticTarget::L => $self.$op($self.registers.get_l()),
            ArithmeticTarget::F => panic!("Invalid arithmetic target"),
            ArithmeticTarget::SP => panic!("Invalid arithmetic target"),
        }
    };
}

macro_rules! carry_op {
    ($self:ident, $target:ident, $op:ident) => {
        let carry = $self.registers.get_f() as u8 >> (FlagMasks::CARRY as u8).trailing_zeros() as u8;
        match $target {
            ArithmeticTarget::A => $self.$op($self.registers.get_a() + carry),
            ArithmeticTarget::B => $self.$op($self.registers.get_b() + carry),
            ArithmeticTarget::C => $self.$op($self.registers.get_c() + carry),
            ArithmeticTarget::D => $self.$op($self.registers.get_d() + carry),
            ArithmeticTarget::E => $self.$op($self.registers.get_e() + carry),
            ArithmeticTarget::H => $self.$op($self.registers.get_h() + carry),
            ArithmeticTarget::L => $self.$op($self.registers.get_l() + carry),
            ArithmeticTarget::F => panic!("Invalid carry target"),
            ArithmeticTarget::SP => panic!("Invalid carry target"),
        }
    };
}

impl<'a> CPU<'a> {
  pub fn new(ram: &'a mut RAM) -> Self {
    CPU {
      registers: Registers::new(),
      flags: Flags::new(),
      ram: ram,
      interrupt_master_enable: false,
      previous_ime: false,
      halted: false,
      stopped: false,
      clock_cycles: 0,
    }
  }

  fn ei(&mut self) {
    self.interrupt_master_enable = true;
  }

  fn di(&mut self) {
    self.interrupt_master_enable = false;
  }

  fn set_flags(&mut self, zero: bool, subtract: bool, half_carry: bool, carry: bool) {
    let flags = Flags {
      zero,
      subtract,
      half_carry,
      carry,
    };
    eprintln!("flags: {:02x}, zero: {}, subtract: {}, half_carry: {}, carry: {}", flags.to_u8(), zero, subtract, half_carry, carry);
    self.registers.set_f(flags.to_u8());
  }

  fn get_flags(&self) -> Flags {
    Flags::from_u8(self.registers.get_f())
  }

  fn ld_rr(&mut self, destination: ArithmeticTarget, source: ArithmeticTarget) {
    
    let value = match source   {
        ArithmeticTarget::A => self.registers.get_a(),
        ArithmeticTarget::B => self.registers.get_b(),
        ArithmeticTarget::C => self.registers.get_c(),
        ArithmeticTarget::D => self.registers.get_d(),
        ArithmeticTarget::E => self.registers.get_e(),
        ArithmeticTarget::H => self.registers.get_h(),
        ArithmeticTarget::L => self.registers.get_l(),
        ArithmeticTarget::F => panic!("Invalid source register"),
        ArithmeticTarget::SP => panic!("Invalid source register"),
        _ => panic!("Invalid get method"),
    };

    match destination {
        ArithmeticTarget::A => self.registers.set_a(value),
        ArithmeticTarget::B => self.registers.set_b(value),
        ArithmeticTarget::C => self.registers.set_c(value),
        ArithmeticTarget::D => self.registers.set_d(value),
        ArithmeticTarget::E => self.registers.set_e(value),
        ArithmeticTarget::H => self.registers.set_h(value),
        ArithmeticTarget::L => self.registers.set_l(value),
        ArithmeticTarget::F => panic!("Invalid destination register"),
        ArithmeticTarget::SP => panic!("Invalid destination register"),
        _ => panic!("Invalid set method"),
    };
  }

  fn ld_reg_mem(&mut self, target: ArithmeticTarget) {
    let address = self.registers.get_hl();
    let value = self.ram.read(address);
    match target {
      ArithmeticTarget::A => self.registers.set_a(value),
      ArithmeticTarget::B => self.registers.set_b(value),
      ArithmeticTarget::C => self.registers.set_c(value),
      ArithmeticTarget::D => self.registers.set_d(value),
      ArithmeticTarget::E => self.registers.set_e(value),
      ArithmeticTarget::H => self.registers.set_h(value),
      ArithmeticTarget::L => self.registers.set_l(value),
      _ => panic!("Invalid destination register"),
    }
  }

  fn ld_mem_imm(&mut self, value: u8) {
    let address = self.registers.get_hl();
    self.ram.write(address, value);
  }

  fn ld_reg_imm(&mut self, target: ArithmeticTarget, value: u8) {
    match target {
      ArithmeticTarget::A => self.registers.set_a(value),
      ArithmeticTarget::B => self.registers.set_b(value),
      ArithmeticTarget::C => self.registers.set_c(value),
      ArithmeticTarget::D => self.registers.set_d(value),
      ArithmeticTarget::E => self.registers.set_e(value),
      ArithmeticTarget::H => self.registers.set_h(value),
      ArithmeticTarget::L => self.registers.set_l(value),
      _ => panic!("Invalid destination register"),
    }
  }
  
  fn ld_mem_inc(&mut self, increment: bool, load: bool) {
    let address = self.registers.get_hl();
    if increment {
      self.registers.set_hl(self.registers.get_hl() + 1);
    } else {
      self.registers.set_hl(self.registers.get_hl() - 1);
    }
    if load {
      let value = self.ram.read(address);
      self.registers.set_a(value);
    } else {
      self.ram.write(address, self.registers.get_a());
    }
    
  }

  fn mod_mem(&mut self, increment: bool) {
    let address = self.registers.get_hl();
    let value = self.ram.read(address);
    let half_carry;
    let zero;
    if increment {
      self.ram.write(address, value + 1);
      half_carry = (value & 0xF) + 1 > 0xF;
      zero = (value + 1) & 0xFF == 0;
    } else {
      self.ram.write(address, value - 1);
      half_carry = (value & 0xF) == 0;
      zero = (value - 1) & 0xFF == 0;
    }
    self.set_flags(half_carry, false, zero, !increment);
  }

  fn ld_bcde(&mut self, target1: ArithmeticTarget, target2: ArithmeticTarget, load: bool) {
    let address = match (target1, target2) {
      (ArithmeticTarget::B, ArithmeticTarget::C) => self.registers.get_bc(),
      (ArithmeticTarget::D, ArithmeticTarget::E) => self.registers.get_de(),
      _ => panic!("Invalid target"),
    };
    
    if load {
      self.registers.set_a(self.ram.read(address));
    } else {
      self.ram.write(address, self.registers.get_a());
    }
  }

  fn ld_imm_16(&mut self, address: u16, load: bool) {
    if load {
      let value = self.ram.read(address);
      self.registers.set_a(value);
    } else {
      self.ram.write(address, self.registers.get_a());
    }
  }

  fn ld_imm_8(&mut self, value: u8, load: bool) {
    let address = 0xFF00 + value as u16;
    if load {
      self.registers.set_a(self.ram.read(address));
    } else {
      self.ram.write(address, self.registers.get_a());
    }
  }

  fn ld_ac(&mut self, load: bool) {
    let address = 0xFF00 + self.registers.get_c() as u16;
    if load {
      self.registers.set_a(self.ram.read(address));
    } else {
      self.ram.write(address, self.registers.get_a());
    }
  }

  fn ld_reg_imm_16(&mut self, target1: ArithmeticTarget, target2: ArithmeticTarget, value: u16) {
    match (target1, target2) {
      (ArithmeticTarget::B, ArithmeticTarget::C) => self.registers.set_bc(value),
      (ArithmeticTarget::D, ArithmeticTarget::E) => self.registers.set_de(value),
      (ArithmeticTarget::H, ArithmeticTarget::L) => self.registers.set_hl(value),
      (ArithmeticTarget::SP, ArithmeticTarget::SP) => self.registers.set_sp(value),
      _ => panic!("Invalid destination register"),
    }
  }

  fn store_sp(&mut self, address: u16) {
    let sp = self.registers.get_sp();
    self.ram.write(address, (sp & 0xFF) as u8);
    self.ram.write(address + 1, (sp >> 8) as u8);
  }

  fn inc_sp(&mut self, value: i8) {
    let (result, carry) = self.registers.get_sp().overflowing_add((value as i16) as u16);
    self.registers.set_sp(result);
    let half_carry = (result & 0xFF) + (((value as i16) as u16) & 0xFF) > 0xFF;
    self.set_flags(half_carry, carry, false, false);
  }

  fn ld_sp_hl(&mut self) {
    self.registers.set_sp(self.registers.get_hl());
  }

  fn ld_hl_sp(&mut self, value: i8) {
    let (result, carry) = self.registers.get_sp().overflowing_add((value as i16) as u16);
    self.registers.set_hl(result);
    let half_carry = (result & 0xFF) + (((value as i16) as u16) & 0xFF) > 0xFF;
    self.set_flags(half_carry, carry, false, false);
  }

  fn ret(&mut self, carry: bool, zero: bool, interrupt: bool) {
    assert!(!carry || !zero);
    assert!(!(carry || zero) || !interrupt);
    let flags = self.registers.get_f();
    let is_carry_set = flags & (FlagMasks::CARRY as u8) != 0;
    let is_zero_set = flags & (FlagMasks::ZERO as u8) != 0;
    let is_interrupt_enabled = self.interrupt_master_enable;

    let should_jump = is_carry_set && carry || is_zero_set && zero || interrupt && is_interrupt_enabled  || (!carry && !zero && !interrupt); 
    if should_jump {
      let mut sp = self.registers.get_sp();
      pop_16bit!(self, &mut sp, set_pc);
      self.registers.set_sp(sp);
    }
    if interrupt {
      let previous_ime = self.previous_ime;
      self.previous_ime = self.interrupt_master_enable;
      self.interrupt_master_enable = previous_ime;
    }
  }

  fn ret_n(&mut self, carry: bool, zero: bool) {
    assert!(!carry || !zero);
    let flags = self.registers.get_f();
    let is_carry_set = flags & (FlagMasks::CARRY as u8) != 0;
    let is_zero_set = flags & (FlagMasks::ZERO as u8) != 0;
    let should_jump = !is_carry_set && carry || !is_zero_set && zero;
    if should_jump {
      let mut sp = self.registers.get_sp();
      pop_16bit!(self, &mut sp, set_pc);
      self.registers.set_sp(sp);
    }
  }
     
  fn rst(&mut self, value: u8) {
    assert!(0x00 <= value && value <= 0x7);
    let mut sp = self.registers.get_sp();
    push_16bit!(self, &mut sp, get_pc);
    self.registers.set_sp(sp);
    self.registers.set_pc(value as u16 * 8);
  }

  fn call(&mut self, address: u16, carry: bool, zero: bool, negative: bool) {
    assert!(!carry || !zero);
    let mut is_carry_set = self.registers.get_f() & (FlagMasks::CARRY as u8) != 0;
    let mut is_zero_set = self.registers.get_f() & (FlagMasks::ZERO as u8) != 0;
    is_carry_set = if negative { !is_carry_set } else { is_carry_set };
    is_zero_set = if negative { !is_zero_set } else { is_zero_set };
    
    let should_jump = carry && is_carry_set || zero && is_zero_set || !carry && !zero;
    if should_jump {
      let mut sp = self.registers.get_sp();
      let pc = self.registers.get_pc();
      self.registers.set_pc(pc + 3); // 3 bytes = 1 byte for instruction, 2 bytes for address
      push_16bit!(self, &mut sp, get_pc);
      self.registers.set_sp(sp);
      self.registers.set_pc(address);
    }
  }

  fn jr(&mut self, carry: bool, zero: bool, negative: bool, jump_value: i8) {
    assert!(!carry || !zero);
    let mut is_carry_set = self.registers.get_f() & (FlagMasks::CARRY as u8) != 0;
    let mut is_zero_set = self.registers.get_f() & (FlagMasks::ZERO as u8) != 0;
    is_carry_set = if negative { !is_carry_set } else { is_carry_set };
    is_zero_set = if negative { !is_zero_set } else { is_zero_set };

    let should_jump = carry && is_carry_set || zero && is_zero_set || !carry && !zero;
    if should_jump {
      let pc = self.registers.get_pc();
      let jump_value_i16 = jump_value as i16;
      /* TODO: Check if this is correct */
      let result = if jump_value_i16 >= 0 {
          pc.wrapping_add(jump_value_i16 as u16)
      } else {
          pc.wrapping_sub((-jump_value_i16) as u16)
      };
      self.registers.set_pc(result);
    }
  }

  fn jp(&mut self, carry: bool, zero: bool, negative: bool, jump_value: u16) {
    assert!(!carry || !zero);
    let mut is_carry_set = self.registers.get_f() & (FlagMasks::CARRY as u8) != 0;
    let mut is_zero_set = self.registers.get_f() & (FlagMasks::ZERO as u8) != 0;
    is_carry_set = if negative { !is_carry_set } else { is_carry_set };
    is_zero_set = if negative { !is_zero_set } else { is_zero_set };

    let should_jump = carry && is_carry_set || zero && is_zero_set || !carry && !zero;
    if should_jump {
      self.registers.set_pc(jump_value);
    }
  }

  fn jp_hl(&mut self) {
    self.registers.set_pc(self.registers.get_hl());
  }

  fn pop(&mut self, target: ArithmeticTarget, target2: ArithmeticTarget) {
    let mut sp = self.registers.get_sp();
    match (target, target2) {
      (ArithmeticTarget::B, ArithmeticTarget::C) => {
        pop_16bit!(self, &mut sp, set_bc);
      }
      (ArithmeticTarget::D, ArithmeticTarget::E) => {
        pop_16bit!(self, &mut sp, set_de);
      }
      (ArithmeticTarget::H, ArithmeticTarget::L) => {
        pop_16bit!(self, &mut sp, set_hl);
      }
      (ArithmeticTarget::A, ArithmeticTarget::F) => {
        pop_16bit!(self, &mut sp, set_af);
      }
      _ => {
        panic!("Invalid pop target");
      }
    }
    self.registers.set_sp(sp);
  }

  fn push(&mut self, target: ArithmeticTarget, target2: ArithmeticTarget) {
    let mut sp = self.registers.get_sp();
    match (target, target2) {
      (ArithmeticTarget::B, ArithmeticTarget::C) => {
        push_16bit!(self, &mut sp, get_bc);
      }
      (ArithmeticTarget::D, ArithmeticTarget::E) => {
        push_16bit!(self, &mut sp, get_de);
      }
      (ArithmeticTarget::H, ArithmeticTarget::L) => {
        push_16bit!(self, &mut sp, get_hl);
      } 
      (ArithmeticTarget::A, ArithmeticTarget::F) => {
        push_16bit!(self, &mut sp, get_af);
      }
      _ => {
        panic!("Invalid push target");
      }
    } 
    self.registers.set_sp(sp);
  } 

  fn add_hl(&mut self, target1: ArithmeticTarget, target2: ArithmeticTarget) {
    let value = self.registers.get_hl();
    
    let add_value = match (target1, target2) {
      (ArithmeticTarget::B, ArithmeticTarget::C) => self.registers.get_bc(),
      (ArithmeticTarget::D, ArithmeticTarget::E) => self.registers.get_de(),
      (ArithmeticTarget::H, ArithmeticTarget::L) => self.registers.get_hl(),
      (ArithmeticTarget::SP, ArithmeticTarget::SP) => self.registers.get_sp(),
      _ => panic!("Invalid add hl target"),
    };

    let (result, carry) = value.overflowing_add(add_value);
    self.registers.set_hl(result);
    let half_carry = (value & 0xFF) + (add_value & 0xFF) > 0xFF;
    self.set_flags(half_carry, carry, result == 0, false);
  }
  
  fn inc(&mut self, target: ArithmeticTarget) {
    let value: u16;
    match target {
      ArithmeticTarget::A => {
        value = self.registers.get_a() as u16;
        self.registers.set_a(self.registers.get_a() + 1);
      }
      ArithmeticTarget::B => {
        value = self.registers.get_b() as u16;
        self.registers.set_b(self.registers.get_b() + 1);
      }
      ArithmeticTarget::C => {
        value = self.registers.get_c() as u16;
        self.registers.set_c(self.registers.get_c() + 1);
      }
      ArithmeticTarget::D => {
        value = self.registers.get_d() as u16;
        self.registers.set_d(self.registers.get_d() + 1);
      }
      ArithmeticTarget::E => {
        value = self.registers.get_e() as u16;
        self.registers.set_e(self.registers.get_e() + 1);
      }
      ArithmeticTarget::H => {
        value = self.registers.get_h() as u16;
        self.registers.set_h(self.registers.get_h() + 1);
      }
      ArithmeticTarget::L => {
        value = self.registers.get_l() as u16;
        self.registers.set_l(self.registers.get_l() + 1);
      }
      ArithmeticTarget::SP => {
        value = self.registers.get_sp() as u16;
        self.registers.set_sp(self.registers.get_sp() + 1);
      }
      _ => panic!("Invalid increment target"),
    }

    if let ArithmeticTarget::SP = target {
        return;
    }

    let half_carry = (value & 0xF) + 1 > 0xF;
    let carry = (self.registers.get_f() & FlagMasks::CARRY as u8) != 0;
    let zero = (value + 1) & 0xFF == 0;
    self.set_flags(zero, false, half_carry, carry);
  }

  fn dec(&mut self, target: ArithmeticTarget) {
    let value: u16;
    match target {
      ArithmeticTarget::A => {
        value = self.registers.get_a() as u16;
        self.registers.set_a(self.registers.get_a() - 1); 
      }
      ArithmeticTarget::B => {
        value = self.registers.get_b() as u16;
        self.registers.set_b(self.registers.get_b() - 1);
      }
      ArithmeticTarget::C => {  
        value = self.registers.get_c() as u16;
        self.registers.set_c(self.registers.get_c() - 1);
      }
      ArithmeticTarget::D => {
        value = self.registers.get_d() as u16;
        self.registers.set_d(self.registers.get_d() - 1);
      } 
      ArithmeticTarget::E => {
        value = self.registers.get_e() as u16;
        self.registers.set_e(self.registers.get_e() - 1);
      }
      ArithmeticTarget::H => {
        value = self.registers.get_h() as u16;
        self.registers.set_h(self.registers.get_h() - 1);
      }
      ArithmeticTarget::L => {
        value = self.registers.get_l() as u16;
        self.registers.set_l(self.registers.get_l() - 1);
      } 
      ArithmeticTarget::SP => {
        value = self.registers.get_sp() as u16;
        self.registers.set_sp(self.registers.get_sp() - 1);
      }
      _ => panic!("Invalid decrement target"),
    }

    if let ArithmeticTarget::SP = target {
        return;
    }

    let half_carry = (value & 0xF) - 1 < 0;
    let carry = self.registers.get_f() & FlagMasks::CARRY as u8 != 0;
    let zero = (value - 1) & 0xFF == 0;
    self.set_flags(zero, true, half_carry, carry);
  }

  fn inc_16(&mut self, target1: ArithmeticTarget, target2: ArithmeticTarget) {
    match (target1, target2) {
      (ArithmeticTarget::B, ArithmeticTarget::C) => {
        self.registers.set_bc(self.registers.get_bc() + 1);
      }
      (ArithmeticTarget::D, ArithmeticTarget::E) => {
        self.registers.set_de(self.registers.get_de() + 1);
      }
      (ArithmeticTarget::H, ArithmeticTarget::L) => {
        self.registers.set_hl(self.registers.get_hl() + 1);
      }
      (ArithmeticTarget::SP, ArithmeticTarget::SP) => {
        self.registers.set_sp(self.registers.get_sp() + 1);
      }
      _ => {
        panic!("Invalid 16-bit increment target");
      }
    }
  }

  fn dec_16(&mut self, target1: ArithmeticTarget, target2: ArithmeticTarget) {
    let value: u16; 
    match (target1, target2) {
      (ArithmeticTarget::B, ArithmeticTarget::C) => {
        value = self.registers.get_bc();
        self.registers.set_bc(self.registers.get_bc() - 1);
      }
      (ArithmeticTarget::D, ArithmeticTarget::E) => {
        value = self.registers.get_de();
        self.registers.set_de(self.registers.get_de() - 1);
      }
      (ArithmeticTarget::H, ArithmeticTarget::L) => {
        value = self.registers.get_hl();
        self.registers.set_hl(self.registers.get_hl() - 1);
      }
      (ArithmeticTarget::SP, ArithmeticTarget::SP) => {
        self.registers.set_sp(self.registers.get_sp() - 1);
      }
      _ => {
        panic!("Invalid 16-bit decrement target");
      }
    }
  }

  fn nop(&mut self) {
    self.registers.increment_pc();
  }

  fn add(&mut self, value: u8) {
    let (result, carry) = self.registers.get_a().overflowing_add(value);
    self.registers.set_a(result);
    let half_carry = (self.registers.get_a() & 0xF) < (value & 0xF);
    let zero = result == 0;
    self.set_flags(zero, false, half_carry, carry);
  }
  
  fn sub(&mut self, value: u8) {
    let (result, carry) = self.registers.get_a().overflowing_sub(value);
    self.registers.set_a(result);
    let half_carry = (self.registers.get_a() & 0xF) < (value & 0xF);
    let zero = result == 0;
    self.set_flags(zero, true, half_carry, carry);
  }

  fn and(&mut self, value: u8) {
    self.registers.set_a(self.registers.get_a() & value);
    let zero = self.registers.get_a() == 0;
    self.set_flags(zero, false, true, false);
  }

  fn or(&mut self, value: u8) {
    self.registers.set_a(self.registers.get_a() | value);
    let zero = self.registers.get_a() == 0;
    self.set_flags(zero, false, false, false);
  }

  fn xor(&mut self, value: u8) {
    self.registers.set_a(self.registers.get_a() ^ value);
    let zero = self.registers.get_a() == 0;
    self.set_flags(zero, false, false, false);
  }
  
  fn cp(&mut self, value: u8) {
    let result = self.registers.get_a() - value;
    let carry = self.registers.get_a() < value;
    let half_carry = ((self.registers.get_a() & 0xF) + (value & 0xF)) > 0xF;
    let zero = result == 0;
    self.set_flags(zero, true, half_carry, carry);
  }

  fn rl(&mut self, carry: bool) {
    let value = self.registers.get_a();
    let mut result = value << 1;
    let overflow = value & (1 << 7) != 0;
    if carry {
      result |= overflow as u8;
    } else {
      let flags = self.registers.get_f();
      result |= (flags & (FlagMasks::CARRY as u8) >> 4) as u8;
    }
    self.registers.set_a(result);
    self.set_flags(false, false, false, overflow);
  }

  fn rr(&mut self, carry: bool) {
    let value = self.registers.get_a();
    let mut result = value >> 1;
    let overflow = value & 1 != 0;
    if carry {
      result |= (overflow as u8) << 7;
    } else {
      let flags = self.registers.get_f();
      result |= (flags & (FlagMasks::CARRY as u8) >> 4) as u8;
    }
    self.registers.set_a(result);
    self.set_flags(false, false, false, overflow);
  }

  fn daa(&mut self) {
    /*
    TODO: I don't fully understand this instruction.
     */
    let mut a = self.registers.get_a();
    let mut adjust = 0;
    let mut carry = false;
    let flags = self.registers.get_f();
    
    // Check if we need to adjust the lower nibble
    if (a & 0x0F) > 9 || (flags & (FlagMasks::HALF_CARRY as u8)) != 0 {
      adjust |= 0x06;
    }
    
    // Check if we need to adjust the upper nibble
    if (a >> 4) > 9 || (flags & (FlagMasks::CARRY as u8)) != 0 {
      adjust |= 0x60;
      carry = true;
    }
    
    // If we're in subtract mode, subtract the adjustment
    if (flags & (FlagMasks::SUBTRACT as u8)) != 0 {
      a = a.wrapping_sub(adjust);
    } else {
      a = a.wrapping_add(adjust);
    }
    
    // Set the flags
    let half_carry = (a & 0x0F) < (adjust & 0x0F);
    let zero = a == 0;
    self.set_flags(zero, (flags & (FlagMasks::SUBTRACT as u8)) != 0, half_carry, carry);
    
    self.registers.set_a(a);
  }

  fn scf(&mut self) {
    let flags = self.registers.get_f();
    self.set_flags((flags & (FlagMasks::ZERO as u8) != 0), false, false, true);
  }

  fn ccf(&mut self) {
    let flags = self.registers.get_f();
    self.set_flags((flags & (FlagMasks::ZERO as u8) != 0), false, false, !(flags & (FlagMasks::CARRY as u8) != 0));
  }

  fn cpl(&mut self) {
    let a = self.registers.get_a();
    self.registers.set_a(!a);
    let flags = self.registers.get_f();
    self.set_flags((flags & (FlagMasks::ZERO as u8) != 0), true, true, (flags & (FlagMasks::CARRY as u8) != 0));
  }

  pub fn execute(&mut self, instruction: Instruction) {
    match instruction {
      Instruction::ADD(target) => {
        arithmetic_op!(self, target, add);
      }

      Instruction::ADC(target) => {
        carry_op!(self, target, add);
      }

      Instruction::SUB(target) => {
        arithmetic_op!(self, target, sub);
      }

      Instruction::SBC(target) => {
        carry_op!(self, target, sub);
      }

      Instruction::AND(target) => {
        arithmetic_op!(self, target, and);
      }

      Instruction::OR(target) => {
        arithmetic_op!(self, target, or);
      }

      Instruction::XOR(target) => {
        arithmetic_op!(self, target, xor);
      }
      
      Instruction::CP(target) => {
        arithmetic_op!(self, target, cp);
      }

      Instruction::INC(target) => {
        self.inc(target);
      }

      Instruction::DEC(target) => {
        self.dec(target);
      }

      Instruction::INC_16(target1, target2) => {
        self.inc_16(target1, target2);
      }

      Instruction::DEC_16(target1, target2) => {
        self.dec_16(target1, target2);
      }

      Instruction::RL(carry) => {
        self.rl(carry);
      }

      Instruction::RR(carry) => {
        self.rr(carry);
      }

      Instruction::DAA => {
        self.daa();
      }

      Instruction::NOP => {
        self.nop();
      }

      Instruction::EI => {
        self.ei();
      }

      Instruction::DI => {
        self.di();
      }

      Instruction::PUSH(target, target2) => {
        self.push(target, target2);
      }

      Instruction::POP(target, target2) => {  
        self.pop(target, target2);
      }

      Instruction::RET(carry, zero, interrupt) => {
        self.ret(carry, zero, interrupt);
      }

      Instruction::RET_N(carry, zero) => {
        self.ret_n(carry, zero);
      }

      Instruction::RST(value) => {
        self.rst(value);
      }

      Instruction::CALL(address, carry, zero, negative) => {
        self.call(address, carry, zero, negative);
      }

      Instruction::JR(carry, zero, negative, jump_value) => {
        self.jr(carry, zero, negative, jump_value);
      }

      Instruction::JP(carry, zero, negative, jump_value) => {
        self.jp(carry, zero, negative, jump_value);
      }

      Instruction::JP_HL() => {
        self.jp_hl();
      }

      Instruction::LD_RR(target, source) => {
        self.ld_rr(target, source);
      }

      Instruction::LD_MEM_REG(source) => {
        arithmetic_op!(self, source, ld_mem_imm);
      }

      Instruction::LD_REG_MEM(target) => {
        self.ld_reg_mem(target);
      }

      Instruction::LD_MEM_IMM(value) => {
        self.ld_mem_imm(value);
      }

      Instruction::LD_REG_IMM(target, value) => {
        self.ld_reg_imm(target, value);
      }

      Instruction::LD_MEM_INC(increment, load) => {
        self.ld_mem_inc(increment, load);
      }

      Instruction::LD_BCDE(target1, target2, load) => {
        self.ld_bcde(target1, target2, load);
      }

      Instruction::LD_IMM_16(address, load) => {
        self.ld_imm_16(address, load);
      }

      Instruction::LD_IMM_8(value, load) => {
        self.ld_imm_8(value, load);
      }

      Instruction::LD_AC(load) => {
        self.ld_ac(load);
      }

      Instruction::ADD_IMM(value) => {
        self.add(value);
      }

      Instruction::ADC_IMM(value) => {
        self.add(value + self.registers.get_f() & (FlagMasks::CARRY as u8));
      } 

      Instruction::SUB_IMM(value) => {
        self.sub(value);
      }

      Instruction::SBC_IMM(value) => {
        self.sub(value + self.registers.get_f() & (FlagMasks::CARRY as u8));
      }

      Instruction::AND_IMM(value) => {
        self.and(value);
      }

      Instruction::OR_IMM(value) => {
        self.or(value);
      }

      Instruction::XOR_IMM(value) => {
        self.xor(value);
      }

      Instruction::CP_IMM(value) => {
        self.cp(value);
      }

      Instruction::ADD_MEM => {
        self.add(self.ram.read(self.registers.get_hl()));
      }

      Instruction::SUB_MEM => {
        self.sub(self.ram.read(self.registers.get_hl()));
      }

      Instruction::ADC_MEM => {
        self.add(self.ram.read(self.registers.get_hl()) + self.registers.get_f() & (FlagMasks::CARRY as u8));
      }

      Instruction::SBC_MEM => {
        self.sub(self.ram.read(self.registers.get_hl()) + self.registers.get_f() & (FlagMasks::CARRY as u8));
      }

      Instruction::AND_MEM => {
        self.and(self.ram.read(self.registers.get_hl()));
      }

      Instruction::OR_MEM => {
        self.or(self.ram.read(self.registers.get_hl()));
      }

      Instruction::XOR_MEM => {
        self.xor(self.ram.read(self.registers.get_hl()));
      }

      Instruction::CP_MEM => {
        self.cp(self.ram.read(self.registers.get_hl()));
      }

      Instruction::LD_REG_IMM_16(target1, target2, value) => {
        self.ld_reg_imm_16(target1, target2, value);
      }

      Instruction::ADD_HL(target1, target2) => {
        self.add_hl(target1, target2);
      }

      Instruction::STORE_SP(address) => {
        self.store_sp(address);
      }

      Instruction::INC_SP(value) => {
        self.inc_sp(value);
      }

      Instruction::LD_SP_HL => {
        self.ld_sp_hl();
      }

      Instruction::LD_HL_SP(value) => {
        self.ld_hl_sp(value);
      }

      Instruction::MOD_MEM(increment) => {
        self.mod_mem(increment);
      }

      Instruction::SCF => {
        self.scf();
      }

      Instruction::CCF => {
        self.ccf();
      }

      Instruction::CPL => {
        self.cpl();
      }
      
    }
  }
  
  fn decode_instruction(&self, opcode: u8) -> Instruction {
    let pc = self.registers.get_pc();
    let immediate1 = self.ram.read(pc + 1);
    let immediate2 = self.ram.read(pc + 2);
    let immediate_16 = (immediate2 as u16) << 8 | immediate1 as u16;

    let instruction = match opcode {
      0x00 => Instruction::NOP, // NOP
      0x01 => Instruction::LD_REG_IMM_16(ArithmeticTarget::B, ArithmeticTarget::C, immediate_16), // LD BC, d16
      0x02 => Instruction::LD_BCDE(ArithmeticTarget::B, ArithmeticTarget::C, false), // LD (BC), A
      0x03 => Instruction::INC_16(ArithmeticTarget::B, ArithmeticTarget::C), // INC BC
      0x04 => Instruction::INC(ArithmeticTarget::B), // INC B
      0x05 => Instruction::DEC(ArithmeticTarget::B), // DEC B
      0x06 => Instruction::LD_REG_IMM(ArithmeticTarget::B, immediate1), // LD B, d8
      0x07 => Instruction::RL(true), // RLCA
      0x08 => Instruction::STORE_SP(immediate_16), // LD (a16), SP
      0x09 => Instruction::ADD_HL(ArithmeticTarget::B, ArithmeticTarget::C), // ADD HL, BC
      0x0A => Instruction::LD_BCDE(ArithmeticTarget::B, ArithmeticTarget::C, true), // LD A, (BC)
      0x0B => Instruction::DEC_16(ArithmeticTarget::B, ArithmeticTarget::C), // DEC BC
      0x0C => Instruction::INC(ArithmeticTarget::C), // INC C
      0x0D => Instruction::DEC(ArithmeticTarget::C), // DEC C
      0x0E => Instruction::LD_REG_IMM(ArithmeticTarget::C, immediate1), // LD C, d8
      0x0F => Instruction::RR(true), // RRCA
      
      // 0x10 => Instruction::STOP, // STOP // Not implemented
      0x10 => panic!("STOP not implemented!"),
      0x11 => Instruction::LD_REG_IMM_16(ArithmeticTarget::D, ArithmeticTarget::E, immediate_16), // LD DE, d16
      0x12 => Instruction::LD_BCDE(ArithmeticTarget::D, ArithmeticTarget::E, false), // LD (DE), A
      0x13 => Instruction::INC_16(ArithmeticTarget::D, ArithmeticTarget::E), // INC DE
      0x14 => Instruction::INC(ArithmeticTarget::D), // INC D
      0x15 => Instruction::DEC(ArithmeticTarget::D), // DEC D
      0x16 => Instruction::LD_REG_IMM(ArithmeticTarget::D, immediate1), // LD D, d8
      0x17 => Instruction::RL(false), // RLA
      0x18 => Instruction::JR(false, false, false, immediate1 as i8), // JR e8
      0x19 => Instruction::ADD_HL(ArithmeticTarget::D, ArithmeticTarget::E), // ADD HL, DE
      0x1A => Instruction::LD_BCDE(ArithmeticTarget::D, ArithmeticTarget::E, true), // LD A, (DE)
      0x1B => Instruction::DEC_16(ArithmeticTarget::D, ArithmeticTarget::E), // DEC DE
      0x1C => Instruction::INC(ArithmeticTarget::E), // INC E
      0x1D => Instruction::DEC(ArithmeticTarget::E), // DEC E
      0x1E => Instruction::LD_REG_IMM(ArithmeticTarget::E, immediate1), // LD E, d8
      0x1F => Instruction::RR(false), // RRA

      0x20 => Instruction::JR(false, true, true, immediate1 as i8), // JR NZ, e8
      0x21 => Instruction::LD_REG_IMM_16(ArithmeticTarget::H, ArithmeticTarget::L, immediate_16), // LD HL, d16
      0x22 => Instruction::LD_MEM_INC(true, false), // LD (HL+), A
      0x23 => Instruction::INC_16(ArithmeticTarget::H, ArithmeticTarget::L), // INC HL
      0x24 => Instruction::INC(ArithmeticTarget::H), // INC H
      0x25 => Instruction::DEC(ArithmeticTarget::H), // DEC H
      0x26 => Instruction::LD_REG_IMM(ArithmeticTarget::H, immediate1), // LD H, d8
      0x27 => Instruction::DAA, // DAA
      0x28 => Instruction::JR(false, true, false, immediate1 as i8), // JR Z, s8
      0x29 => Instruction::ADD_HL(ArithmeticTarget::H, ArithmeticTarget::L), // ADD HL, HL
      0x2A => Instruction::LD_MEM_INC(true, true), // LD A, (HL+)
      0x2B => Instruction::DEC_16(ArithmeticTarget::H, ArithmeticTarget::L), // DEC HL
      0x2C => Instruction::INC(ArithmeticTarget::L), // INC L
      0x2D => Instruction::DEC(ArithmeticTarget::L), // DEC L
      0x2E => Instruction::LD_REG_IMM(ArithmeticTarget::L, immediate1), // LD L, d8
      0x2F => Instruction::CPL, // CPL

      0x30 => Instruction::JR(true, false, true, immediate1 as i8), // JR NC, e8
      0x31 => Instruction::LD_REG_IMM_16(ArithmeticTarget::SP, ArithmeticTarget::SP, immediate_16), // LD SP, d16
      0x32 => Instruction::LD_MEM_INC(false, false), // LD (HL-), A
      0x33 => Instruction::INC_16(ArithmeticTarget::SP, ArithmeticTarget::SP), // INC SP
      0x34 => Instruction::MOD_MEM(false), // INC (HL)
      0x35 => Instruction::MOD_MEM(true), // DEC (HL)
      0x36 => Instruction::LD_MEM_IMM(immediate1), // LD (HL), d8
      0x37 => Instruction::SCF, // SCF
      0x38 => Instruction::JR(true, false, false, immediate1 as i8), // JR C, e8
      0x39 => Instruction::ADD_HL(ArithmeticTarget::SP, ArithmeticTarget::SP), // ADD HL, SP
      0x3A => Instruction::LD_MEM_INC(false, true), // LD A, (HL-)
      0x3B => Instruction::DEC_16(ArithmeticTarget::SP, ArithmeticTarget::SP), // DEC SP
      0x3C => Instruction::INC(ArithmeticTarget::A), // INC A
      0x3D => Instruction::DEC(ArithmeticTarget::A), // DEC A
      0x3E => Instruction::LD_REG_IMM(ArithmeticTarget::A, immediate1), // LD A, d8
      0x3F => Instruction::CCF, // CCF


      0x40 => Instruction::LD_RR(ArithmeticTarget::B, ArithmeticTarget::B), // LD B, B
      0x41 => Instruction::LD_RR(ArithmeticTarget::B, ArithmeticTarget::C), // LD B, C
      0x42 => Instruction::LD_RR(ArithmeticTarget::B, ArithmeticTarget::D), // LD B, D
      0x43 => Instruction::LD_RR(ArithmeticTarget::B, ArithmeticTarget::E), // LD B, E
      0x44 => Instruction::LD_RR(ArithmeticTarget::B, ArithmeticTarget::H), // LD B, H
      0x45 => Instruction::LD_RR(ArithmeticTarget::B, ArithmeticTarget::L), // LD B, L
      0x46 => Instruction::LD_REG_MEM(ArithmeticTarget::B), // LD B, (HL)
      0x47 => Instruction::LD_RR(ArithmeticTarget::B, ArithmeticTarget::A), // LD B, A
      0x48 => Instruction::LD_RR(ArithmeticTarget::C, ArithmeticTarget::B), // LD C, B
      0x49 => Instruction::LD_RR(ArithmeticTarget::C, ArithmeticTarget::C), // LD C, C
      0x4A => Instruction::LD_RR(ArithmeticTarget::C, ArithmeticTarget::D), // LD C, D
      0x4B => Instruction::LD_RR(ArithmeticTarget::C, ArithmeticTarget::E), // LD C, E
      0x4C => Instruction::LD_RR(ArithmeticTarget::C, ArithmeticTarget::H), // LD C, H  
      0x4D => Instruction::LD_RR(ArithmeticTarget::C, ArithmeticTarget::L), // LD C, L
      0x4E => Instruction::LD_REG_MEM(ArithmeticTarget::C), // LD C, (HL)
      0x4F => Instruction::LD_RR(ArithmeticTarget::C, ArithmeticTarget::A), // LD C, A
     
      0x50 => Instruction::LD_RR(ArithmeticTarget::D, ArithmeticTarget::B), // LD D, B
      0x51 => Instruction::LD_RR(ArithmeticTarget::D, ArithmeticTarget::C), // LD D, C
      0x52 => Instruction::LD_RR(ArithmeticTarget::D, ArithmeticTarget::D), // LD D, D
      0x53 => Instruction::LD_RR(ArithmeticTarget::D, ArithmeticTarget::E), // LD D, E
      0x54 => Instruction::LD_RR(ArithmeticTarget::D, ArithmeticTarget::H), // LD D, H
      0x55 => Instruction::LD_RR(ArithmeticTarget::D, ArithmeticTarget::L), // LD D, L
      0x56 => Instruction::LD_REG_MEM(ArithmeticTarget::D), // LD D, (HL)
      0x57 => Instruction::LD_RR(ArithmeticTarget::D, ArithmeticTarget::A), // LD D, A
      0x58 => Instruction::LD_RR(ArithmeticTarget::E, ArithmeticTarget::B), // LD E, B
      0x59 => Instruction::LD_RR(ArithmeticTarget::E, ArithmeticTarget::C), // LD E, C
      0x5A => Instruction::LD_RR(ArithmeticTarget::E, ArithmeticTarget::D), // LD E, D
      0x5B => Instruction::LD_RR(ArithmeticTarget::E, ArithmeticTarget::E), // LD E, E
      0x5C => Instruction::LD_RR(ArithmeticTarget::E, ArithmeticTarget::H), // LD E, H
      0x5D => Instruction::LD_RR(ArithmeticTarget::E, ArithmeticTarget::L), // LD E, L
      0x5E => Instruction::LD_REG_MEM(ArithmeticTarget::E), // LD E, (HL)
      0x5F => Instruction::LD_RR(ArithmeticTarget::E, ArithmeticTarget::A), // LD E, A  
     
      0x60 => Instruction::LD_RR(ArithmeticTarget::H, ArithmeticTarget::B), // LD H, B  
      0x61 => Instruction::LD_RR(ArithmeticTarget::H, ArithmeticTarget::C), // LD H, C
      0x62 => Instruction::LD_RR(ArithmeticTarget::H, ArithmeticTarget::D), // LD H, D
      0x63 => Instruction::LD_RR(ArithmeticTarget::H, ArithmeticTarget::E), // LD H, E
      0x64 => Instruction::LD_RR(ArithmeticTarget::H, ArithmeticTarget::H), // LD H, H
      0x65 => Instruction::LD_RR(ArithmeticTarget::H, ArithmeticTarget::L), // LD H, L
      0x66 => Instruction::LD_REG_MEM(ArithmeticTarget::H), // LD H, (HL)
      0x67 => Instruction::LD_RR(ArithmeticTarget::H, ArithmeticTarget::A), // LD H, A
      0x68 => Instruction::LD_RR(ArithmeticTarget::L, ArithmeticTarget::B), // LD L, B
      0x69 => Instruction::LD_RR(ArithmeticTarget::L, ArithmeticTarget::C), // LD L, C
      0x6A => Instruction::LD_RR(ArithmeticTarget::L, ArithmeticTarget::D), // LD L, D
      0x6B => Instruction::LD_RR(ArithmeticTarget::L, ArithmeticTarget::E), // LD L, E
      0x6C => Instruction::LD_RR(ArithmeticTarget::L, ArithmeticTarget::H), // LD L, H
      0x6D => Instruction::LD_RR(ArithmeticTarget::L, ArithmeticTarget::L), // LD L, L
      0x6E => Instruction::LD_REG_MEM(ArithmeticTarget::L), // LD L, (HL)
      0x6F => Instruction::LD_RR(ArithmeticTarget::L, ArithmeticTarget::A), // LD L, A
     
      0x70 => Instruction::LD_MEM_REG(ArithmeticTarget::B), // LD (HL), B
      0x71 => Instruction::LD_MEM_REG(ArithmeticTarget::C), // LD (HL), C
      0x72 => Instruction::LD_MEM_REG(ArithmeticTarget::D), // LD (HL), D
      0x73 => Instruction::LD_MEM_REG(ArithmeticTarget::E), // LD (HL), E
      0x74 => Instruction::LD_MEM_REG(ArithmeticTarget::H), // LD (HL), H
      0x75 => Instruction::LD_MEM_REG(ArithmeticTarget::L), // LD (HL), L
      // 0x76 => Instruction::HALT, // HALT // Not implemented
      0x76 => panic!("HALT not implemented!"),
      0x77 => Instruction::LD_MEM_REG(ArithmeticTarget::A), // LD (HL), A
      0x78 => Instruction::LD_RR(ArithmeticTarget::A, ArithmeticTarget::B), // LD A, B
      0x79 => Instruction::LD_RR(ArithmeticTarget::A, ArithmeticTarget::C), // LD A, C
      0x7A => Instruction::LD_RR(ArithmeticTarget::A, ArithmeticTarget::D), // LD A, D
      0x7B => Instruction::LD_RR(ArithmeticTarget::A, ArithmeticTarget::E), // LD A, E
      0x7C => Instruction::LD_RR(ArithmeticTarget::A, ArithmeticTarget::H), // LD A, H
      0x7D => Instruction::LD_RR(ArithmeticTarget::A, ArithmeticTarget::L), // LD A, L
      0x7E => Instruction::LD_REG_MEM(ArithmeticTarget::A), // LD A, (HL)
      0x7F => Instruction::LD_RR(ArithmeticTarget::A, ArithmeticTarget::A), // LD A, A
     
      0x80 => Instruction::ADD(ArithmeticTarget::B), // ADD A, B
      0x81 => Instruction::ADD(ArithmeticTarget::C), // ADD A, C
      0x82 => Instruction::ADD(ArithmeticTarget::D), // ADD A, D
      0x83 => Instruction::ADD(ArithmeticTarget::E), // ADD A, E
      0x84 => Instruction::ADD(ArithmeticTarget::H), // ADD A, H
      0x85 => Instruction::ADD(ArithmeticTarget::L), // ADD A, L
      0x86 => Instruction::ADD_MEM, // ADD A, (HL) 
      0x87 => Instruction::ADD(ArithmeticTarget::A), // ADD A, A
      0x88 => Instruction::ADC(ArithmeticTarget::B), // ADC A, B
      0x89 => Instruction::ADC(ArithmeticTarget::C), // ADC A, C
      0x8A => Instruction::ADC(ArithmeticTarget::D), // ADC A, D
      0x8B => Instruction::ADC(ArithmeticTarget::E), // ADC A, E
      0x8C => Instruction::ADC(ArithmeticTarget::H), // ADC A, H
      0x8D => Instruction::ADC(ArithmeticTarget::L), // ADC A, L
      0x8E => Instruction::ADC_MEM, // ADC A, (HL)
      0x8F => Instruction::ADC(ArithmeticTarget::A), // ADC A, A 
    
      0x90 => Instruction::SUB(ArithmeticTarget::B), // SUB A, B
      0x91 => Instruction::SUB(ArithmeticTarget::C), // SUB A, C
      0x92 => Instruction::SUB(ArithmeticTarget::D), // SUB A, D
      0x93 => Instruction::SUB(ArithmeticTarget::E), // SUB A, E
      0x94 => Instruction::SUB(ArithmeticTarget::H), // SUB A, H
      0x95 => Instruction::SUB(ArithmeticTarget::L), // SUB A, L
      0x96 => Instruction::SUB_MEM, // SUB A, (HL)
      0x97 => Instruction::SUB(ArithmeticTarget::A), // SUB A, A
      0x98 => Instruction::SBC(ArithmeticTarget::B), // SBC A, B
      0x99 => Instruction::SBC(ArithmeticTarget::C), // SBC A, C
      0x9A => Instruction::SBC(ArithmeticTarget::D), // SBC A, D
      0x9B => Instruction::SBC(ArithmeticTarget::E), // SBC A, E
      0x9C => Instruction::SBC(ArithmeticTarget::H), // SBC A, H
      0x9D => Instruction::SBC(ArithmeticTarget::L), // SBC A, L
      0x9E => Instruction::SBC_MEM, // SBC A, (HL)
      0x9F => Instruction::SBC(ArithmeticTarget::A), // SBC A, A
    
      0xA0 => Instruction::AND(ArithmeticTarget::B), // AND A, B
      0xA1 => Instruction::AND(ArithmeticTarget::C), // AND A, C
      0xA2 => Instruction::AND(ArithmeticTarget::D), // AND A, D
      0xA3 => Instruction::AND(ArithmeticTarget::E), // AND A, E
      0xA4 => Instruction::AND(ArithmeticTarget::H), // AND A, H
      0xA5 => Instruction::AND(ArithmeticTarget::L), // AND A, L
      0xA6 => Instruction::AND_MEM, // AND A, (HL)
      0xA7 => Instruction::AND(ArithmeticTarget::A), // AND A, A
      0xA8 => Instruction::XOR(ArithmeticTarget::B), // XOR A, B
      0xA9 => Instruction::XOR(ArithmeticTarget::C), // XOR A, C
      0xAA => Instruction::XOR(ArithmeticTarget::D), // XOR A, D
      0xAB => Instruction::XOR(ArithmeticTarget::E), // XOR A, E
      0xAC => Instruction::XOR(ArithmeticTarget::H), // XOR A, H
      0xAD => Instruction::XOR(ArithmeticTarget::L), // XOR A, L
      0xAE => Instruction::XOR_MEM, // XOR A, (HL)
      0xAF => Instruction::XOR(ArithmeticTarget::A), // XOR A, A

      0xB0 => Instruction::OR(ArithmeticTarget::B), // OR A, B
      0xB1 => Instruction::OR(ArithmeticTarget::C), // OR A, C
      0xB2 => Instruction::OR(ArithmeticTarget::D), // OR A, D
      0xB3 => Instruction::OR(ArithmeticTarget::E), // OR A, E
      0xB4 => Instruction::OR(ArithmeticTarget::H), // OR A, H
      0xB5 => Instruction::OR(ArithmeticTarget::L), // OR A, L
      0xB6 => Instruction::OR_MEM, // OR A, (HL)
      0xB7 => Instruction::OR(ArithmeticTarget::A), // OR A, A
      0xB8 => Instruction::CP(ArithmeticTarget::B), // CP A, B
      0xB9 => Instruction::CP(ArithmeticTarget::C), // CP A, C
      0xBA => Instruction::CP(ArithmeticTarget::D), // CP A, D
      0xBB => Instruction::CP(ArithmeticTarget::E), // CP A, E
      0xBC => Instruction::CP(ArithmeticTarget::H), // CP A, H
      0xBD => Instruction::CP(ArithmeticTarget::L), // CP A, L
      0xBE => Instruction::CP_MEM, // CP A, (HL)
      0xBF => Instruction::CP(ArithmeticTarget::A), // CP A, A
    
      0xC0 => Instruction::RET_N(false, true), // RET NZ
      0xC1 => Instruction::POP(ArithmeticTarget::B, ArithmeticTarget::C), // POP BC
      0xC2 => Instruction::JP(false, true, true, immediate_16), // JP NZ, a16
      0xC3 => Instruction::JP(false, false, false, immediate_16), // JP a16
      0xC4 => Instruction::CALL(immediate_16, false, true, true), // CALL NZ, a16
      0xC5 => Instruction::PUSH(ArithmeticTarget::B, ArithmeticTarget::C), // PUSH BC
      0xC6 => Instruction::ADD_IMM(immediate1), // ADD A, d8
      0xC7 => Instruction::RST(0x00), // RST 00H
      0xC8 => Instruction::RET(false, true, false), // RET Z
      0xC9 => Instruction::RET(false, false, false), // RET
      0xCA => Instruction::JP(false, true, false, immediate_16), // JP Z, a16
      // 0xCB => Instruction::PREFIX_CB, // PREFIX CB // Not implemented
      0xCB => panic!("PREFIX CB not implemented!"),
      0xCC => Instruction::CALL(immediate_16, false, true, false ), // CALL Z, a16
      0xCD => Instruction::CALL(immediate_16, false, false, false), // CALL a16
      0xCE => Instruction::ADC_IMM(immediate1), // ADC A, d8
      0xCF => Instruction::RST(0x08), // RST 08H
    
      0xD0 => Instruction::RET_N(true, false), // RET NC, e8
      0xD1 => Instruction::POP(ArithmeticTarget::D, ArithmeticTarget::E), // POP DE
      0xD2 => Instruction::JP(true, false, true, immediate_16), // JP NC, a16
      0xD3 => panic!("OUT (C), A not implemented"),// OUT (C), A
      0xD4 => Instruction::CALL(immediate_16, true, false, true), // CALL NC, a16
      0xD5 => Instruction::PUSH(ArithmeticTarget::D, ArithmeticTarget::E), // PUSH DE
      0xD6 => Instruction::SUB_IMM(immediate1), // SUB A, d8
      0xD7 => Instruction::RST(0x10), // RST 10H
      0xD8 => Instruction::RET(true, false, false), // RET C, e8
      0xD9 => Instruction::RET(false, false, true), // RETI
      0xDA => Instruction::JP(true, false, false, immediate_16), // JP C, a16
      0xDB => panic!("IN A, (C) not implemented"),// IN A, (C)
      0xDC => Instruction::CALL(immediate_16, true, false, false), // CALL C, a16
      0xDD => panic!("PREFIX DD not implemented"), // PREFIX DD
      0xDE => Instruction::SBC_IMM(immediate1), // SBC A, d8
      0xDF => Instruction::RST(0x18), // RST 18H
    
      0xE0 => Instruction::LD_IMM_8(immediate1, false), // LD (FF00+d8), A
      0xE1 => Instruction::POP(ArithmeticTarget::B, ArithmeticTarget::C), // POP HL
      0xE2 => Instruction::LD_AC(false), // LD (FF00+C), A
      0xE3 => panic!("EX (SP), HL not implemented"), // EX (SP), HL
      0xE4 => panic!("CALL HL, a16 not implemented"), // CALL HL, a16
      0xE5 => Instruction::PUSH(ArithmeticTarget::B, ArithmeticTarget::C), // PUSH HL
      0xE6 => Instruction::AND_IMM(immediate1), // AND A, d8
      0xE7 => Instruction::RST(0x20), // RST 20H
      0xE8 => Instruction::INC_SP(immediate1 as i8), // ADD SP, r8
      0xE9 => Instruction::JP_HL(), // JP (HL)
      0xEA => Instruction::LD_IMM_16(immediate_16, false), // LD (a16), A
      0xEB => panic!("EX DE, HL not implemented"), // EX DE, HL
      0xEC => panic!("CALL HL, a16 not implemented"), // CALL HL, a16
      0xED => panic!("PREFIX ED not implemented"), // PREFIX ED
      0xEE => Instruction::XOR_IMM(immediate1), // XOR A, d8
      0xEF => Instruction::RST(0x28), // RST 28H
    
      0xF0 => Instruction::LD_IMM_8(immediate1, true), // LD A, (FF00+C)
      0xF1 => Instruction::POP(ArithmeticTarget::A, ArithmeticTarget::B), // POP AF
      0xF2 => Instruction::LD_AC(true), // LD A, (FF00+C)
      0xF3 => Instruction::DI, // DI
      0xF4 => panic!("CALL HL, a16 not implemented"), // CALL HL, a16
      0xF5 => Instruction::PUSH(ArithmeticTarget::A, ArithmeticTarget::B), // PUSH AF
      0xF6 => Instruction::OR_IMM(immediate1), // OR A, d8
      0xF7 => Instruction::RST(0x30), // RST 30H
      0xF8 => Instruction::LD_HL_SP(immediate1 as i8), // LD HL, SP+r8 // did i not do this???
      0xF9 => Instruction::LD_SP_HL, // LD SP, HL
      0xFA => Instruction::LD_IMM_16(immediate_16, false), // LD A, a16
      0xFB => Instruction::EI, // EI
      0xFC => panic!("CALL HL, a16 not implemented"), // CALL HL, a16
      0xFD => panic!("PREFIX FD not implemented"), // PREFIX FD
      0xFE => Instruction::CP_IMM(immediate1), // CP A, d8
      0xFF => Instruction::RST(0x38), // RST 38H
    }; 
    
    instruction
  }

  pub fn step(&mut self) -> u8 {
    // Read opcode at current PC
    let opcode = self.ram.read(self.registers.get_pc());
    
    // Get instruction size and cycles before executing
    let (size, cycles) = self.get_instruction_info(opcode);
    // Store original PC to check if it was modified
    let original_pc = self.registers.get_pc();
    
    // Decode and execute the instruction
    let instruction = self.decode_instruction(opcode);
    self.execute(instruction);
    
    // Only update PC if it wasn't modified by the instruction
    if self.registers.get_pc() == original_pc {
      self.registers.set_pc(original_pc + size as u16);
    }
    
    // Update total clock cycles
    self.clock_cycles += cycles as u64;
    
    // Return number of cycles for this instruction
    cycles
  }

  fn get_instruction_info(&self, opcode: u8) -> (u8, u8) {
    // Default size is 1 byte for opcode
    let mut size = 1;
    let mut cycles = 4; // Base cycles for most instructions

    match opcode {
      // 2-byte instructions (opcode + 1 byte immediate)
      0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E | // LD r, d8
      0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE | // ALU operations with immediate
      0x18 | 0x20 | 0x28 | 0x30 | 0x38 => { // JR instructions
        size = 2;
        cycles = 8;
      }

      // 3-byte instructions (opcode + 2 bytes immediate)
      0x01 | 0x11 | 0x21 | 0x31 | // LD rr, d16
      0xC2 | 0xC3 | 0xC4 | 0xCA | 0xCC | 0xCD | // JP/CALL instructions
      0xD2 | 0xD4 | 0xDA | 0xDC | // JP/CALL instructions
      0xE2 | 0xEA | 0xF2 | 0xFA => { // LD instructions with 16-bit address
        size = 3;
        cycles = 12;
      }

      // Special cases for conditional instructions
      0x20 | 0x28 | 0x30 | 0x38 => { // JR cc, e8
        cycles = 8; // Not taken
        // TODO: Add 4 more cycles if condition is met
      }
      0xC0 | 0xC8 | 0xD0 | 0xD8 => { // RET cc
        cycles = 8; // Not taken
        // TODO: Add 12 more cycles if condition is met
      }
      0xC2 | 0xC4 | 0xCA | 0xCC | 0xD2 | 0xD4 | 0xDA | 0xDC => { // JP/CALL cc, a16
        cycles = 12; // Not taken
        // TODO: Add 4 more cycles if condition is met
      }

      // Special cases for other instructions
      0x08 => { // LD (a16), SP
        size = 3;
        cycles = 20;
      }
      0xE8 => { // ADD SP, r8
        size = 2;
        cycles = 16;
      }
      0xF8 => { // LD HL, SP+r8
        size = 2;
        cycles = 12;
      }
      0xF9 => { // LD SP, HL
        cycles = 8;
      }
      0x00 => { // NOP
        cycles = 4;
      }
      0x10 => { // STOP
        cycles = 4;
      }
      0x76 => { // HALT
        cycles = 4;
      }
      0xF3 | 0xFB => { // DI/EI
        cycles = 4;
      }

      // Default case - most instructions are 1 byte and take 4 cycles
      _ => {
        size = 1;
        cycles = 4;
      }
    }

    (size, cycles)
  }

  fn get_interrupt_vector(&self, interrupt_flag: u8) -> Interrupt {
    if interrupt_flag & 0x01 != 0 {
      Interrupt::VBLANK
    } else if interrupt_flag & 0x02 != 0 {
      Interrupt::LCD_STAT
    } else if interrupt_flag & 0x04 != 0 {
      Interrupt::TIMER
    } else if interrupt_flag & 0x08 != 0 {
      Interrupt::SERIAL
    } else {
      Interrupt::JOYPAD
    }
  }

  fn get_interrupt_handler(&self, interrupt_vector: &Interrupt) -> InterruptHandlers {
    match interrupt_vector {
      Interrupt::VBLANK => InterruptHandlers::VBLANK,
      Interrupt::LCD_STAT => InterruptHandlers::LCD_STAT,
      Interrupt::TIMER => InterruptHandlers::TIMER,
      Interrupt::SERIAL => InterruptHandlers::SERIAL,
      Interrupt::JOYPAD => InterruptHandlers::JOYPAD,
    }
  }

  pub fn handle_interrupts(&mut self) {
    /*
    TODO: this is probably the next thing to implement

    https://gbdev.io/pandocs/Interrupts.html

     */
    if self.interrupt_master_enable {
      assert!(self.ram.read(INTERRUPT_FLAG_ADDRESS) & 0x1F != 0);
      let interrupt_flag = self.ram.read(INTERRUPT_FLAG_ADDRESS);
      let interrupt_enable = self.ram.read(INTERRUPT_ENABLE_ADDRESS);
      if interrupt_flag & interrupt_enable != 0 {
        self.previous_ime = self.interrupt_master_enable;
        self.interrupt_master_enable = false;

        let mut sp = self.registers.get_sp();
        push_16bit!(self, &mut sp, get_pc);
        self.registers.set_sp(sp);

        let interrupt_vector = self.get_interrupt_vector(interrupt_flag);
        let interrupt_handler = self.get_interrupt_handler(&interrupt_vector);
        self.ram.write(INTERRUPT_FLAG_ADDRESS, interrupt_flag & !(interrupt_vector as u8));
        self.registers.set_pc(interrupt_handler as u16);
      }
    }
  }
}


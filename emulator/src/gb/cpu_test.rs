#[cfg(test)]
mod tests {
    use super::*;
    use crate::gb::cpu::{CPU, Instruction, ArithmeticTarget};
    use crate::gb::ram::RAM;
    use crate::gb::register::Flags;

    // Helper function to create a CPU with specific initial state
    fn create_cpu_with_state(
        a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, h: u8, l: u8,
        sp: u16, pc: u16, ram: &mut RAM
    ) -> CPU {
        let mut cpu = CPU::new(ram);
        cpu.registers.set_a(a);
        cpu.registers.set_b(b);
        cpu.registers.set_c(c);
        cpu.registers.set_d(d);
        cpu.registers.set_e(e);
        cpu.registers.set_f(f);
        cpu.registers.set_h(h);
        cpu.registers.set_l(l);
        cpu.registers.set_sp(sp);
        cpu.registers.set_pc(pc);
        cpu
    }

    // Helper function to check register values
    fn assert_registers(
        cpu: &CPU,
        a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, h: u8, l: u8,
        sp: u16, pc: u16
    ) {
        assert_eq!(cpu.registers.get_a(), a, "Register A mismatch");
        assert_eq!(cpu.registers.get_b(), b, "Register B mismatch");
        assert_eq!(cpu.registers.get_c(), c, "Register C mismatch");
        assert_eq!(cpu.registers.get_d(), d, "Register D mismatch");
        assert_eq!(cpu.registers.get_e(), e, "Register E mismatch");
        assert_eq!(cpu.registers.get_f(), f, "Register F mismatch");
        assert_eq!(cpu.registers.get_h(), h, "Register H mismatch");
        assert_eq!(cpu.registers.get_l(), l, "Register L mismatch");
        assert_eq!(cpu.registers.get_sp(), sp, "Register SP mismatch");
        assert_eq!(cpu.registers.get_pc(), pc, "Register PC mismatch");
    }

    // Helper function to check specific flags
    fn assert_flags(cpu: &CPU, zero: bool, subtract: bool, half_carry: bool, carry: bool) {
        let flags = Flags::from_u8(cpu.registers.get_f());
        assert_eq!(flags.zero, zero, "Zero flag mismatch");
        assert_eq!(flags.subtract, subtract, "Subtract flag mismatch");
        assert_eq!(flags.half_carry, half_carry, "Half carry flag mismatch");
        assert_eq!(flags.carry, carry, "Carry flag mismatch");
    }

    #[test]
    fn test_ld_rr() {
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0x11, 0x22, 0x33, 0x44, 0x55, 0x00, 0x77, 0x88, 0x1234, 0x5678, &mut ram);
        cpu.execute(Instruction::LD_RR(ArithmeticTarget::A, ArithmeticTarget::B));
        assert_registers(&cpu, 0x22, 0x22, 0x33, 0x44, 0x55, 0x00, 0x77, 0x88, 0x1234, 0x5678);
    }

    #[test]
    fn test_ld_reg_imm() {
        // Test LD A, 0x42
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, &mut ram);
        cpu.ram.write(0, 0x3E); // LD A, d8 opcode
        cpu.ram.write(1, 0x42); // Immediate value
        let cycles = cpu.step();
        assert_eq!(cycles, 8, "LD A, d8 should take 8 cycles");
        assert_registers(&cpu, 0x42, 0, 0, 0, 0, 0, 0, 0, 0, 2);
    }

    #[test]
    fn test_add() {
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0x11, 0x22, 0x33, 0x44, 0x55, 0x00, 0x77, 0x88, 0x1234, 0x5678, &mut ram);
        cpu.execute(Instruction::ADD(ArithmeticTarget::B));
        assert_registers(&cpu, 0x33, 0x22, 0x33, 0x44, 0x55, 0x00, 0x77, 0x88, 0x1234, 0x5678);
        eprintln!("cpu.registers.get_b(): {}", cpu.registers.get_b());
        // print the flags
        eprintln!("cpu.registers.get_f(): {:02x}", cpu.registers.get_f());
        assert_flags(&cpu, false, false, false, false);
    }

    #[test]
    fn test_add_with_carry() {
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0xFF, 0x22, 0x33, 0x44, 0x55, 0x00, 0x77, 0x88, 0x1234, 0x5678, &mut ram);
        cpu.execute(Instruction::ADD(ArithmeticTarget::C)); // 0xFF + 0x33
        eprintln!("cpu.registers.get_a(): {:02x}", cpu.registers.get_a());
        assert_registers(&cpu, 0x32, 0x22, 0x33, 0x44, 0x55, 0x30, 0x77, 0x88, 0x1234, 0x5678);
        // You may want to adjust the expected flags here based on your implementation
    }

    #[test]
    fn test_sub() {
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0x33, 0x22, 0x33, 0x44, 0x55, 0x00, 0x77, 0x88, 0x1234, 0x5678, &mut ram);
        cpu.execute(Instruction::SUB(ArithmeticTarget::B));
        assert_registers(&cpu, 0x11, 0x22, 0x33, 0x44, 0x55, 0x60, 0x77, 0x88, 0x1234, 0x5678);
        assert_flags(&cpu, false, true, true, false);
    }

    #[test]
    fn test_sub_with_borrow() {
        // Test SUB A, B with borrow
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0x00, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, &mut ram);
        cpu.ram.write(0, 0x90); // SUB A, B opcode
        let cycles = cpu.step();
        assert_eq!(cycles, 4, "SUB A, B should take 4 cycles");
        assert_registers(&cpu, 0xFF, 0x01, 0, 0, 0, 0x50, 0, 0, 0, 1);
        assert_flags(&cpu, false, true, false, true);
    }

    #[test]
    fn test_inc() {
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0x11, 0x22, 0x33, 0x44, 0x55, 0x00, 0x77, 0x88, 0x1234, 0x5678, &mut ram);
        cpu.execute(Instruction::INC(ArithmeticTarget::B));
        assert_registers(&cpu, 0x11, 0x23, 0x33, 0x44, 0x55, 0x00, 0x77, 0x88, 0x1234, 0x5678);
        assert_flags(&cpu, false, false, false, false);
    }

    #[test]
    fn test_dec() {
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0x11, 0x22, 0x33, 0x44, 0x55, 0x00, 0x77, 0x88, 0x1234, 0x5678, &mut ram);
        cpu.execute(Instruction::DEC(ArithmeticTarget::B));
        assert_registers(&cpu, 0x11, 0x21, 0x33, 0x44, 0x55, 0x40, 0x77, 0x88, 0x1234, 0x5678);
        assert_flags(&cpu, false, true, false, false);
    }

    #[test]
    fn test_jp() {
        // Test JP 0x1234
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, &mut ram);
        cpu.ram.write(0, 0xC3); // JP a16 opcode
        cpu.ram.write(1, 0x34); // Low byte of address
        cpu.ram.write(2, 0x12); // High byte of address
        let cycles = cpu.step();
        assert_eq!(cycles, 12, "JP a16 should take 12 cycles");
        assert_registers(&cpu, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x1234);
    }

    #[test]
    fn test_jr() {
        // Test JR 0x10 (forward jump)
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, &mut ram);
        cpu.ram.write(0, 0x18); // JR e8 opcode
        cpu.ram.write(1, 0x10); // Jump offset
        let cycles = cpu.step();
        assert_eq!(cycles, 8, "JR e8 should take 8 cycles");
        assert_registers(&cpu, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10);
    }

    #[test]
    fn test_push_pop() {
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0x11, 0x22, 0x33, 0x44, 0x55, 0x00, 0x77, 0x88, 0x1234, 0x5678, &mut ram);
        cpu.execute(Instruction::PUSH(ArithmeticTarget::B, ArithmeticTarget::C));
        assert_eq!(cpu.registers.get_sp(), 0x1232);
        cpu.execute(Instruction::POP(ArithmeticTarget::D, ArithmeticTarget::E));
        assert_registers(&cpu, 0x11, 0x22, 0x33, 0x22, 0x33, 0x00, 0x77, 0x88, 0x1234, 0x5678);
    }

    #[test]
    fn test_instruction_sequence() {
        // Test a sequence of instructions
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0, 0, 0, 0, 0, 0, 0, 0, 0xFFFE, 0, &mut ram);
        
        // Write instruction sequence
        cpu.ram.write(0, 0x3E); // LD A, d8
        cpu.ram.write(1, 0x42); // Value 0x42
        cpu.ram.write(2, 0x06); // LD B, d8
        cpu.ram.write(3, 0x10); // Value 0x10
        cpu.ram.write(4, 0x80); // ADD A, B
        cpu.ram.write(5, 0x04); // INC B
        cpu.ram.write(6, 0x90); // SUB A, B

        // Execute sequence
        let mut total_cycles = 0;
        for _ in 0..5 {
            total_cycles += cpu.step();
        }

        assert_eq!(total_cycles, 28, "Total cycles mismatch");
        assert_registers(&cpu, 0x41, 0x11, 0, 0, 0, 0x40, 0, 0, 0xFFFE, 7);
        assert_flags(&cpu, false, true, false, false);
    }

    #[test]
    fn test_conditional_jump() {
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x1234, 0x5678, &mut ram);
        // Set zero flag manually
        cpu.registers.set_f(Flags { zero: true, subtract: false, half_carry: false, carry: false }.to_u8());
        cpu.execute(Instruction::JP(false, true, false, 0xABCD));
        assert_eq!(cpu.registers.get_pc(), 0xABCD);
    }

    #[test]
    fn test_16bit_operations() {
        let mut ram = RAM::new();
        let mut cpu = create_cpu_with_state(0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x1234, 0x5678, &mut ram);
        cpu.execute(Instruction::INC_16(ArithmeticTarget::B, ArithmeticTarget::C));
        assert_eq!(cpu.registers.get_bc(), 0x2234);
    }
} 
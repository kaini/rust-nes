use cpu::cpu::{Cpu, Hardware, STACK_START};
use std::marker::PhantomData;
use std::io::Write;

trait AddrMode {
	fn decode(cpu: &mut Cpu, hw: &mut Hardware) -> Self;
	fn read(&self, cpu: &mut Cpu, hw: &mut Hardware) -> u8;
	fn write(&self, cpu: &mut Cpu, hw: &mut Hardware, value: u8);
	fn asm_str(cpu: &Cpu) -> String;
}

// Access A.
struct AddrAccumulator;
impl AddrMode for AddrAccumulator {
	fn decode(_: &mut Cpu, _: &mut Hardware) -> AddrAccumulator {
		AddrAccumulator
	}
	fn read(&self, cpu: &mut Cpu, _: &mut Hardware) -> u8 {
		cpu.registers().a
	}
	fn write(&self, cpu: &mut Cpu, _: &mut Hardware, value: u8) {
		cpu.registers_mut().a = value;
	}
	fn asm_str(_: &Cpu) -> String {
		String::from("A")
	}
}

// Access immediate from opcode.
struct AddrImmediate {
	value: u8,
}
impl AddrMode for AddrImmediate {
	fn decode(cpu: &mut Cpu, _: &mut Hardware) -> AddrImmediate {
		AddrImmediate { value: cpu.opcode8() }
	}
	fn read(&self, _: &mut Cpu, _: &mut Hardware) -> u8 {
		self.value
	}
	fn write(&self, _: &mut Cpu, _: &mut Hardware, _: u8) {
		unreachable!();
	}
	fn asm_str(cpu: &Cpu) -> String {
		format!("#${:02X}", cpu.opcode8())
	}
}

// Access at the immediate address.
struct AddrZeroPage {
	addr: u16,
}
impl AddrMode for AddrZeroPage {
	fn decode(cpu: &mut Cpu, _: &mut Hardware) -> AddrZeroPage {
		AddrZeroPage { addr: cpu.opcode8() as u16 }
	}
	fn read(&self, cpu: &mut Cpu, hw: &mut Hardware) -> u8 {
		cpu.read_memory(hw, self.addr)
	}
	fn write(&self, cpu: &mut Cpu, hw: &mut Hardware, value: u8) {
		cpu.write_memory(hw, self.addr, value);
	}
	fn asm_str(cpu: &Cpu) -> String {
		format!("${:02X}", cpu.opcode8())
	}
}

// Access at the immediate address + X (modulo).
struct AddrZeroPageX {
	addr: u16,
}
impl AddrMode for AddrZeroPageX {
	fn decode(cpu: &mut Cpu, _: &mut Hardware) -> AddrZeroPageX {
		AddrZeroPageX { addr: (cpu.opcode8().wrapping_add(cpu.registers().x)) as u16 }
	}
	fn read(&self, cpu: &mut Cpu, hw: &mut Hardware) -> u8 {
		cpu.read_memory(hw, self.addr)
	}
	fn write(&self, cpu: &mut Cpu, hw: &mut Hardware, value: u8) {
		cpu.write_memory(hw, self.addr, value);
	}
	fn asm_str(cpu: &Cpu) -> String {
		format!("${:02X},X", cpu.opcode8())
	}
}

// Access at the immediate address + Y (modulo).
struct AddrZeroPageY {
	addr: u16,
}
impl AddrMode for AddrZeroPageY {
	fn decode(cpu: &mut Cpu, _: &mut Hardware) -> AddrZeroPageY {
		AddrZeroPageY { addr: (cpu.opcode8().wrapping_add(cpu.registers().y)) as u16 }
	}
	fn read(&self, cpu: &mut Cpu, hw: &mut Hardware) -> u8 {
		cpu.read_memory(hw, self.addr)
	}
	fn write(&self, cpu: &mut Cpu, hw: &mut Hardware, value: u8) {
		cpu.write_memory(hw, self.addr, value);
	}
	fn asm_str(cpu: &Cpu) -> String {
		format!("${:02X},Y", cpu.opcode8())
	}
}

// Access absolute memory address.
struct AddrAbsolute {
	addr: u16,
}
impl AddrMode for AddrAbsolute {
	fn decode(cpu: &mut Cpu, _: &mut Hardware) -> AddrAbsolute {
		AddrAbsolute { addr: cpu.opcode16() }
	}
	fn read(&self, cpu: &mut Cpu, hw: &mut Hardware) -> u8 {
		cpu.read_memory(hw, self.addr)
	}
	fn write(&self, cpu: &mut Cpu, hw: &mut Hardware, value: u8) {
		cpu.write_memory(hw, self.addr, value);
	}
	fn asm_str(cpu: &Cpu) -> String {
		format!("${:04X}", cpu.opcode16())
	}
}

// Access absolute memory address + X.
struct AddrAbsoluteX {
	addr: u16,
}
impl AddrMode for AddrAbsoluteX {
	fn decode(cpu: &mut Cpu, _: &mut Hardware) -> AddrAbsoluteX {
		let offset = cpu.registers().x as u16;
		AddrAbsoluteX { addr: cpu.opcode16().wrapping_add(offset) }
	}
	fn read(&self, cpu: &mut Cpu, hw: &mut Hardware) -> u8 {
		cpu.read_memory(hw, self.addr)
	}
	fn write(&self, cpu: &mut Cpu, hw: &mut Hardware, value: u8) {
		cpu.write_memory(hw, self.addr, value);
	}
	fn asm_str(cpu: &Cpu) -> String {
		format!("${:04X},X", cpu.opcode16())
	}
}

// Access absolute memory address + Y.
struct AddrAbsoluteY {
	addr: u16,
}
impl AddrMode for AddrAbsoluteY {
	fn decode(cpu: &mut Cpu, _: &mut Hardware) -> AddrAbsoluteY {
		let offset = cpu.registers().y as u16;
		AddrAbsoluteY { addr: cpu.opcode16().wrapping_add(offset) }
	}
	fn read(&self, cpu: &mut Cpu, hw: &mut Hardware) -> u8 {
		cpu.read_memory(hw, self.addr)
	}
	fn write(&self, cpu: &mut Cpu, hw: &mut Hardware, value: u8) {
		cpu.write_memory(hw, self.addr, value);
	}
	fn asm_str(cpu: &Cpu) -> String {
		format!("${:04X},Y", cpu.opcode16())
	}
}

// Access memory address at given zero parge memory address + X (modulo).
struct AddrIndirectX {
	addr: u16,
}
impl AddrMode for AddrIndirectX {
	fn decode(cpu: &mut Cpu, hw: &mut Hardware) -> AddrIndirectX {
		let iaddr = cpu.opcode8().wrapping_add(cpu.registers().x);
		let addr_lo = cpu.read_memory(hw, iaddr as u16) as u16;
		let addr_hi = cpu.read_memory(hw, iaddr.wrapping_add(1) as u16) as u16;
		AddrIndirectX { addr: (addr_hi << 8) | addr_lo }
	}
	fn read(&self, cpu: &mut Cpu, hw: &mut Hardware) -> u8 {
		cpu.read_memory(hw, self.addr)
	}
	fn write(&self, cpu: &mut Cpu, hw: &mut Hardware, value: u8) {
		cpu.write_memory(hw, self.addr, value);
	}
	fn asm_str(cpu: &Cpu) -> String {
		format!("(${:02X},X)", cpu.opcode8())
	}
}

// Access memory address + Y at given zero parge memory address.
struct AddrIndirectY {
	addr: u16,
}
impl AddrMode for AddrIndirectY {
	fn decode(cpu: &mut Cpu, hw: &mut Hardware) -> AddrIndirectY {
		let iaddr = cpu.opcode8();
		let addr_lo = cpu.read_memory(hw, iaddr as u16) as u16;
		let addr_hi = cpu.read_memory(hw, iaddr.wrapping_add(1) as u16) as u16;
		let offset = cpu.registers().y as u16;
		AddrIndirectY { addr: ((addr_hi << 8) | addr_lo).wrapping_add(offset) }
	}
	fn read(&self, cpu: &mut Cpu, hw: &mut Hardware) -> u8 {
		cpu.read_memory(hw, self.addr)
	}
	fn write(&self, cpu: &mut Cpu, hw: &mut Hardware, value: u8) {
		cpu.write_memory(hw, self.addr, value);
	}
	fn asm_str(cpu: &Cpu) -> String {
		format!("(${:02X}),Y", cpu.opcode8())
	}
}

// Represents a single operation.
pub trait Instruction {
	// Execute the operation.
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware);
	// Print the instruction
	fn asm_str(&self, cpu: &Cpu) -> String;
}

// Add with carry.
struct OpADC<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpADC<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let a = cpu.registers().a as u16;
		let src = A::decode(cpu, hw).read(cpu, hw) as u16;
		let result = a + src + (cpu.registers().p.carry as u16);
		cpu.registers_mut().a = result as u8;
		cpu.registers_mut().p.carry = result > 0xFF;
		cpu.registers_mut().p.zero = result & 0xFF == 0;
		cpu.registers_mut().p.overflow = (a ^ src) & 0x80 == 0 && (a ^ result) & 0x80 != 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("ADC {}", A::asm_str(cpu))
	}
}

// AND and LSR A.
struct OpALR<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpALR<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		OpAND::<A>{ phantom: PhantomData }.execute(cpu, hw);
		OpLSR::<AddrAccumulator>{ phantom: PhantomData }.execute(cpu, hw);
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("ALR {}", A::asm_str(cpu))
	}
}

// AND, then copy flags N to C.
struct OpANC<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpANC<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		OpAND::<A>{ phantom: PhantomData }.execute(cpu, hw);
		cpu.registers_mut().p.carry = cpu.registers().p.negative;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("ANC {}", A::asm_str(cpu))
	}
}

// Logical and.
struct OpAND<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpAND<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let result = cpu.registers().a & A::decode(cpu, hw).read(cpu, hw);
		cpu.registers_mut().a = result;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("AND {}", A::asm_str(cpu))
	}
}

// AND and ROR A, with C to bit 6 and V bit 6 xor bit 5.
struct OpARR<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpARR<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let result = 
			((cpu.registers().a & A::decode(cpu, hw).read(cpu, hw)) >> 1) |
			if cpu.registers().p.carry { 0b10000000 } else { 0 };
		cpu.registers_mut().a = result;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.carry = result & 0b01000000 != 0;
		cpu.registers_mut().p.negative =
			(result & 0b01000000 != 0) != (result & 0b00100000 != 0);
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("ARR {}", A::asm_str(cpu))
	}
}

// Arithmetic shift left.
struct OpASL<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpASL<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let access = A::decode(cpu, hw);
		let src = access.read(cpu, hw);
		let result = src << 1;
		access.write(cpu, hw, result);
		cpu.registers_mut().p.carry = src & 0x80 != 0;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("ASL {}", A::asm_str(cpu))
	}
}

// X = (A & X) - src (without borrow)
struct OpAXS<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpAXS<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		cpu.registers_mut().a = cpu.registers().a & cpu.registers().x;
		cpu.registers_mut().p.carry = true;
		OpSBC::<A>{ phantom: PhantomData }.execute(cpu, hw);
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("AXS {}", A::asm_str(cpu))
	}
}

// Branch if carry clear.
struct OpBCC;
impl Instruction for OpBCC {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let offset = cpu.opcode8() as i8 as i16 as u16;
		if !cpu.registers().p.carry {
			cpu.registers_mut().pc = cpu.registers().pc.wrapping_add(offset);
		}
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("BCC #${:+03X}", cpu.opcode8() as i8)
	}
}

// Branch if carry set.
struct OpBCS;
impl Instruction for OpBCS {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let offset = cpu.opcode8() as i8 as i16 as u16;
		if cpu.registers().p.carry {
			cpu.registers_mut().pc = cpu.registers().pc.wrapping_add(offset);
		}
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("BCS #${:+03X}", cpu.opcode8() as i8)
	}
}

// Branch if equal.
struct OpBEQ;
impl Instruction for OpBEQ {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let offset = cpu.opcode8() as i8 as i16 as u16;
		if cpu.registers().p.zero {
			cpu.registers_mut().pc = cpu.registers().pc.wrapping_add(offset);
		}
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("BEQ #${:+03X}", cpu.opcode8() as i8)
	}
}

// Bit test.
struct OpBIT<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpBIT<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let src = A::decode(cpu, hw).read(cpu, hw);
		let result = cpu.registers().a & src;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.overflow = src & 0x40 != 0;
		cpu.registers_mut().p.negative = src & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("BIT {}", A::asm_str(cpu))
	}
}

// Branch if minus.
struct OpBMI;
impl Instruction for OpBMI {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let offset = cpu.opcode8() as i8 as i16 as u16;
		if cpu.registers().p.negative {
			cpu.registers_mut().pc = cpu.registers().pc.wrapping_add(offset);
		}
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("BMI #${:+03X}", cpu.opcode8() as i8)
	}
}

// Branch if not equal.
struct OpBNE;
impl Instruction for OpBNE {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let offset = cpu.opcode8() as i8 as i16 as u16;
		if !cpu.registers().p.zero {
			cpu.registers_mut().pc = cpu.registers().pc.wrapping_add(offset);
		}
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("BNE #${:+03X}", cpu.opcode8() as i8)
	}
}

// Branch if positive.
struct OpBPL;
impl Instruction for OpBPL {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let offset = cpu.opcode8() as i8 as i16 as u16;
		if !cpu.registers().p.negative {
			cpu.registers_mut().pc = cpu.registers().pc.wrapping_add(offset);
		}
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("BPL #${:+03X}", cpu.opcode8() as i8)
	}
}

// Force interrupt
struct OpBRK;
impl Instruction for OpBRK {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		cpu.jump_to_interrupt(hw, true);
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("BRK")
	}
}

// Branch if overflow clear.
struct OpBVC;
impl Instruction for OpBVC {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let offset = cpu.opcode8() as i8 as i16 as u16;
		if !cpu.registers().p.overflow {
			cpu.registers_mut().pc = cpu.registers().pc.wrapping_add(offset);
		}
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("BVC #${:+03X}", cpu.opcode8() as i8)
	}
}

// Branch if overflow set.
struct OpBVS;
impl Instruction for OpBVS {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let offset = cpu.opcode8() as i8 as i16 as u16;
		if cpu.registers().p.overflow {
			cpu.registers_mut().pc = cpu.registers().pc.wrapping_add(offset);
		}
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("BVS #${:+03X}", cpu.opcode8() as i8)
	}
}

// Clear carry flag.
struct OpCLC;
impl Instruction for OpCLC {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		cpu.registers_mut().p.carry = false;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("CLC")
	}
}

// Clear decimal mode.
struct OpCLD;
impl Instruction for OpCLD {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		cpu.registers_mut().p.decimal = false;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("CLD")
	}
}

// Clear interrupt disable.
struct OpCLI;
impl Instruction for OpCLI {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		cpu.registers_mut().p.interrupt = false;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("CLI")
	}
}

// Clear overflow flag.
struct OpCLV;
impl Instruction for OpCLV {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		cpu.registers_mut().p.overflow = false;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("CLV")
	}
}

// Compare.
struct OpCMP<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpCMP<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let src = A::decode(cpu, hw).read(cpu, hw);
		let result = cpu.registers().a.wrapping_add((!src).wrapping_add(1));
		cpu.registers_mut().p.carry = cpu.registers().a >= src;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("CMP {}", A::asm_str(cpu))
	}
}

// Compare X register.
struct OpCPX<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpCPX<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let src = A::decode(cpu, hw).read(cpu, hw);
		let result = cpu.registers().x.wrapping_add((!src).wrapping_add(1));
		cpu.registers_mut().p.carry = cpu.registers().x >= src;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("CPX {}", A::asm_str(cpu))
	}
}

// Compare Y register.
struct OpCPY<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpCPY<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let src = A::decode(cpu, hw).read(cpu, hw);
		let result = cpu.registers().y.wrapping_add((!src).wrapping_add(1));
		cpu.registers_mut().p.carry = cpu.registers().y >= src;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("CPY {}", A::asm_str(cpu))
	}
}

// DEC + CMP.
struct OpDCP<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpDCP<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		OpDEC::<A>{ phantom: PhantomData }.execute(cpu, hw);
		OpCMP::<A>{ phantom: PhantomData }.execute(cpu, hw);
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("DCP {}", A::asm_str(cpu))
	}
}

// Decrement memory.
struct OpDEC<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpDEC<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let access = A::decode(cpu, hw);
		let result = access.read(cpu, hw).wrapping_sub(1);
		access.write(cpu, hw, result);
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("DEC {}", A::asm_str(cpu))
	}
}

// Decrement X
struct OpDEX;
impl Instruction for OpDEX {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let result = cpu.registers().x.wrapping_sub(1);
		cpu.registers_mut().x = result;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("DEX")
	}
}

// Decrement Y
struct OpDEY;
impl Instruction for OpDEY {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let result = cpu.registers().y.wrapping_sub(1);
		cpu.registers_mut().y = result;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("DEY")
	}
}

// Logical exclusive or.
struct OpEOR<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpEOR<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let result = cpu.registers().a ^ A::decode(cpu, hw).read(cpu, hw);
		cpu.registers_mut().a = result;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("EOR {}", A::asm_str(cpu))
	}
}

// Increment memory.
struct OpINC<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpINC<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let access = A::decode(cpu, hw);
		let result = access.read(cpu, hw).wrapping_add(1);
		access.write(cpu, hw, result);
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("INC {}", A::asm_str(cpu))
	}
}

// Increment X
struct OpINX;
impl Instruction for OpINX {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let result = cpu.registers().x.wrapping_add(1);
		cpu.registers_mut().x = result;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("INX")
	}
}

// Increment Y
struct OpINY;
impl Instruction for OpINY {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let result = cpu.registers().y.wrapping_add(1);
		cpu.registers_mut().y = result;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("INY")
	}
}

// Jump (absolute).
struct OpJMPAbsolute;
impl Instruction for OpJMPAbsolute {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		cpu.registers_mut().pc = cpu.opcode16();
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("JMP ${:04X}", cpu.opcode16())
	}
}

// Jump (indirect).
struct OpJMPIndirect;
impl Instruction for OpJMPIndirect {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let iaddr_hi = cpu.opcode16() & 0xFF00;
		let iaddr_lo = cpu.opcode16() & 0x00FF;
		let addr_lo = cpu.read_memory(hw, iaddr_hi | iaddr_lo) as u16;
		let addr_hi = cpu.read_memory(hw, iaddr_hi | ((iaddr_lo + 1) & 0xFF)) as u16;
		cpu.registers_mut().pc = (addr_hi << 8) | addr_lo;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("JMP (${:04X})", cpu.opcode16())
	}
}

// INC and SBC.
struct OpISB<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpISB<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		OpINC::<A>{ phantom: PhantomData }.execute(cpu, hw);
		OpSBC::<A>{ phantom: PhantomData }.execute(cpu, hw);
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("ISB {}", A::asm_str(cpu))
	}
}

// Jump to subroutine.
struct OpJSR;
impl Instruction for OpJSR {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let mut sp = cpu.registers().s;
		let pc = cpu.registers().pc.wrapping_sub(1);
		cpu.write_memory(hw, STACK_START + sp as u16, (pc >> 8) as u8);
		sp = sp.wrapping_sub(1);
		cpu.write_memory(hw, STACK_START + sp as u16, pc as u8);
		sp = sp.wrapping_sub(1);

		let addr = cpu.opcode16();

		cpu.registers_mut().pc = addr;
		cpu.registers_mut().s = sp;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("JSR ${:04X}", cpu.opcode16())
	}
}

// Load accumulator and X.
struct OpLAX<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpLAX<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let result = A::decode(cpu, hw).read(cpu, hw);
		cpu.registers_mut().a = result;
		cpu.registers_mut().x = result;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("LAX {}", A::asm_str(cpu))
	}
}

// Load accumulator.
struct OpLDA<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpLDA<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let result = A::decode(cpu, hw).read(cpu, hw);
		cpu.registers_mut().a = result;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("LDA {}", A::asm_str(cpu))
	}
}

// Load X.
struct OpLDX<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpLDX<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let result = A::decode(cpu, hw).read(cpu, hw);
		cpu.registers_mut().x = result;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("LDX {}", A::asm_str(cpu))
	}
}

// Load accumulator.
struct OpLDY<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpLDY<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let result = A::decode(cpu, hw).read(cpu, hw);
		cpu.registers_mut().y = result;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("LDY {}", A::asm_str(cpu))
	}
}

// No operation.
struct OpNOPMulti<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpNOPMulti<A> {
	fn execute(&self, _: &mut Cpu, _: &mut Hardware) {
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("NOP {}", A::asm_str(cpu))
	}
}

// No operation.
struct OpNOPSingle;
impl Instruction for OpNOPSingle {
	fn execute(&self, _: &mut Cpu, _: &mut Hardware) {
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("NOP")
	}
}

// Logical shift right.
struct OpLSR<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpLSR<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let access = A::decode(cpu, hw);
		let src = access.read(cpu, hw);
		let result = src >> 1;
		access.write(cpu, hw, result);
		cpu.registers_mut().p.carry = src & 1 != 0;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("LSR {}", A::asm_str(cpu))
	}
}

// Logical inclusive or.
struct OpORA<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpORA<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let result = cpu.registers().a | A::decode(cpu, hw).read(cpu, hw);
		cpu.registers_mut().a = result;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("ORA {}", A::asm_str(cpu))
	}
}

// Push accumulator
struct OpPHA;
impl Instruction for OpPHA {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let sp = cpu.registers().s;
		let value = cpu.registers().a;
		cpu.write_memory(hw, STACK_START + sp as u16, value);
		cpu.registers_mut().s = sp.wrapping_sub(1);
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("PHA")
	}
}

// Push processor status
struct OpPHP;
impl Instruction for OpPHP {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let sp = cpu.registers().s;
		let value = cpu.registers().p.value(true);
		cpu.write_memory(hw, STACK_START + sp as u16, value);
		cpu.registers_mut().s = sp.wrapping_sub(1);
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("PHP")
	}
}

// Pull accumulator
struct OpPLA;
impl Instruction for OpPLA {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let sp = cpu.registers().s.wrapping_add(1);
		let value = cpu.read_memory(hw, STACK_START + sp as u16);
		cpu.registers_mut().a = value;
		cpu.registers_mut().s = sp;
		cpu.registers_mut().p.zero = value == 0;
		cpu.registers_mut().p.negative = value & 0x80 != 0;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("PLA")
	}
}

// Pull processor status
struct OpPLP;
impl Instruction for OpPLP {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let sp = cpu.registers().s.wrapping_add(1);
		let value = cpu.read_memory(hw, STACK_START + sp as u16);
		cpu.registers_mut().p.set_value(value);
		cpu.registers_mut().s = sp;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("PLP")
	}
}

// ROL + AND.
struct OpRLA<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpRLA<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		OpROL::<A>{ phantom: PhantomData }.execute(cpu, hw);
		OpAND::<A>{ phantom: PhantomData }.execute(cpu, hw);
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("RLA {}", A::asm_str(cpu))
	}
}

// Rotate left.
struct OpROL<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpROL<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let access = A::decode(cpu, hw);
		let src = access.read(cpu, hw);
		let result = (src << 1) | cpu.registers().p.carry as u8;
		access.write(cpu, hw, result);
		cpu.registers_mut().p.carry = src & 0x80 != 0;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("ROL {}", A::asm_str(cpu))
	}
}

// Rotate right.
struct OpROR<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpROR<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let access = A::decode(cpu, hw);
		let src = access.read(cpu, hw);
		let result = (src >> 1) | ((cpu.registers().p.carry as u8) << 7);
		access.write(cpu, hw, result);
		cpu.registers_mut().p.carry = src & 1 != 0;
		cpu.registers_mut().p.zero = result == 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("ROR {}", A::asm_str(cpu))
	}
}

// ROR + ADC.
struct OpRRA<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpRRA<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		OpROR::<A>{ phantom: PhantomData }.execute(cpu, hw);
		OpADC::<A>{ phantom: PhantomData }.execute(cpu, hw);
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("RRA {}", A::asm_str(cpu))
	}
}

// Return from interrupt.
struct OpRTI;
impl Instruction for OpRTI {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let mut sp = cpu.registers().s;
		sp = sp.wrapping_add(1);
		let p = cpu.read_memory(hw, STACK_START + sp as u16);
		sp = sp.wrapping_add(1);
		let addr_lo = cpu.read_memory(hw, STACK_START + sp as u16) as u16;
		sp = sp.wrapping_add(1);
		let addr_hi = cpu.read_memory(hw, STACK_START + sp as u16) as u16;
		let addr = (addr_hi << 8) | addr_lo;
		cpu.registers_mut().s = sp;
		cpu.registers_mut().pc = addr;
		cpu.registers_mut().p.set_value(p);
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("RTI")
	}
}

// Return from subroutine.
struct OpRTS;
impl Instruction for OpRTS {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let mut sp = cpu.registers().s;
		sp = sp.wrapping_add(1);
		let addr_lo = cpu.read_memory(hw, STACK_START + sp as u16) as u16;
		sp = sp.wrapping_add(1);
		let addr_hi = cpu.read_memory(hw, STACK_START + sp as u16) as u16;
		let addr = ((addr_hi << 8) | addr_lo).wrapping_add(1);
		cpu.registers_mut().s = sp;
		cpu.registers_mut().pc = addr;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("RTS")
	}
}

// Store A and X.
struct OpSAX<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpSAX<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let value = cpu.registers().a & cpu.registers().x;
		A::decode(cpu, hw).write(cpu, hw, value);
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("SAX {}", A::asm_str(cpu))
	}
}

// Add with carry.
struct OpSBC<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpSBC<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let a = cpu.registers().a as u16;
		let src = A::decode(cpu, hw).read(cpu, hw) as u16;
		let carry = 1 - cpu.registers().p.carry as u16;
		let result = a.wrapping_sub(src).wrapping_sub(carry);
		cpu.registers_mut().a = result as u8;
		cpu.registers_mut().p.carry = result <= 0xFF;
		cpu.registers_mut().p.zero = result & 0xFF == 0;
		cpu.registers_mut().p.overflow = (a ^ src) & 0x80 != 0 && (result ^ a) & 0x80 != 0;
		cpu.registers_mut().p.negative = result & 0x80 != 0;
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("SBC {}", A::asm_str(cpu))
	}
}

// Set carry flag.
struct OpSEC;
impl Instruction for OpSEC {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		cpu.registers_mut().p.carry = true;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("SEC")
	}
}

// Set decimal flag.
struct OpSED;
impl Instruction for OpSED {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		cpu.registers_mut().p.decimal = true;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("SED")
	}
}

// Set interrupt disable flag.
struct OpSEI;
impl Instruction for OpSEI {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		cpu.registers_mut().p.interrupt = true;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("SEI")
	}
}

// ASL + ORA.
struct OpSLO<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpSLO<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		OpASL::<A>{ phantom: PhantomData }.execute(cpu, hw);
		OpORA::<A>{ phantom: PhantomData }.execute(cpu, hw);
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("SLO {}", A::asm_str(cpu))
	}
}

// LSR + EOR.
struct OpSRE<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpSRE<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		OpLSR::<A>{ phantom: PhantomData }.execute(cpu, hw);
		OpEOR::<A>{ phantom: PhantomData }.execute(cpu, hw);
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("SRE {}", A::asm_str(cpu))
	}
}

// Store accumulator.
struct OpSTA<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpSTA<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let value = cpu.registers().a;
		A::decode(cpu, hw).write(cpu, hw, value);
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("STA {}", A::asm_str(cpu))
	}
}

// Store accumulator.
struct OpSTX<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpSTX<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let value = cpu.registers().x;
		A::decode(cpu, hw).write(cpu, hw, value);
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("STX {}", A::asm_str(cpu))
	}
}

// Store accumulator.
struct OpSTY<A: AddrMode> {
	phantom: PhantomData<A>,
}
impl<A: AddrMode> Instruction for OpSTY<A> {
	fn execute(&self, cpu: &mut Cpu, hw: &mut Hardware) {
		let value = cpu.registers().y;
		A::decode(cpu, hw).write(cpu, hw, value);
	}
	fn asm_str(&self, cpu: &Cpu) -> String {
		format!("STY {}", A::asm_str(cpu))
	}
}

// Transfer accumulator to X.
struct OpTAX;
impl Instruction for OpTAX {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let value = cpu.registers().a;
		cpu.registers_mut().x = value;
		cpu.registers_mut().p.zero = value == 0;
		cpu.registers_mut().p.negative = value & 0x80 != 0;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("TAX")
	}
}

// Transfer accumulator to Y.
struct OpTAY;
impl Instruction for OpTAY {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let value = cpu.registers().a;
		cpu.registers_mut().y = value;
		cpu.registers_mut().p.zero = value == 0;
		cpu.registers_mut().p.negative = value & 0x80 != 0;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("TAY")
	}
}

// Transfer stack pointer to X.
struct OpTSX;
impl Instruction for OpTSX {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let value = cpu.registers().s;
		cpu.registers_mut().x = value;
		cpu.registers_mut().p.zero = value == 0;
		cpu.registers_mut().p.negative = value & 0x80 != 0;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("TSX")
	}
}

// Transfer X to accumulator.
struct OpTXA;
impl Instruction for OpTXA {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let value = cpu.registers().x;
		cpu.registers_mut().a = value;
		cpu.registers_mut().p.zero = value == 0;
		cpu.registers_mut().p.negative = value & 0x80 != 0;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("TXA")
	}
}

// Transfer X to stack pointer.
struct OpTXS;
impl Instruction for OpTXS {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let value = cpu.registers().x;
		cpu.registers_mut().s = value;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("TXS")
	}
}

// Transfer Y to accumulator.
struct OpTYA;
impl Instruction for OpTYA {
	fn execute(&self, cpu: &mut Cpu, _: &mut Hardware) {
		let value = cpu.registers().y;
		cpu.registers_mut().a = value;
		cpu.registers_mut().p.zero = value == 0;
		cpu.registers_mut().p.negative = value & 0x80 != 0;
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("TYA")
	}
}

// TODO Inofficial Instructions
struct OpTODO;
impl Instruction for OpTODO {
	fn execute(&self, _: &mut Cpu, _: &mut Hardware) {
		unimplemented!()
	}
	fn asm_str(&self, _: &Cpu) -> String {
		String::from("???")
	}
}

pub const INSTRUCTION_SIZES: [usize; 256] = [
	//         0  1  2  3  4  5  6  7  8  9  A  B  C  D  E  F
	/* 0x00 */ 2, 2, 1, 2, 2, 2, 2, 2, 1, 2, 1, 2, 3, 3, 3, 3,
	/* 0x10 */ 2, 2, 1, 2, 2, 2, 2, 2, 1, 3, 1, 3, 3, 3, 3, 3,
	/* 0x20 */ 3, 2, 1, 2, 2, 2, 2, 2, 1, 2, 1, 2, 3, 3, 3, 3,
	/* 0x30 */ 2, 2, 1, 2, 2, 2, 2, 2, 1, 3, 1, 3, 3, 3, 3, 3,
	/* 0x40 */ 1, 2, 1, 2, 2, 2, 2, 2, 1, 2, 1, 2, 3, 3, 3, 3,
	/* 0x50 */ 2, 2, 1, 2, 2, 2, 2, 2, 1, 3, 1, 3, 3, 3, 3, 3,
	/* 0x60 */ 1, 2, 1, 2, 2, 2, 2, 2, 1, 2, 1, 2, 3, 3, 3, 3,
	/* 0x70 */ 2, 2, 1, 2, 2, 2, 2, 2, 1, 3, 1, 3, 3, 3, 3, 3,
	/* 0x80 */ 2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 3,
	/* 0x90 */ 2, 2, 1, 1, 2, 2, 2, 2, 1, 3, 1, 1, 1, 3, 1, 1,
	/* 0xA0 */ 2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 2, 3, 3, 3, 3,
	/* 0xB0 */ 2, 2, 1, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 3,
	/* 0xC0 */ 2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 2, 3, 3, 3, 3,
	/* 0xD0 */ 2, 2, 1, 2, 2, 2, 2, 2, 1, 3, 1, 3, 3, 3, 3, 3,
	/* 0xE0 */ 2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 2, 3, 3, 3, 3,
	/* 0xF0 */ 2, 2, 1, 2, 2, 2, 2, 2, 1, 3, 1, 3, 3, 3, 3, 3,
];

pub const INSTRUCTIONS: [&'static (Instruction + Sync); 256] = [
	// 0x00
	/* 0 */ &OpBRK,
	/* 1 */ &OpORA::<AddrIndirectX>{ phantom: PhantomData },
	/* 2 */ &OpTODO,
	/* 3 */ &OpSLO::<AddrIndirectX>{ phantom: PhantomData },
	/* 4 */ &OpNOPMulti::<AddrZeroPage>{ phantom: PhantomData },
	/* 5 */ &OpORA::<AddrZeroPage>{ phantom: PhantomData },
	/* 6 */ &OpASL::<AddrZeroPage>{ phantom: PhantomData },
	/* 7 */ &OpSLO::<AddrZeroPage>{ phantom: PhantomData },
	/* 8 */ &OpPHP,
	/* 9 */ &OpORA::<AddrImmediate>{ phantom: PhantomData },
	/* A */ &OpASL::<AddrAccumulator>{ phantom: PhantomData },
	/* B */ &OpANC::<AddrImmediate>{ phantom: PhantomData },
	/* C */ &OpNOPMulti::<AddrAbsolute>{ phantom: PhantomData },
	/* D */ &OpORA::<AddrAbsolute>{ phantom: PhantomData },
	/* E */ &OpASL::<AddrAbsolute>{ phantom: PhantomData },
	/* F */ &OpSLO::<AddrAbsolute>{ phantom: PhantomData },
	
	// 0x10
	/* 0 */ &OpBPL,
	/* 1 */ &OpORA::<AddrIndirectY>{ phantom: PhantomData },
	/* 2 */ &OpTODO,
	/* 3 */ &OpSLO::<AddrIndirectY>{ phantom: PhantomData },
	/* 4 */ &OpNOPMulti::<AddrZeroPageX>{ phantom: PhantomData },
	/* 5 */ &OpORA::<AddrZeroPageX>{ phantom: PhantomData },
	/* 6 */ &OpASL::<AddrZeroPageX>{ phantom: PhantomData },
	/* 7 */ &OpSLO::<AddrZeroPageX>{ phantom: PhantomData },
	/* 8 */ &OpCLC,
	/* 9 */ &OpORA::<AddrAbsoluteY>{ phantom: PhantomData },
	/* A */ &OpNOPSingle,
	/* B */ &OpSLO::<AddrAbsoluteY>{ phantom: PhantomData },
	/* C */ &OpNOPMulti::<AddrAbsoluteX>{ phantom: PhantomData },
	/* D */ &OpORA::<AddrAbsoluteX>{ phantom: PhantomData },
	/* E */ &OpASL::<AddrAbsoluteX>{ phantom: PhantomData },
	/* F */ &OpSLO::<AddrAbsoluteX>{ phantom: PhantomData },
	
	// 0x20
	/* 0 */ &OpJSR,
	/* 1 */ &OpAND::<AddrIndirectX>{ phantom: PhantomData },
	/* 2 */ &OpTODO,
	/* 3 */ &OpRLA::<AddrIndirectX>{ phantom: PhantomData },
	/* 4 */ &OpBIT::<AddrZeroPage>{ phantom: PhantomData },
	/* 5 */ &OpAND::<AddrZeroPage>{ phantom: PhantomData },
	/* 6 */ &OpROL::<AddrZeroPage>{ phantom: PhantomData },
	/* 7 */ &OpRLA::<AddrZeroPage>{ phantom: PhantomData },
	/* 8 */ &OpPLP,
	/* 9 */ &OpAND::<AddrImmediate>{ phantom: PhantomData },
	/* A */ &OpROL::<AddrAccumulator>{ phantom: PhantomData },
	/* B */ &OpANC::<AddrImmediate>{ phantom: PhantomData },
	/* C */ &OpBIT::<AddrAbsolute>{ phantom: PhantomData },
	/* D */ &OpAND::<AddrAbsolute>{ phantom: PhantomData },
	/* E */ &OpROL::<AddrAbsolute>{ phantom: PhantomData },
	/* F */ &OpRLA::<AddrAbsolute>{ phantom: PhantomData },
	
	// 0x30
	/* 0 */ &OpBMI,
	/* 1 */ &OpAND::<AddrIndirectY>{ phantom: PhantomData },
	/* 2 */ &OpTODO,
	/* 3 */ &OpRLA::<AddrIndirectY>{ phantom: PhantomData },
	/* 4 */ &OpNOPMulti::<AddrZeroPageX>{ phantom: PhantomData },
	/* 5 */ &OpAND::<AddrZeroPageX>{ phantom: PhantomData },
	/* 6 */ &OpROL::<AddrZeroPageX>{ phantom: PhantomData },
	/* 7 */ &OpRLA::<AddrZeroPageX>{ phantom: PhantomData },
	/* 8 */ &OpSEC,
	/* 9 */ &OpAND::<AddrAbsoluteY>{ phantom: PhantomData },
	/* A */ &OpNOPSingle,
	/* B */ &OpRLA::<AddrAbsoluteY>{ phantom: PhantomData },
	/* C */ &OpNOPMulti::<AddrAbsoluteX>{ phantom: PhantomData },
	/* D */ &OpAND::<AddrAbsoluteX>{ phantom: PhantomData },
	/* E */ &OpROL::<AddrAbsoluteX>{ phantom: PhantomData },
	/* F */ &OpRLA::<AddrAbsoluteX>{ phantom: PhantomData },
	
	// 0x40
	/* 0 */ &OpRTI,
	/* 1 */ &OpEOR::<AddrIndirectX>{ phantom: PhantomData },
	/* 2 */ &OpTODO,
	/* 3 */ &OpSRE::<AddrIndirectX>{ phantom: PhantomData },
	/* 4 */ &OpNOPMulti::<AddrZeroPage>{ phantom: PhantomData },
	/* 5 */ &OpEOR::<AddrZeroPage>{ phantom: PhantomData },
	/* 6 */ &OpLSR::<AddrZeroPage>{ phantom: PhantomData },
	/* 7 */ &OpSRE::<AddrZeroPage>{ phantom: PhantomData },
	/* 8 */ &OpPHA,
	/* 9 */ &OpEOR::<AddrImmediate>{ phantom: PhantomData },
	/* A */ &OpLSR::<AddrAccumulator>{ phantom: PhantomData },
	/* B */ &OpALR::<AddrImmediate>{ phantom: PhantomData },
	/* C */ &OpJMPAbsolute,
	/* D */ &OpEOR::<AddrAbsolute>{ phantom: PhantomData },
	/* E */ &OpLSR::<AddrAbsolute>{ phantom: PhantomData },
	/* F */ &OpSRE::<AddrAbsolute>{ phantom: PhantomData },
	
	// 0x50
	/* 0 */ &OpBVC,
	/* 1 */ &OpEOR::<AddrIndirectY>{ phantom: PhantomData },
	/* 2 */ &OpTODO,
	/* 3 */ &OpSRE::<AddrIndirectY>{ phantom: PhantomData },
	/* 4 */ &OpNOPMulti::<AddrZeroPageX>{ phantom: PhantomData },
	/* 5 */ &OpEOR::<AddrZeroPageX>{ phantom: PhantomData },
	/* 6 */ &OpLSR::<AddrZeroPageX>{ phantom: PhantomData },
	/* 7 */ &OpSRE::<AddrZeroPageX>{ phantom: PhantomData },
	/* 8 */ &OpCLI,
	/* 9 */ &OpEOR::<AddrAbsoluteY>{ phantom: PhantomData },
	/* A */ &OpNOPSingle,
	/* B */ &OpSRE::<AddrAbsoluteY>{ phantom: PhantomData },
	/* C */ &OpNOPMulti::<AddrAbsoluteX>{ phantom: PhantomData },
	/* D */ &OpEOR::<AddrAbsoluteX>{ phantom: PhantomData },
	/* E */ &OpLSR::<AddrAbsoluteX>{ phantom: PhantomData },
	/* F */ &OpSRE::<AddrAbsoluteX>{ phantom: PhantomData },
	
	// 0x60
	/* 0 */ &OpRTS,
	/* 1 */ &OpADC::<AddrIndirectX>{ phantom: PhantomData },
	/* 2 */ &OpTODO,
	/* 3 */ &OpRRA::<AddrIndirectX>{ phantom: PhantomData },
	/* 4 */ &OpNOPMulti::<AddrZeroPage>{ phantom: PhantomData },
	/* 5 */ &OpADC::<AddrZeroPage>{ phantom: PhantomData },
	/* 6 */ &OpROR::<AddrZeroPage>{ phantom: PhantomData },
	/* 7 */ &OpRRA::<AddrZeroPage>{ phantom: PhantomData },
	/* 8 */ &OpPLA,
	/* 9 */ &OpADC::<AddrImmediate>{ phantom: PhantomData },
	/* A */ &OpROR::<AddrAccumulator>{ phantom: PhantomData },
	/* B */ &OpARR::<AddrImmediate>{ phantom: PhantomData },
	/* C */ &OpJMPIndirect,
	/* D */ &OpADC::<AddrAbsolute>{ phantom: PhantomData },
	/* E */ &OpROR::<AddrAbsolute>{ phantom: PhantomData },
	/* F */ &OpRRA::<AddrAbsolute>{ phantom: PhantomData },
	
	// 0x70
	/* 0 */ &OpBVS,
	/* 1 */ &OpADC::<AddrIndirectY>{ phantom: PhantomData },
	/* 2 */ &OpTODO,
	/* 3 */ &OpRRA::<AddrIndirectY>{ phantom: PhantomData },
	/* 4 */ &OpNOPMulti::<AddrZeroPageX>{ phantom: PhantomData },
	/* 5 */ &OpADC::<AddrZeroPageX>{ phantom: PhantomData },
	/* 6 */ &OpROR::<AddrZeroPageX>{ phantom: PhantomData },
	/* 7 */ &OpRRA::<AddrZeroPageX>{ phantom: PhantomData },
	/* 8 */ &OpSEI,
	/* 9 */ &OpADC::<AddrAbsoluteY>{ phantom: PhantomData },
	/* A */ &OpNOPSingle,
	/* B */ &OpRRA::<AddrAbsoluteY>{ phantom: PhantomData },
	/* C */ &OpNOPMulti::<AddrAbsoluteX>{ phantom: PhantomData },
	/* D */ &OpADC::<AddrAbsoluteX>{ phantom: PhantomData },
	/* E */ &OpROR::<AddrAbsoluteX>{ phantom: PhantomData },
	/* F */ &OpRRA::<AddrAbsoluteX>{ phantom: PhantomData },
	
	// 0x80
	/* 0 */ &OpNOPMulti::<AddrImmediate>{ phantom: PhantomData },
	/* 1 */ &OpSTA::<AddrIndirectX>{ phantom: PhantomData },
	/* 2 */ &OpNOPMulti::<AddrImmediate>{ phantom: PhantomData },
	/* 3 */ &OpSAX::<AddrIndirectX>{ phantom: PhantomData },
	/* 4 */ &OpSTY::<AddrZeroPage>{ phantom: PhantomData },
	/* 5 */ &OpSTA::<AddrZeroPage>{ phantom: PhantomData },
	/* 6 */ &OpSTX::<AddrZeroPage>{ phantom: PhantomData },
	/* 7 */ &OpSAX::<AddrZeroPage>{ phantom: PhantomData },
	/* 8 */ &OpDEY,
	/* 9 */ &OpNOPMulti::<AddrImmediate>{ phantom: PhantomData },
	/* A */ &OpTXA,
	/* B */ &OpTODO,
	/* C */ &OpSTY::<AddrAbsolute>{ phantom: PhantomData },
	/* D */ &OpSTA::<AddrAbsolute>{ phantom: PhantomData },
	/* E */ &OpSTX::<AddrAbsolute>{ phantom: PhantomData },
	/* F */ &OpSAX::<AddrAbsolute>{ phantom: PhantomData },
	
	// 0x90
	/* 0 */ &OpBCC,
	/* 1 */ &OpSTA::<AddrIndirectY>{ phantom: PhantomData },
	/* 2 */ &OpTODO,
	/* 3 */ &OpTODO,
	/* 4 */ &OpSTY::<AddrZeroPageX>{ phantom: PhantomData },
	/* 5 */ &OpSTA::<AddrZeroPageX>{ phantom: PhantomData },
	/* 6 */ &OpSTX::<AddrZeroPageY>{ phantom: PhantomData },
	/* 7 */ &OpSAX::<AddrZeroPageY>{ phantom: PhantomData },
	/* 8 */ &OpTYA,
	/* 9 */ &OpSTA::<AddrAbsoluteY>{ phantom: PhantomData },
	/* A */ &OpTXS,
	/* B */ &OpTODO,
	/* C */ &OpTODO,
	/* D */ &OpSTA::<AddrAbsoluteX>{ phantom: PhantomData },
	/* E */ &OpTODO,
	/* F */ &OpTODO,
	
	// 0xA0
	/* 0 */ &OpLDY::<AddrImmediate>{ phantom: PhantomData },
	/* 1 */ &OpLDA::<AddrIndirectX>{ phantom: PhantomData },
	/* 2 */ &OpLDX::<AddrImmediate>{ phantom: PhantomData },
	/* 3 */ &OpLAX::<AddrIndirectX>{ phantom: PhantomData },
	/* 4 */ &OpLDY::<AddrZeroPage>{ phantom: PhantomData },
	/* 5 */ &OpLDA::<AddrZeroPage>{ phantom: PhantomData },
	/* 6 */ &OpLDX::<AddrZeroPage>{ phantom: PhantomData },
	/* 7 */ &OpLAX::<AddrZeroPage>{ phantom: PhantomData },
	/* 8 */ &OpTAY,
	/* 9 */ &OpLDA::<AddrImmediate>{ phantom: PhantomData },
	/* A */ &OpTAX,
	/* B */ &OpLAX::<AddrImmediate>{ phantom: PhantomData },
	/* C */ &OpLDY::<AddrAbsolute>{ phantom: PhantomData },
	/* D */ &OpLDA::<AddrAbsolute>{ phantom: PhantomData },
	/* E */ &OpLDX::<AddrAbsolute>{ phantom: PhantomData },
	/* F */ &OpLAX::<AddrAbsolute>{ phantom: PhantomData },
	
	// 0xB0
	/* 0 */ &OpBCS,
	/* 1 */ &OpLDA::<AddrIndirectY>{ phantom: PhantomData },
	/* 2 */ &OpTODO,
	/* 3 */ &OpLAX::<AddrIndirectY>{ phantom: PhantomData },
	/* 4 */ &OpLDY::<AddrZeroPageX>{ phantom: PhantomData },
	/* 5 */ &OpLDA::<AddrZeroPageX>{ phantom: PhantomData },
	/* 6 */ &OpLDX::<AddrZeroPageY>{ phantom: PhantomData },
	/* 7 */ &OpLAX::<AddrZeroPageY>{ phantom: PhantomData },
	/* 8 */ &OpCLV,
	/* 9 */ &OpLDA::<AddrAbsoluteY>{ phantom: PhantomData },
	/* A */ &OpTSX,
	/* B */ &OpTODO,
	/* C */ &OpLDY::<AddrAbsoluteX>{ phantom: PhantomData },
	/* D */ &OpLDA::<AddrAbsoluteX>{ phantom: PhantomData },
	/* E */ &OpLDX::<AddrAbsoluteY>{ phantom: PhantomData },
	/* F */ &OpLAX::<AddrAbsoluteY>{ phantom: PhantomData },
	
	// 0xC0
	/* 0 */ &OpCPY::<AddrImmediate>{ phantom: PhantomData },
	/* 1 */ &OpCMP::<AddrIndirectX>{ phantom: PhantomData },
	/* 2 */ &OpNOPMulti::<AddrImmediate>{ phantom: PhantomData },
	/* 3 */ &OpDCP::<AddrIndirectX>{ phantom: PhantomData },
	/* 4 */ &OpCPY::<AddrZeroPage>{ phantom: PhantomData },
	/* 5 */ &OpCMP::<AddrZeroPage>{ phantom: PhantomData },
	/* 6 */ &OpDEC::<AddrZeroPage>{ phantom: PhantomData },
	/* 7 */ &OpDCP::<AddrZeroPage>{ phantom: PhantomData },
	/* 8 */ &OpINY,
	/* 9 */ &OpCMP::<AddrImmediate>{ phantom: PhantomData },
	/* A */ &OpDEX,
	/* B */ &OpAXS::<AddrImmediate>{ phantom: PhantomData },
	/* C */ &OpCPY::<AddrAbsolute>{ phantom: PhantomData },
	/* D */ &OpCMP::<AddrAbsolute>{ phantom: PhantomData },
	/* E */ &OpDEC::<AddrAbsolute>{ phantom: PhantomData },
	/* F */ &OpDCP::<AddrAbsolute>{ phantom: PhantomData },
	
	// 0xD0
	/* 0 */ &OpBNE,
	/* 1 */ &OpCMP::<AddrIndirectY>{ phantom: PhantomData },
	/* 2 */ &OpTODO,
	/* 3 */ &OpDCP::<AddrIndirectY>{ phantom: PhantomData },
	/* 4 */ &OpNOPMulti::<AddrZeroPageX>{ phantom: PhantomData },
	/* 5 */ &OpCMP::<AddrZeroPageX>{ phantom: PhantomData },
	/* 6 */ &OpDEC::<AddrZeroPageX>{ phantom: PhantomData },
	/* 7 */ &OpDCP::<AddrZeroPageX>{ phantom: PhantomData },
	/* 8 */ &OpCLD,
	/* 9 */ &OpCMP::<AddrAbsoluteY>{ phantom: PhantomData },
	/* A */ &OpNOPSingle,
	/* B */ &OpDCP::<AddrAbsoluteY>{ phantom: PhantomData },
	/* C */ &OpNOPMulti::<AddrAbsoluteX>{ phantom: PhantomData },
	/* D */ &OpCMP::<AddrAbsoluteX>{ phantom: PhantomData },
	/* E */ &OpDEC::<AddrAbsoluteX>{ phantom: PhantomData },
	/* F */ &OpDCP::<AddrAbsoluteX>{ phantom: PhantomData },
	
	// 0xE0
	/* 0 */ &OpCPX::<AddrImmediate>{ phantom: PhantomData },
	/* 1 */ &OpSBC::<AddrIndirectX>{ phantom: PhantomData },
	/* 2 */ &OpNOPMulti::<AddrImmediate>{ phantom: PhantomData },
	/* 3 */ &OpISB::<AddrIndirectX>{ phantom: PhantomData },
	/* 4 */ &OpCPX::<AddrZeroPage>{ phantom: PhantomData },
	/* 5 */ &OpSBC::<AddrZeroPage>{ phantom: PhantomData },
	/* 6 */ &OpINC::<AddrZeroPage>{ phantom: PhantomData },
	/* 7 */ &OpISB::<AddrZeroPage>{ phantom: PhantomData },
	/* 8 */ &OpINX,
	/* 9 */ &OpSBC::<AddrImmediate>{ phantom: PhantomData },
	/* A */ &OpNOPSingle,
	/* B */ &OpSBC::<AddrImmediate>{ phantom: PhantomData },
	/* C */ &OpCPX::<AddrAbsolute>{ phantom: PhantomData },
	/* D */ &OpSBC::<AddrAbsolute>{ phantom: PhantomData },
	/* E */ &OpINC::<AddrAbsolute>{ phantom: PhantomData },
	/* F */ &OpISB::<AddrAbsolute>{ phantom: PhantomData },
	
	// 0xF0
	/* 0 */ &OpBEQ,
	/* 1 */ &OpSBC::<AddrIndirectY>{ phantom: PhantomData },
	/* 2 */ &OpTODO,
	/* 3 */ &OpISB::<AddrIndirectY>{ phantom: PhantomData },
	/* 4 */ &OpNOPMulti::<AddrZeroPageX>{ phantom: PhantomData },
	/* 5 */ &OpSBC::<AddrZeroPageX>{ phantom: PhantomData },
	/* 6 */ &OpINC::<AddrZeroPageX>{ phantom: PhantomData },
	/* 7 */ &OpISB::<AddrZeroPageX>{ phantom: PhantomData },
	/* 8 */ &OpSED,
	/* 9 */ &OpSBC::<AddrAbsoluteY>{ phantom: PhantomData },
	/* A */ &OpNOPSingle,
	/* B */ &OpISB::<AddrAbsoluteY>{ phantom: PhantomData },
	/* C */ &OpNOPMulti::<AddrAbsoluteX>{ phantom: PhantomData },
	/* D */ &OpSBC::<AddrAbsoluteX>{ phantom: PhantomData },
	/* E */ &OpINC::<AddrAbsoluteX>{ phantom: PhantomData },
	/* F */ &OpISB::<AddrAbsoluteX>{ phantom: PhantomData },
];


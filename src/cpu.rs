use memory::Memory;
use memory_map;
use cartridge::Cartridge;
use instructions::{INSTRUCTION_SIZES, INSTRUCTIONS};
use std::io::Write;

pub const STACK_START: u16 = 0x0100;

// Status register
pub struct Status {
	pub carry: bool,
	pub zero: bool,
	pub interrupt: bool,
	pub decimal: bool,
	pub overflow: bool,
	pub negative: bool,
}

impl Status {
	pub fn new() -> Status {
		// TODO initial values
		let mut p = Status {
			carry: false, zero: false, interrupt: false, decimal: false,
			overflow: false, negative: false
		};
		p.set_value(0x34);
		p
	}

	pub fn value(&self, break_flag: bool) -> u8 {
		0b00100000 |
			if break_flag     { 0b00010000 } else { 0 } |
			if self.carry     { 0b00000001 } else { 0 } |
			if self.zero      { 0b00000010 } else { 0 } |
			if self.interrupt { 0b00000100 } else { 0 } |
			if self.decimal   { 0b00001000 } else { 0 } |
			if self.overflow  { 0b01000000 } else { 0 } |
			if self.negative  { 0b10000000 } else { 0 }
	}

	pub fn set_value(&mut self, value: u8) {
		self.carry     = value & 0b00000001 != 0;
		self.zero      = value & 0b00000010 != 0;
		self.interrupt = value & 0b00000100 != 0;
		self.decimal   = value & 0b00001000 != 0;
		self.overflow  = value & 0b01000000 != 0;
		self.negative  = value & 0b10000000 != 0;
	}
}

// Register file of the CPU.
pub struct Registers {
	pub a: u8,
	pub x: u8,
	pub y: u8,
	pub pc: u16,
	pub s: u8,
	pub p: Status,
}

impl Registers {
	pub fn new() -> Registers {
		// TODO initial values!
		Registers {
			a: 0,
			x: 0,
			y: 0,
			pc: 0,
			s: 0xFD,
			p: Status::new()
		}
	}
}

// CPU and "root object" of the NES. Everything ends up here at the end.
//
// The memory map is as follows:
// * 0000 - 07FF is RAM
// * 0800 - 1FFF mirrors RAM
// * 2000 - 2007 are PPU registers
// * 2008 - 3FFF mirrors PPU registers
// * 4000 - 401F are APU and IO registers
// * 4020 - FFFF cartridge space
pub struct Cpu {
	registers: Registers,
	memory: Memory,
	cartridge: Box<Cartridge>,
	opcode8: u8,
	opcode16: u16,
}

impl Cpu {
	pub fn new(cartridge: Box<Cartridge>) -> Cpu {
		let mut it = Cpu {
			registers: Registers::new(),
			memory: Memory::new(),
			cartridge: cartridge,
			opcode8: 0,
			opcode16: 0,
		};
		it.reset();
		it
	}

	fn reset(&mut self) {
		let addr_lo = self.read_memory(0xFFFC) as u16;
		let addr_hi = self.read_memory(0xFFFD) as u16;
		self.registers.pc = (addr_hi << 8) | addr_lo;
	}

	pub fn jump_to_interrupt(&mut self, break_flag: bool) {
		let mut sp = self.registers.s;
		let old_pc = self.registers.pc;
		let old_p = self.registers.p.value(break_flag);
		self.write_memory(STACK_START + sp as u16, (old_pc >> 8) as u8);
		sp = sp.wrapping_sub(1);
		self.write_memory(STACK_START + sp as u16, old_pc as u8);
		sp = sp.wrapping_sub(1);
		self.write_memory(STACK_START + sp as u16, old_p);
		sp = sp.wrapping_sub(1);

		let addr_lo = self.read_memory(0xFFFE) as u16;
		let addr_hi = self.read_memory(0xFFFF) as u16;
		self.registers.pc = (addr_hi << 8) | addr_lo;
		self.registers.p.interrupt = true;
		self.registers.s = sp;
	}

	pub fn registers_mut(&mut self) -> &mut Registers {
		&mut self.registers
	}

	pub fn registers(&self) -> &Registers {
		&self.registers
	}

	pub fn write_memory(&mut self, address: u16, value: u8) {
		if address < memory_map::PPU_START {
			self.memory.write(address, value);
		} else if address < memory_map::APU_IO_START {
			// TODO
		} else if address < memory_map::CARTRIDGE_START {
			// TODO
		} else {
			self.cartridge.write_cpu(address, value);
		}
	}

	pub fn read_memory(&mut self, address: u16) -> u8 {
		if address < memory_map::PPU_START {
			self.memory.read(address)
		} else if address < memory_map::APU_IO_START {
			0  // TODO
		} else if address < memory_map::CARTRIDGE_START {
			0  // TODO
		} else {
			self.cartridge.read_cpu(address)
		}
	}

	// Returns the value of the last 2 byte opcode.
	pub fn opcode8(&self) -> u8 {
		self.opcode8
	}

	// Returns the value of the last 3 byte opcode.
	pub fn opcode16(&self) -> u16 {
		self.opcode16
	}

	// One CPU tick.
	pub fn tick(&mut self, instr_log: &mut Option<&mut Write>) {
		// fetch PC
		let mut pc = self.registers.pc;

		// decode
		let mut opcode = [0, 0, 0];
		opcode[0] = self.read_memory(pc);
		pc = pc.wrapping_add(1);
		let opcode_size = INSTRUCTION_SIZES[opcode[0] as usize];
		match opcode_size {
			1 => {}
			2 => {
				opcode[1] = self.read_memory(pc);
				pc = pc.wrapping_add(1);
				self.opcode8 = opcode[1];
			}
			3 => {
				opcode[1] = self.read_memory(pc);
				pc = pc.wrapping_add(1);
				opcode[2] = self.read_memory(pc);
				pc = pc.wrapping_add(1);
				self.opcode16 = ((opcode[2] as u16) << 8) | (opcode[1] as u16);
			}
			_ => { unreachable!(); }
		};
		let instruction = INSTRUCTIONS[opcode[0] as usize];

		// log
		if let &mut Some(ref mut fp) = instr_log {
			let asm_str = instruction.asm_str(self);
			let _ = writeln!(
				fp,
				"{:04X}  {:-8}  {:-30}  A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}",
				self.registers.pc,
				match opcode_size {
					1 => { format!("{:02X}", opcode[0]) }
					2 => { format!("{:02X} {:02X}", opcode[0], opcode[1]) }
					3 => { format!("{:02X} {:02X} {:02X}", opcode[0], opcode[1], opcode[2]) }
					_ => { unreachable!() }
				},
				asm_str,
				self.registers.a,
				self.registers.x,
				self.registers.y,
				self.registers.p.value(false),
				self.registers.s);
		}

		// execute
		self.registers.pc = pc;
		instruction.execute(self);
	}
}

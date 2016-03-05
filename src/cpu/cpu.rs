use cpu::memory_map;
use cartridge::Cartridge;
use cpu::instructions::{INSTRUCTION_SIZES, INSTRUCTIONS};
use std::io::Write;
use ppu::Ppu;
use apu::Apu;

// Tuple to pass the whole hardware to the CPU.
pub struct Hardware<'a> {
	pub apu: &'a mut Apu,
	pub ppu: &'a mut Ppu,
	pub cartridge: &'a mut Cartridge
}

// Start of the stack
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

// CPU of the NES.
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
	opcode8: u8,
	opcode16: u16,
	ram: [u8; memory_map::RAM_SIZE as usize],
}

impl Cpu {
	pub fn new() -> Cpu {
		Cpu {
			registers: Registers::new(),
			opcode8: 0,
			opcode16: 0,
			ram: [0; memory_map::RAM_SIZE as usize],
		}
	}

	pub fn jump_to_start(&mut self, hw: &mut Hardware) {
		let addr_lo = self.read_memory(hw, 0xFFFC) as u16;
		let addr_hi = self.read_memory(hw, 0xFFFD) as u16;
		self.registers.pc = (addr_hi << 8) | addr_lo;
	}

	pub fn jump_to_interrupt(&mut self, hw: &mut Hardware, break_flag: bool) {
		let mut sp = self.registers.s;
		let old_pc = self.registers.pc;
		let old_p = self.registers.p.value(break_flag);
		self.write_memory(hw, STACK_START + sp as u16, (old_pc >> 8) as u8);
		sp = sp.wrapping_sub(1);
		self.write_memory(hw, STACK_START + sp as u16, old_pc as u8);
		sp = sp.wrapping_sub(1);
		self.write_memory(hw, STACK_START + sp as u16, old_p);
		sp = sp.wrapping_sub(1);

		let addr_lo = self.read_memory(hw, 0xFFFE) as u16;
		let addr_hi = self.read_memory(hw, 0xFFFF) as u16;
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

	pub fn write_memory(&mut self, hw: &mut Hardware, address: u16, value: u8) {
		if address < memory_map::PPU_START {
			self.ram[(address & (memory_map::RAM_SIZE - 1)) as usize] = value;
		} else if address < memory_map::APU_IO_START {
			hw.ppu.write(hw.cartridge, address, value);
		} else if address < memory_map::CARTRIDGE_START {
			// TODO
		} else {
			hw.cartridge.write_cpu(address, value);
		}
	}

	pub fn read_memory(&self, hw: &mut Hardware, address: u16) -> u8 {
		if address < memory_map::PPU_START {
			self.ram[(address & (memory_map::RAM_SIZE - 1)) as usize]
		} else if address < memory_map::APU_IO_START {
			hw.ppu.read(hw.cartridge, address)
		} else if address < memory_map::CARTRIDGE_START {
			// TODO
			//hw.apu.read(address)
			0
		} else {
			hw.cartridge.read_cpu(address)
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
	pub fn tick(&mut self, hw: &mut Hardware, instr_log: &mut Option<&mut Write>) {
		// fetch PC
		let mut pc = self.registers.pc;

		// decode
		let mut opcode = [0, 0, 0];
		opcode[0] = self.read_memory(hw, pc);
		pc = pc.wrapping_add(1);
		let opcode_size = INSTRUCTION_SIZES[opcode[0] as usize];
		match opcode_size {
			1 => {}
			2 => {
				opcode[1] = self.read_memory(hw, pc);
				pc = pc.wrapping_add(1);
				self.opcode8 = opcode[1];
			}
			3 => {
				opcode[1] = self.read_memory(hw, pc);
				pc = pc.wrapping_add(1);
				opcode[2] = self.read_memory(hw, pc);
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
		instruction.execute(self, hw);
	}
}

use memory_map;

const PPUCTRL: usize = 0;
const PPUMASK: usize = 1;
const PPUSTATUS: usize = 2;
const OAMADDR: usize = 3;
const OAMDATA: usize = 4;
const PPUSCROLL: usize = 5;
const PPUADDR: usize = 6;
const PPUDATA: usize = 7;

pub struct Ppu {
	// http://wiki.nesdev.com/w/index.php/PPU_registers
	registers: [u8; 8],
}

impl Ppu {
	pub fn new() -> Ppu {
		Ppu {
			registers: [0; 8],
		}
	}

	pub fn read(&mut self, addr: u16) -> u8 {
		debug_assert!(memory_map::PPU_START <= addr && addr < memory_map::APU_IO_START);
		let len = self.registers.len();
		self.registers[(addr - memory_map::PPU_START) as usize % len]
	}

	pub fn write(&mut self, addr: u16, value: u8) {
		debug_assert!(memory_map::PPU_START <= addr && addr < memory_map::APU_IO_START);
		if addr != 0x2002 {
			let len = self.registers.len();
			self.registers[(addr - memory_map::PPU_START) as usize % len] = value;
		}
	}
}

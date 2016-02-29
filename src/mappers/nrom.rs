use cartridge::Cartridge;
use memory_map;

// Simple non-banking ROM with some RAM.
// iNES mapper 000
// TODO document memory map!!
pub struct NRom {
	prg_rom: Vec<u8>,
	prg_mask: usize,
	chr_rom: Vec<u8>,
	ram: Vec<u8>,
	ram_mask: usize,
}

impl NRom {
	// TODO validate input!!! (ram size ...)
	pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, ram_size: usize) -> NRom {
		assert!(prg_rom.len() == 16 * 1024 || prg_rom.len() == 32 * 1024);
		assert!(ram_size % 0x400 == 0 && ram_size <= 0x2000);
		let prg_mask = prg_rom.len() - 1;
		NRom {
			prg_rom: prg_rom,
			prg_mask: prg_mask,
			chr_rom: chr_rom,
			ram: vec![0; ram_size],
			ram_mask: if ram_size == 0 { 0 } else { ram_size as usize - 1 },
		}
	}
}

impl Cartridge for NRom {
	fn read_cpu(&mut self, addr: u16) -> u8 {
		debug_assert!(addr >= memory_map::CARTRIDGE_START);
		if addr < 0x6000 {
			0
		} else if addr < 0x8000 {
			if self.ram_mask == 0 {
				0
			} else {
				self.ram[(addr as usize - 0x6000) & self.ram_mask]
			}
		} else {
			self.prg_rom[(addr as usize - 0x8000) & self.prg_mask]
		}
	}

	fn write_cpu(&mut self, addr: u16, value: u8) {
		debug_assert!(addr >= memory_map::CARTRIDGE_START);
		if addr < 0x6000 {
		} else if addr < 0x8000 {
			if self.ram_mask != 0 {
				self.ram[(addr as usize - 0x6000) & self.ram_mask] = value;
			}
		} else {
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use cartridge::Cartridge;

	#[test]
	fn unmapped() {
		let mut a = NRom::new(vec![0; 16 * 1024], vec![0; 8 * 1024], 0);
		a.write_cpu(0x5000, 123);
		assert_eq!(0, a.read_cpu(0x5000));
	}

	#[test]
	fn ram() {
		let mut a = NRom::new(vec![0; 16 * 1024], vec![0; 8 * 1024], 0);
		a.write_cpu(0x6001, 123);
		assert_eq!(0, a.read_cpu(0x6001));

		a = NRom::new(vec![0; 16 * 1024], vec![0; 8 * 1024], 0x800);
		a.write_cpu(0x6001, 123);
		assert_eq!(123, a.read_cpu(0x6001));
		assert_eq!(123, a.read_cpu(0x6801));
		assert_eq!(123, a.read_cpu(0x7001));
		assert_eq!(123, a.read_cpu(0x6801));

		a = NRom::new(vec![0; 16 * 1024], vec![0; 8 * 1024], 0x1000);
		a.write_cpu(0x6001, 123);
		assert_eq!(123, a.read_cpu(0x6001));
		assert_eq!(0, a.read_cpu(0x6801));
		assert_eq!(123, a.read_cpu(0x7001));
		assert_eq!(0, a.read_cpu(0x6801));

		a = NRom::new(vec![0; 16 * 1024], vec![0; 8 * 1024], 0x2000);
		a.write_cpu(0x6001, 123);
		assert_eq!(123, a.read_cpu(0x6001));
		assert_eq!(0, a.read_cpu(0x6801));
		assert_eq!(0, a.read_cpu(0x7001));
		assert_eq!(0, a.read_cpu(0x6801));
	}

	#[test]
	fn rom() {
		let mut rom = vec![123; 16 * 1024];
		rom[1] = 0;
		let mut a = NRom::new(rom, vec![0; 8 * 1024], 0);
		a.write_cpu(0x8001, 111);
		assert_eq!(0, a.read_cpu(0x8001));
		assert_eq!(123, a.read_cpu(0x8002));
		assert_eq!(0, a.read_cpu(0xC001));
		assert_eq!(123, a.read_cpu(0xC002));

		rom = vec![123; 32 * 1024];
		rom[1] = 0;
		a = NRom::new(rom, vec![0; 8 * 1024], 0);
		a.write_cpu(0x8001, 111);
		assert_eq!(0, a.read_cpu(0x8001));
		assert_eq!(123, a.read_cpu(0x8002));
		assert_eq!(123, a.read_cpu(0xC001));
		assert_eq!(123, a.read_cpu(0xC002));
	}
}

use cartridge::{Cartridge, MirrorMode};
use cpu::memory_map;
use std::clone::Clone;

// Simple non-banking ROM with some RAM.
// iNES mapper 000
pub struct NRom {
	prg_rom: Vec<u8>,
	prg_mask: usize,
	chr_rom: Vec<u8>,
	ram: Vec<u8>,
	ram_mask: usize,
	ppu_ram: [u8; 2048],
	mirror_mode: MirrorMode,
}

impl NRom {
	// TODO validate input!!! (ram size ...)
	pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, ram_size: usize, mirror_mode: MirrorMode) -> NRom {
		assert!(prg_rom.len() == 16 * 1024 || prg_rom.len() == 32 * 1024);
		assert!(ram_size % 0x400 == 0 && ram_size <= 0x2000);
		assert!(chr_rom.len() == 8 * 1024);
		let prg_mask = prg_rom.len() - 1;
		NRom {
			prg_rom: prg_rom,
			prg_mask: prg_mask,
			chr_rom: chr_rom,
			ram: vec![0; ram_size],
			ram_mask: if ram_size == 0 { 0 } else { ram_size as usize - 1 },
			ppu_ram: [0; 2048],
			mirror_mode: mirror_mode,
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
		}
	}

	fn read_ppu(&mut self, addr: u16) -> u8 {
		debug_assert!(addr <= 0x3EFF);
		if addr <= 0x1FFF {
			self.chr_rom[addr as usize]
		} else if addr <= 0x2FFF {
			self.ppu_ram[(addr as usize - 0x1000) & 0x7FF]
		} else {
			self.ppu_ram[(addr as usize - 0x2000) & 0x7FF]
		}
	}

	fn write_ppu(&mut self, addr: u16, value: u8) {
		debug_assert!(addr <= 0x3EFF);
		if addr <= 0x1FFF {
		} else if addr <= 0x2FFF {
			self.ppu_ram[(addr as usize - 0x1000) & 0x7FF] = value;
		} else {
			self.ppu_ram[(addr as usize - 0x2000) & 0x7FF] = value;
		}
	}

	fn mirror_mode(&self) -> MirrorMode {
		self.mirror_mode.clone()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use cartridge::{Cartridge, MirrorMode};

	#[test]
	fn unmapped() {
		let mut a = NRom::new(vec![0; 16 * 1024], vec![0; 8 * 1024], 0, MirrorMode::HorizontalMirroring);
		a.write_cpu(0x5000, 123);
		assert_eq!(0, a.read_cpu(0x5000));
	}

	#[test]
	fn ram() {
		let mut a = NRom::new(vec![0; 16 * 1024], vec![0; 8 * 1024], 0, MirrorMode::HorizontalMirroring);
		a.write_cpu(0x6001, 123);
		assert_eq!(0, a.read_cpu(0x6001));

		a = NRom::new(vec![0; 16 * 1024], vec![0; 8 * 1024], 0x800, MirrorMode::HorizontalMirroring);
		a.write_cpu(0x6001, 123);
		assert_eq!(123, a.read_cpu(0x6001));
		assert_eq!(123, a.read_cpu(0x6801));
		assert_eq!(123, a.read_cpu(0x7001));
		assert_eq!(123, a.read_cpu(0x6801));

		a = NRom::new(vec![0; 16 * 1024], vec![0; 8 * 1024], 0x1000, MirrorMode::HorizontalMirroring);
		a.write_cpu(0x6001, 123);
		assert_eq!(123, a.read_cpu(0x6001));
		assert_eq!(0, a.read_cpu(0x6801));
		assert_eq!(123, a.read_cpu(0x7001));
		assert_eq!(0, a.read_cpu(0x6801));

		a = NRom::new(vec![0; 16 * 1024], vec![0; 8 * 1024], 0x2000, MirrorMode::HorizontalMirroring);
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
		let mut a = NRom::new(rom, vec![0; 8 * 1024], 0, MirrorMode::HorizontalMirroring);
		a.write_cpu(0x8001, 111);
		assert_eq!(0, a.read_cpu(0x8001));
		assert_eq!(123, a.read_cpu(0x8002));
		assert_eq!(0, a.read_cpu(0xC001));
		assert_eq!(123, a.read_cpu(0xC002));

		rom = vec![123; 32 * 1024];
		rom[1] = 0;
		a = NRom::new(rom, vec![0; 8 * 1024], 0, MirrorMode::HorizontalMirroring);
		a.write_cpu(0x8001, 111);
		assert_eq!(0, a.read_cpu(0x8001));
		assert_eq!(123, a.read_cpu(0x8002));
		assert_eq!(123, a.read_cpu(0xC001));
		assert_eq!(123, a.read_cpu(0xC002));
	}

	#[test]
	fn ppu() {
		let mut chr = vec![0; 8 * 1024];
		chr[2] = 123;
		let mut a = NRom::new(vec![123; 16 * 1024], chr, 0, MirrorMode::HorizontalMirroring);

		a.write_ppu(0x0002, 42);
		assert_eq!(123, a.read_ppu(0x0002));

		a.write_ppu(0x2002, 2);
		a.write_ppu(0x3403, 3);
		assert_eq!(2, a.read_ppu(0x2002));
		assert_eq!(0, a.read_ppu(0x2402));
		assert_eq!(2, a.read_ppu(0x2802));
		assert_eq!(0, a.read_ppu(0x2C02));
		assert_eq!(0, a.read_ppu(0x2003));
		assert_eq!(3, a.read_ppu(0x2403));
		assert_eq!(0, a.read_ppu(0x2803));
		assert_eq!(3, a.read_ppu(0x2C03));
		assert_eq!(2, a.read_ppu(0x3002));
		assert_eq!(0, a.read_ppu(0x3402));
		assert_eq!(2, a.read_ppu(0x3802));
		assert_eq!(0, a.read_ppu(0x3C02));
		assert_eq!(0, a.read_ppu(0x3003));
		assert_eq!(3, a.read_ppu(0x3403));
		assert_eq!(0, a.read_ppu(0x3803));
		assert_eq!(3, a.read_ppu(0x3C03));
	}
}

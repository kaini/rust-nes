use cartridge::{Cartridge, MirrorMode};
use cpu::memory_map;

// Nintendo MMC1
// CPU:
//   6000-7FFF  PRG RAM (8 KiB)
//   8000-BFFF  PRG ROM (switchable/fixed to first)
//   C000-FFFF  PRG ROM (fixed to last/switchable)
// See http://wiki.nesdev.com/w/index.php/MMC1
pub struct Mmc1 {
	prg_rom: Vec<u8>,
	chr_rom: Vec<u8>,
	ram: Vec<u8>,
	control: u8,
	chr_bank0: u8,
	chr_bank1: u8,
	prg_bank: u8,
	shifter: u8,
	ppu_ram: [u8; 2048],
}

impl Mmc1 {
	// TODO validate input!!! (ram size ...)
	pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, ram_size: usize) -> Mmc1 {
		assert!(prg_rom.len() == 16 * 16 * 1024);
		assert!(chr_rom.len() == 128 * 1024);
		assert!(ram_size == 8 * 1024);
		Mmc1 {
			prg_rom: prg_rom,
			chr_rom: chr_rom,
			ram: vec![0; ram_size],
			control: 0x0C,
			chr_bank0: 0,
			chr_bank1: 0,
			prg_bank: 0,
			shifter: 0b00100000,
			ppu_ram: [0; 2048],
		}
	}
}

impl Cartridge for Mmc1 {
	fn read_cpu(&mut self, addr: u16) -> u8 {
		debug_assert!(addr >= memory_map::CARTRIDGE_START);
		if addr < 0x6000 {
			// not mapped
			0
		} else if addr < 0x8000 {
			// ram
			if self.prg_bank & 0b10000 == 0 {
				self.ram[addr as usize - 0x6000]
			} else {
				0
			}
		} else {
			// program rom
			match (self.control >> 2) & 0b11 {
				0 | 1 => {
					let bank = ((self.prg_bank >> 1) & 0b111) as usize;
					self.prg_rom[0x8000 * bank + addr as usize - 0x8000]
				},
				2 => {
					if addr < 0xC000 {
						self.prg_rom[addr as usize - 0x8000]
					} else {
						let bank = (self.prg_bank & 0b1111) as usize;
						self.prg_rom[0x4000 * bank + addr as usize - 0xC000]
					}
				},
				3 => {
					if addr < 0xC000 {
						let bank = (self.prg_bank & 0b1111) as usize;
						self.prg_rom[0x4000 * bank + addr as usize - 0x8000]
					} else {
						self.prg_rom[0x4000 * 15 + addr as usize - 0xC000]
					}
				},
				_ => { unreachable!() }
			}
		}
	}

	// TODO ugly write ignore stuff thingy (see docs ...)
	fn write_cpu(&mut self, addr: u16, value: u8) {
		debug_assert!(addr >= memory_map::CARTRIDGE_START);
		if addr < 0x6000 {
			// not mapped
		} else if addr < 0x8000 {
			// ram
			if self.prg_bank & 0b10000 == 0 {
				self.ram[addr as usize - 0x6000] = value;
			}
		} else {
			// load register
			if value & 0b10000000 != 0 {
				self.control |= 0x0C;
				self.shifter = 0b00100000;
			} else {
				self.shifter >>= 1;
				self.shifter |= (value & 1) << 7;
				if self.shifter & 1 == 1 {
					let result = self.shifter >> 3;
					self.shifter = 0b00100000;
					if addr < 0xA000 {
						// control
						self.control = result;
					} else if addr < 0xC000 {
						// chr bank 0
						self.chr_bank0 = result;
					} else if addr < 0xE000 {
						// chr bank 1
						self.chr_bank1 = result;
					} else {
						// prg bank
						self.prg_bank = result;
					}
				}
			}
		}
	}

	fn read_ppu(&mut self, addr: u16) -> u8 {
		debug_assert!(addr <= 0x3EFF);
		if addr <= 0x1FFF {
			if self.control & 0b10000 == 0 {
				// 8 KiB mode
				let bank = (self.chr_bank0 >> 1) as usize;
				self.chr_rom[bank * 8 * 1024 + addr as usize]
			} else {
				// 4 KiB mode
				if addr <= 0x0FFF {
					self.chr_rom[(self.chr_bank0 as usize) * 4 * 1024 + addr as usize]
				} else {
					self.chr_rom[(self.chr_bank1 as usize) * 4 * 1024 + addr as usize - 0x1000]
				}
			}
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
		unimplemented!()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use cartridge::Cartridge;

	#[test]
	fn unmapped() {
		let mut a = Mmc1::new(vec![0; 256 * 1024], vec![0; 128 * 1024], 0x2000);
		a.write_cpu(0x5000, 123);
		assert_eq!(0, a.read_cpu(0x5000));
	}

	#[test]
	fn ram() {
		let mut a = Mmc1::new(vec![0; 256 * 1024], vec![0; 128 * 1024], 0x2000);
		a.write_cpu(0x6001, 123);
		assert_eq!(123, a.read_cpu(0x6001));

		// disable RAM
		a.write_cpu(0x8000, 0);
		a.write_cpu(0x8000, 0);
		a.write_cpu(0x8000, 0);
		a.write_cpu(0x8000, 0);
		a.write_cpu(0xE000, 1);
		assert_eq!(0, a.read_cpu(0x6001));
		a.write_cpu(0x6001, 111);

		// enable RAM
		a.write_cpu(0x8000, 0);
		a.write_cpu(0x8000, 0);
		a.write_cpu(0x8000, 0);
		a.write_cpu(0x8000, 0);
		a.write_cpu(0xE000, 0);
		assert_eq!(123, a.read_cpu(0x6001));
	}

	#[test]
	fn rom() {
		let mut rom = vec![255; 256 * 1024];
		for i in 0..16 {
			rom[i * 16 * 1024 + 1] = i as u8;
		}
		let mut a = Mmc1::new(rom, vec![0; 128 * 1024], 0x2000);

		// 32 switch mode
		for i in 0..2 {
			a.write_cpu(0x8001, 0);
			a.write_cpu(0x8001, 0);
			a.write_cpu(0x8001, i);
			a.write_cpu(0x8001, 0);
			a.write_cpu(0x8001, 0);
			for j in 0..16 {
				a.write_cpu(0xE000, j);
				a.write_cpu(0xE000, j >> 1);
				a.write_cpu(0xE000, j >> 2);
				a.write_cpu(0xE000, j >> 3);
				a.write_cpu(0xE000, 0);
				assert_eq!((j / 2) * 2, a.read_cpu(0x8001));
				assert_eq!((j / 2) * 2 + 1, a.read_cpu(0xC001));
			}
		}

		// fix first, 16 switch
		a.write_cpu(0x8001, 0);
		a.write_cpu(0x8001, 0);
		a.write_cpu(0x8001, 0);
		a.write_cpu(0x8001, 1);
		a.write_cpu(0x8001, 0);
		for i in 0..16 {
			a.write_cpu(0xE000, i);
			a.write_cpu(0xE000, i >> 1);
			a.write_cpu(0xE000, i >> 2);
			a.write_cpu(0xE000, i >> 3);
			a.write_cpu(0xE000, 0);
			assert_eq!(0, a.read_cpu(0x8001));
			assert_eq!(i, a.read_cpu(0xC001));
		}

		// fix last, 16 switch
		a.write_cpu(0x8001, 0);
		a.write_cpu(0x8001, 0);
		a.write_cpu(0x8001, 1);
		a.write_cpu(0x8001, 1);
		a.write_cpu(0x8001, 0);
		for i in 0..16 {
			a.write_cpu(0xE000, i);
			a.write_cpu(0xE000, i >> 1);
			a.write_cpu(0xE000, i >> 2);
			a.write_cpu(0xE000, i >> 3);
			a.write_cpu(0xE000, 0);
			assert_eq!(i, a.read_cpu(0x8001));
			assert_eq!(15, a.read_cpu(0xC001));
		}
	}

	#[test]
	fn ppu_ram() {
		let mut a = Mmc1::new(vec![123; 256 * 1024], vec![0; 128 * 1024], 0x2000);
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

	#[test]
	fn ppu_rom() {
		let mut rom = vec![123; 128 * 1024];
		for i in 0..32 {
			rom[i * 4 * 1024 + 2] = i as u8;
		}

		let mut a = Mmc1::new(vec![123; 256 * 1024], rom, 0x2000);
		
		// 8 switch mode
		a.write_cpu(0x8001, 0);
		a.write_cpu(0x8001, 0);
		a.write_cpu(0x8001, 0);
		a.write_cpu(0x8001, 0);
		a.write_cpu(0x8001, 0);
		for i in 0..32 {
			a.write_cpu(0x8001, i >> 0);
			a.write_cpu(0x8001, i >> 1);
			a.write_cpu(0x8001, i >> 2);
			a.write_cpu(0x8001, i >> 3);
			a.write_cpu(0xA001, i >> 4);
			for j in 0..32 {
				a.write_cpu(0x8001, j >> 0);
				a.write_cpu(0x8001, j >> 1);
				a.write_cpu(0x8001, j >> 2);
				a.write_cpu(0x8001, j >> 3);
				a.write_cpu(0xC001, j >> 4);
				assert_eq!(i / 2 * 2, a.read_ppu(0x0002));
				assert_eq!(i / 2 * 2 + 1, a.read_ppu(0x1002));
			}
		}

		// 4 switch mode
		a.write_cpu(0x8001, 0);
		a.write_cpu(0x8001, 0);
		a.write_cpu(0x8001, 0);
		a.write_cpu(0x8001, 0);
		a.write_cpu(0x8001, 1);
		for i in 0..32 {
			a.write_cpu(0x8001, i >> 0);
			a.write_cpu(0x8001, i >> 1);
			a.write_cpu(0x8001, i >> 2);
			a.write_cpu(0x8001, i >> 3);
			a.write_cpu(0xA001, i >> 4);
			for j in 0..32 {
				a.write_cpu(0x8001, j >> 0);
				a.write_cpu(0x8001, j >> 1);
				a.write_cpu(0x8001, j >> 2);
				a.write_cpu(0x8001, j >> 3);
				a.write_cpu(0xC001, j >> 4);
				assert_eq!(i, a.read_ppu(0x0002));
				assert_eq!(j, a.read_ppu(0x1002));
			}
		}
	}
}

// Size of the internal RAM.
pub const RAM_SIZE: u16 = 2048;
// Number of PPU registers à 1 byte.
pub const PPU_SIZE: u16 = 8;
// Number of APU and IO registers à 1 byte.
pub const APU_IO_SIZE: u16 = 32;
// Size of the cartridge area.
pub const CARTRIDGE_SIZE: u16 = 49120;

// Start address of RAM.
pub const RAM_START: u16 = 0;
// Start address of PPU registers.
pub const PPU_START: u16 = RAM_START + 4 * RAM_SIZE;
// Start of API and IO registers.
pub const APU_IO_START: u16 = PPU_START + 1024 * PPU_SIZE;
// Start of cartridge space.
pub const CARTRIDGE_START: u16 = APU_IO_START + 1 * APU_IO_SIZE;

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn constants() {
		assert_eq!(0, RAM_START);
		assert!(RAM_START < PPU_START);
		assert!(PPU_START < APU_IO_START);
		assert!(APU_IO_START < CARTRIDGE_START);
		assert_eq!(0, CARTRIDGE_START.wrapping_add(CARTRIDGE_SIZE));
	}
}

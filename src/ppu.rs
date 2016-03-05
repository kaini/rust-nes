use cpu::memory_map;
use cartridge::Cartridge;

pub trait PpuOutput {
	fn set_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8);
}

// http://wiki.nesdev.com/w/index.php/PPU_registers et al.
pub struct Ppu {
	// PPUCTRL
	nmi_enable: bool,
	ppu_master: bool,
	sprite_height: bool,
	background_tile_select: bool,
	sprite_tile_select: bool,
	increment_mode: bool,
	
	// PPUMASK
	color_emph_b: bool,
	color_emph_g: bool,
	color_emph_r: bool,
	sprite_enable: bool,
	background_enable: bool,
	sprite_left_column_enable: bool,
	background_left_column_enable: bool,
	greyscale: bool,

	// PPUSTATUS
	vblank: bool,
	sprite_0_hit: bool,
	sprite_overflow: bool,
	status_artifact: u8,

	// OAMADDR
	oamaddr: u8,

	// Internal Registers
	current_vram_address: u16, // only 15 bit used
	temp_vram_address: u16,    // only 15 bit used
	fine_x_scroll: u8,         // only 3 bit used
	write_toggle: bool,

	// Internal RAM
	oam: [u8; 256],
	palette: [u8; 256],
	
	// Render state
	current_scanline: usize,
	current_cycle: usize,
	current_nametable_byte: u8,
	current_attributetable_byte: u8,
	current_tilebitmap_low: u8,
	current_tilebitmap_high: u8,
}

impl Ppu {
	pub fn new() -> Ppu {
		Ppu {
			nmi_enable: false,
			ppu_master: false,
			sprite_height: false,
			background_tile_select: false,
			sprite_tile_select: false,
			increment_mode: false,
			color_emph_b: false,
			color_emph_g: false,
			color_emph_r: false,
			sprite_enable: false,
			background_enable: false,
			sprite_left_column_enable: false,
			background_left_column_enable: false,
			greyscale: false,
			vblank: false,
			sprite_0_hit: false,
			sprite_overflow: false,
			status_artifact: 0,
			oamaddr: 0,
			current_vram_address: 0,
			temp_vram_address: 0,
			fine_x_scroll: 0,
			write_toggle: false,
			oam: [0; 256],
			palette: [0; 256],
			current_scanline: 261,
			current_cycle: 0,
			current_nametable_byte: 0,
			current_attributetable_byte: 0,
			current_tilebitmap_low: 0,
			current_tilebitmap_high: 0,
		}
	}

	pub fn read(&mut self, cartridge: &mut Cartridge, addr: u16) -> u8 {
		debug_assert!(memory_map::PPU_START <= addr && addr < memory_map::APU_IO_START);
		let result = match addr {
			0x2002 => {
				self.write_toggle = false;
				(
					(self.status_artifact   & 0b00011111)             |
					if self.sprite_overflow { 0b00100000 } else { 0 } |
					if self.sprite_0_hit    { 0b01000000 } else { 0 } |
					if self.vblank          { 0b10000000 } else { 0 }
				)
			}
			0x2004 => {
				// oam read
				// TODO other oddities while rendering
				self.oam[self.oamaddr as usize]
			}
			0x2007 => {
				// ppu read
				// TODO other oddities while rendering
				let result = self.read_ppu(cartridge, self.current_vram_address);
				self.current_vram_address += if self.increment_mode { 32 } else { 1 };
				self.current_vram_address &= 0x3FFF;
				result
			}
			0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x2006 => {
				self.status_artifact
			}
			_ => { unreachable!() }
		};
		self.status_artifact = result;
		result
	}

	pub fn write(&mut self, cartridge: &mut Cartridge, addr: u16, value: u8) {
		debug_assert!(memory_map::PPU_START <= addr && addr < memory_map::APU_IO_START);
		match addr {
			0x2000 => {
				self.nmi_enable             = value & 0b10000000 != 0;
				self.ppu_master             = value & 0b01000000 != 0;
				self.sprite_height          = value & 0b00100000 != 0;
				self.background_tile_select = value & 0b00010000 != 0;
				self.sprite_tile_select     = value & 0b00001000 != 0;
				self.increment_mode         = value & 0b00000100 != 0;
				self.temp_vram_address      = (value as u16 & 0b00000011) << 10;
			}
			0x2001 => {
				self.color_emph_b                  = value & 0b10000000 != 0;
				self.color_emph_g                  = value & 0b01000000 != 0;
				self.color_emph_r                  = value & 0b00100000 != 0;
				self.sprite_enable                 = value & 0b00010000 != 0;
				self.background_enable             = value & 0b00001000 != 0;
				self.sprite_left_column_enable     = value & 0b00000100 != 0;
				self.background_left_column_enable = value & 0b00000010 != 0;
				self.greyscale                     = value & 0b00000001 != 0;
			}
			0x2002 => {
				// read only
			}
			0x2003 => {
				self.oamaddr = value;
			}
			0x2004 => {
				// TODO ignore writes during rendering and other oddities
				// oam write
				self.oam[self.oamaddr as usize] = value;
				self.oamaddr = self.oamaddr.wrapping_add(1);
			}
			0x2005 => {
				if self.write_toggle {
					self.temp_vram_address &= !0b1110011_11100000;
					self.temp_vram_address |= (value as u16 >> 3) << 5;
					self.temp_vram_address |= (value as u16 & 0b111) << 12;
					self.write_toggle = false;
				} else {
					self.temp_vram_address &= !0b11111;
					self.temp_vram_address |= value as u16 >> 3;
					self.fine_x_scroll = value & 0b111;
					self.write_toggle = true;
				}
			}
			0x2006 => {
				if self.write_toggle {
					self.temp_vram_address &= !0xFF;
					self.temp_vram_address |= value as u16;
					self.current_vram_address = self.temp_vram_address;
					self.write_toggle = false;
				} else {
					self.temp_vram_address &= !0xFF00;
					self.temp_vram_address |= (value as u16 & 0b111111) << 8;
					self.write_toggle = true;
				}
			}
			0x2007 => {
				// ppu write
				// TODO special behavior if write is during lines 0-239.
				let write_addr = self.current_vram_address;
				self.write_ppu(cartridge, write_addr, value);
				self.current_vram_address += if self.increment_mode { 32 } else { 1 };
				self.current_vram_address &= 0x3FFF;
			}
			_ => { unreachable!(); }
		}
		self.status_artifact = value;
	}

	fn read_ppu(&self, cartridge: &mut Cartridge, addr: u16) -> u8 {
		debug_assert!(addr <= 0x3FFF);
		if addr <= 0x3EFF {
			cartridge.read_ppu(addr)
		} else {
			match addr {
				0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => { self.palette[(addr - 0x3F00 - 0x10) as usize] }
				_ => { self.palette[(addr - 0x3F00) as usize] }
			}
		}
	}

	fn write_ppu(&mut self, cartridge: &mut Cartridge, addr: u16, value: u8) {
		debug_assert!(addr <= 0x3FFF);
		if addr <= 0x3EFF {
			cartridge.write_ppu(addr, value);
		} else {
			match addr {
				0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => {
					self.palette[(addr - 0x3F00 - 0x10) as usize] = value & 0b00111111;
				}
				_ => {
					self.palette[(addr - 0x3F00) as usize] = value & 0b00111111;
				}
			}
		}
	}

	pub fn tick(&mut self, cartridge: &mut Cartridge, output: &mut PpuOutput) {
		if self.current_scanline == 261 {
			self.tick_prerender_scanline();
		} else if self.current_scanline <= 239 {
			self.tick_visible_scanline(cartridge, output);
		} else if self.current_scanline == 240 {
			self.tick_postrender_scanline();
		} else if self.current_scanline <= 260 {
			self.tick_vblank_scanline();
		} else {
			unreachable!();
		}
	}

	fn tick_prerender_scanline(&mut self) {
		// TODO prefetching... simulated access...
		if self.current_cycle == 1 {
			self.vblank = false;
		}

		if self.current_cycle == 340 {
			self.current_scanline = 0;
			self.current_cycle = 0;
		} else {
			self.current_cycle += 1;
		}
	}

	fn tick_visible_scanline(&mut self, cartridge: &mut Cartridge, output: &mut PpuOutput) {
		// TODO each cycle one pixel (optimization potential!)
		if self.current_cycle == 0 {
			// do nothing
		} else if self.current_cycle <= 256 {
			// fetch tiles for this scanline
			let y = self.current_scanline;
			let tile_y = y / 8;
			let in_tile_y = y % 8;
			let tile_x = (self.current_cycle - 1) / 8;
			debug_assert!(y < 240 + 1);
			debug_assert!(tile_y < 30 + 1);
			debug_assert!(tile_x < 32);

			// TODO mirroring
			match self.current_cycle % 8 {
				1 => {
					// draw
					if !(y == 0 && tile_x == 0) {
						if tile_x == 0 { self.draw_8x1(256 - 8       , y - 1, output) }
						else           { self.draw_8x1(tile_x * 8 - 8, y    , output) };
					}
				}
				2 => {
					self.current_nametable_byte =
						self.read_ppu(cartridge, (0x2000 + tile_y * 32 + tile_x) as u16);
				}
				3 => {}
				4 => {
					self.current_attributetable_byte =
						self.read_ppu(cartridge, (0x23C0 + (tile_y * 32 + tile_x) / 4) as u16);
				}
				5 => {}
				6 => {
					// TODO when to use 0x1000+?
					self.current_tilebitmap_low =
						self.read_ppu(cartridge, (self.current_nametable_byte as usize * 16 + in_tile_y) as u16);
				}
				7 => {}
				0 => {
					// TODO when to use 0x1000+?
					self.current_tilebitmap_high =
						self.read_ppu(cartridge, (self.current_nametable_byte as usize * 16 + in_tile_y + 8) as u16);
					// TODO inc hori(v)
				}
				_ => { unreachable!(); }
			}
		} else if self.current_cycle == 257 {
			// final draw cycle
			self.draw_8x1(256 - 8, 239, output);
			// TODO hori(v) = hori(t)
		} else if self.current_cycle <= 320 {
			// fetch sprites for next scanline
			// TODO
		} else if self.current_cycle <= 336 {
			// fetch two tiles for next scanline
			// TODO
		} else if self.current_cycle <= 340 {
			// unknown fetches
			// TODO
		}

		if self.current_cycle == 340 {
			self.current_scanline += 1;
			self.current_cycle = 0;
		} else {
			self.current_cycle += 1;
		}
	}

	fn tick_postrender_scanline(&mut self) {
		if self.current_cycle == 340 {
			self.current_scanline += 1;
			self.current_cycle = 0;
		} else {
			self.current_cycle += 1;
		}
	}

	fn tick_vblank_scanline(&mut self) {
		if self.current_scanline == 241 && self.current_cycle == 1 {
			self.vblank = true;
		}
		if self.current_cycle == 260 {
			self.current_scanline += 1;
			self.current_cycle = 0;
		} else {
			self.current_cycle += 1;
		}
	}

	fn draw_8x1(&self, x: usize, y: usize, output: &mut PpuOutput) {
		// extract attribute table value
		let attribute_value = 0b11 &
			if x % 32 < 16 {
				// left
				if y % 32 < 16 {
					// top
					self.current_attributetable_byte >> 0
				} else {
					// bottom
					self.current_attributetable_byte >> 4
				}
			} else {
				// right
				if y % 32 < 16 {
					// top
					self.current_attributetable_byte >> 2
				} else {
					// bottom
					self.current_attributetable_byte >> 6
				}
			};

		for i in 0..8 {
			let color_index =
				(((self.current_tilebitmap_high & (1 << (7 - i))) >> (7 - i)) << 1) |
				((self.current_tilebitmap_low & (1 << (7 - i))) >> (7 - i)) |
				(attribute_value << 2);
			let color =
				if color_index & 0b11 == 0 {
					self.palette[0]
				} else {
					self.palette[color_index as usize]
				};
			let (r, g, b) = (
				RGB_PALETTE[color as usize * 3],
				RGB_PALETTE[color as usize * 3 + 1],
				RGB_PALETTE[color as usize * 3 + 2]);

			output.set_pixel(x + i, y, r, g, b);
		}
	}
}

// TODO real color?
// Generated with http://bisqwit.iki.fi/utils/nespalette.php
const RGB_PALETTE: [u8; 64 * 3] = [
	0x52, 0x52, 0x52, 0x01, 0x1a, 0x51, 0x0f, 0x0f, 0x65, 0x23, 0x06, 0x63, 0x36, 0x03, 0x4b, 0x40,
	0x04, 0x26, 0x3f, 0x09, 0x04, 0x32, 0x13, 0x00, 0x1f, 0x20, 0x00, 0x0b, 0x2a, 0x00, 0x00, 0x2f,
	0x00, 0x00, 0x2e, 0x0a, 0x00, 0x26, 0x2d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0xa0, 0xa0, 0xa0, 0x1e, 0x4a, 0x9d, 0x38, 0x37, 0xbc, 0x58, 0x28, 0xb8, 0x75, 0x21, 0x94, 0x84,
	0x23, 0x5c, 0x82, 0x2e, 0x24, 0x6f, 0x3f, 0x00, 0x51, 0x52, 0x00, 0x31, 0x63, 0x00, 0x1a, 0x6b,
	0x05, 0x0e, 0x69, 0x2e, 0x10, 0x5c, 0x68, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0xfe, 0xff, 0xff, 0x69, 0x9e, 0xfc, 0x89, 0x87, 0xff, 0xae, 0x76, 0xff, 0xce, 0x6d, 0xf1, 0xe0,
	0x70, 0xb2, 0xde, 0x7c, 0x70, 0xc8, 0x91, 0x3e, 0xa6, 0xa7, 0x25, 0x81, 0xba, 0x28, 0x63, 0xc4,
	0x46, 0x54, 0xc1, 0x7d, 0x56, 0xb3, 0xc0, 0x3c, 0x3c, 0x3c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0xfe, 0xff, 0xff, 0xbe, 0xd6, 0xfd, 0xcc, 0xcc, 0xff, 0xdd, 0xc4, 0xff, 0xea, 0xc0, 0xf9, 0xf2,
	0xc1, 0xdf, 0xf1, 0xc7, 0xc2, 0xe8, 0xd0, 0xaa, 0xd9, 0xda, 0x9d, 0xc9, 0xe2, 0x9e, 0xbc, 0xe6,
	0xae, 0xb4, 0xe5, 0xc7, 0xb5, 0xdf, 0xe4, 0xa9, 0xa9, 0xa9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

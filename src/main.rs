mod cartridge;
mod cpu;
mod memory;
mod instructions;
mod memory_map;
mod ppu;
mod mappers;

use cartridge::load_rom;
use cpu::Cpu;
use std::env;
use std::borrow::Borrow;
use std::io::{stderr, Write};

fn main() {
	println!("+---------------------------+");
	println!("| Kaini's Rust NES Emulator |");
	println!("+---------------------------+");
	
	let mut rom_path = String::new();
	let mut i = 0;
	for arg in env::args() {
		match i {
			1 => rom_path = arg,
			_ => (),
		}
		i += 1;
	}
	if rom_path.is_empty() {
		println!("Missing first argument: Path to ROM file.");
		return;
	}

	println!("Loading ROM {}.", rom_path);
	let rom = match load_rom(rom_path.borrow()) {
		Ok(rom) => rom,
		Err(err) => { println!("Could not load ROM: {}", err); return; }
	};

	let mut instr_log_file = stderr();
	let mut instr_log = Option::Some(&mut instr_log_file as &mut Write);

	let mut cpu = Cpu::new(rom);
	for _ in 0..10 {
		cpu.tick(&mut instr_log);
	}
}

#[cfg(test)]
mod test {
	use cartridge::load_rom;
	use std::io::{Write, Read, BufWriter};
	use std::fs::File;
	use cpu::Cpu;

	#[test]
	fn nestest_rom() {
		// Execute ROM
		let rom = load_rom("roms/nestest.nes").unwrap();
		let mut log_buffer = Vec::new();
		let mut cpu = Cpu::new(rom);
		cpu.registers_mut().pc = 0xC000;
		{
			let mut instr_log = Option::Some(&mut log_buffer as &mut Write);
			for _ in 0..8992 {
				cpu.tick(&mut instr_log);
			}
		}
		let my_log = String::from_utf8(log_buffer).unwrap();

		// Load reference log
		let mut ref_log = String::new();
		File::open("roms/nestest.log").unwrap().read_to_string(&mut ref_log).unwrap();

		// Compare logs
		let mut my_lines = my_log.lines();
		let mut line_no = 0;
		for ref_line_str in ref_log.lines() {
			let my_line = my_lines.next().unwrap();
			line_no += 1;
			println!("{:4} MY   {}", line_no, my_line);

			let branch_syntax =  // handle special #$+ and #$- syntax
				my_line.find("#$+").is_some() ||
				my_line.find("#$-").is_some();

			let mut ref_line = String::new();
			let mut cmd_remove = false;  // true when we remove extra info after the opcode
			for (i, c) in ref_line_str.char_indices() {
				if i < 73 {  // use whole string
					if branch_syntax && 17 <= i && i < 48 {
						ref_line.push(my_line.chars().nth(i).unwrap());
					} else if cmd_remove && i < 48 {
						ref_line.push(' ');
					} else if c == '=' || c == '@' {
						cmd_remove = true;
						ref_line.push(' ');
					} else if c == '*' {
						ref_line.push(' ');
					} else {
						ref_line.push(c);
					}
				} else {
					break;
				}
			}

			if ref_line != my_line {
				println!("{:4} REF  {}", line_no, ref_line_str);
				assert!(false);
			}
		}
	}

	macro_rules! gblargg_test_rom {
		($test_name:ident, $rom_name:expr) => {
			#[test]
			fn $test_name() {
				// load
				let rom = load_rom(&format!("roms/{}.nes", $rom_name)).unwrap();
				let mut log_buffer = BufWriter::new(File::create(format!("logs/{}.log", $rom_name)).unwrap());
				let instr_log = &mut Option::Some(&mut log_buffer as &mut Write);
				
				// execute
				let mut cpu = Cpu::new(rom);
				cpu.write_memory(0x6000, 0x80);
				cpu.write_memory(0x6004, 0);
				while cpu.read_memory(0x6000) == 0x80 {
					cpu.tick(instr_log);
				}

				// read message
				let mut message = Vec::new();
				let mut addr = 0x6004;
				loop {
					let byte = cpu.read_memory(addr);
					addr += 1;
					if byte == 0 {
						break;
					}
					message.push(byte);
				}
				println!("{}", String::from_utf8(message).unwrap());
				
				// check
				assert_eq!(0, cpu.read_memory(0x6000));
			}
		}
	}

	gblargg_test_rom!(basics_rom, "01-basics");
	gblargg_test_rom!(implied_rom, "02-implied");
	gblargg_test_rom!(immediate_rom, "03-immediate");
	gblargg_test_rom!(zero_page_rom, "04-zero_page");
	gblargg_test_rom!(zp_xy_rom, "05-zp_xy");
	gblargg_test_rom!(absolute_rom, "06-absolute");
	gblargg_test_rom!(abs_xy_rom, "07-abs_xy");
	gblargg_test_rom!(ind_x_rom, "08-ind_x");
	gblargg_test_rom!(ind_y_rom, "09-ind_y");
	gblargg_test_rom!(branches_rom, "10-branches");
	gblargg_test_rom!(stack_rom, "11-stack");
	gblargg_test_rom!(jmp_jsr_rom, "12-jmp_jsr");
	gblargg_test_rom!(rts_rom, "13-rts");
	gblargg_test_rom!(rti_rom, "14-rti");
	gblargg_test_rom!(brk_rom, "15-brk");
	gblargg_test_rom!(special_rom, "16-special");
}

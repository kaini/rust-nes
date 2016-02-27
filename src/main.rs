mod rom;
mod cpu;
mod memory;
mod instructions;
mod memory_map;

use rom::Rom;
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
	let rom = match Rom::load(rom_path.borrow()) {
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
	use rom::Rom;
	use std::io::{Write, Read};
	use std::fs::File;
	use cpu::Cpu;

	#[test]
	fn nestest_rom() {
		// Execute ROM
		let rom = Rom::load("roms/nestest.nes").unwrap();
		let mut log_buffer = Vec::new();
		let mut cpu = Cpu::new(rom);
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
}

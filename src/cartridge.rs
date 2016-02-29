use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::io;
use std::borrow::Borrow;
use mappers;

pub trait Cartridge {
	fn read_cpu(&mut self, addr: u16) -> u8;
	fn write_cpu(&mut self, addr: u16, value: u8);
}

pub fn load_rom(path: &str) -> Result<Box<Cartridge>, &'static str> {
	let mut file = match File::open(path) {
		Ok(file) => file,
		Err(_) => return Result::Err("Could not open file."),
	};
	let mut header = [0; 4];
	match file.read_exact(&mut header) {
		Ok(_) => (),
		Err(_) => return Result::Err("Could not read file."),
	}
	if header == [0x4E, 0x45, 0x53, 0x1A] {
		println!("Loading iNES file.");
		match load_ines(&mut file) {
			Ok(rom) => Result::Ok(rom),
			Err(_) => Result::Err("Could not read file."),
		}
	} else {
		Result::Err("Unknown file format.")
	}
}

fn load_ines(file: &mut File) -> io::Result<Box<Cartridge>> {
	let mut header = [0; 16];
	try!(file.seek(SeekFrom::Start(0)));
	try!(file.read_exact(&mut header));
	for byte in header.iter() {
		print!("{:02X} ", byte);
	}
	println!("");

	let prg_size = (header[4] as usize) * 16 * 1024;

	let chr_size = (header[5] as usize) * 8 * 1024;

	let flags6 = header[6];
	let mirror_mode =
		if flags6 & 0b1000 != 0 { MirrorMode::FourScreen }
		else if flags6 & 1 == 0 { MirrorMode::HorizontalMirroring }
		else { MirrorMode::VerticalMirroring };
	let persistent = flags6 & 0b10 != 0;
	let trainer = flags6 & 0b100 != 0;
	let mut mapper = flags6 >> 4;
	if trainer {
		return parse_error("ROM contains trainer, this is not implemented yet.");
	}

	let flags7 = header[7];
	mapper |= flags7 & 0xF0;
	let vs_unisystem = flags7 & 1 != 0;
	let file_format = (flags7 & 0b1100) >> 2;
	if vs_unisystem {
		return parse_error("VS Unisystem ROMs not supported.");
	}
	if file_format != 0 {
		return parse_error(format!("Unsupported iNES file format: {}", file_format).borrow());
	}

	let ram_size =
		if header[8] == 0 { 8 * 1024 }
		else { (header[8] as usize) * 8 * 1024 }; 

	if header[9] != 1 && header[9] != 0 {
		return parse_error("Header byte 9 invalid.");
	}

	// ignore flag 10

	for i in 11..16 {
		if header[i] != 0 {
			return parse_error(format!("Unsupported ROM: Byte {} is not zero.", i).borrow());
		}
	}

	let mut prg_rom = vec![0; prg_size];
	try!(file.read_exact(&mut prg_rom[..]));
	let mut chr_rom = vec![0; chr_size];
	try!(file.read_exact(&mut chr_rom[..]));

	println!("Mapper: {:03}  PRG ROM: {} KiB  PRG RAM: {} KiB  CHR: {} KiB", 
		mapper, prg_size / 1024, ram_size / 1024, chr_size / 1024);
	println!("Mirror: {:?}  Persistent: {}  Trainer: {}",
		mirror_mode, persistent, trainer);

	match mapper {
		000 => Result::Ok(Box::new(mappers::NRom::new(prg_rom, chr_rom, ram_size))),
		001 => Result::Ok(Box::new(mappers::Mmc1::new(prg_rom, chr_rom, ram_size))),
		_   => parse_error(format!("Unsupported ROM mapper {:03}.", mapper).borrow()),
	}
}

#[derive(Debug)]
enum MirrorMode {
	HorizontalMirroring,
	VerticalMirroring,
	FourScreen,
}

fn parse_error<T>(error: &str) -> io::Result<T> {
	println!("{}", error);
	Result::Err(io::Error::new(io::ErrorKind::Other, ""))
}

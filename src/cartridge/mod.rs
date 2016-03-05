mod nrom;
mod mmc1;
pub mod cartridge;  // TODO REMOVE RUST BUG!!!!

pub use cartridge::cartridge::{Cartridge, MirrorMode, load_rom};

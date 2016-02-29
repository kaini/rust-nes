use memory_map;

pub struct Memory {
	ram: [u8; memory_map::RAM_SIZE as usize],
}

impl Memory {
    pub fn new() -> Memory {
    	Memory { ram: [0; memory_map::RAM_SIZE as usize] }
    }

    pub fn read(&self, addr: u16) -> u8 {
    	debug_assert!(memory_map::RAM_START <= addr && addr < memory_map::PPU_START);
    	self.ram[(addr % memory_map::RAM_SIZE) as usize]
    }

    pub fn write(&mut self, addr: u16, value: u8) {
    	debug_assert!(memory_map::RAM_START <= addr && addr < memory_map::PPU_START);
    	self.ram[(addr % memory_map::RAM_SIZE) as usize] = value;
    }
}


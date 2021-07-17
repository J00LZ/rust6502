pub trait Device {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, data: u8);
}

impl Device for [u8; 65536] {
    fn read(&self, address: u16) -> u8 {
        self[address as usize]
    }

    fn write(&mut self, address: u16, data: u8) {
        self[address as usize] = data;
    }
}

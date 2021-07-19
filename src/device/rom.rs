use std::fs;

use super::{CreateError, Device, WriteError};
use std::cmp::min;

pub struct Rom {
    start: u16,
    data: Vec<u8>,
}

impl Rom {
    pub fn new_file(start: u16, file: &str) -> Result<Self, CreateError> {
        let data = fs::read(file)?.into();
        Ok(Self { start, data })
    }
}

impl Device for Rom {
    fn read(&mut self, address: u16) -> Option<u8> {
        let cc = min(self.start as usize + self.data.len(), 0xFFFF) as u16;
        // println!(
        //     "min: {:#06X}, max: {:#06X}, r = {:#06X}",
        //     self.start, cc, address
        // );
        if address < self.start || address >= cc {
            None
        } else {
            Some(self.data[(address - self.start) as usize])
        }
    }

    fn write(&mut self, _: u16, _: u8) -> Result<(), WriteError> {
        Err(WriteError::NotWritable)
    }
}

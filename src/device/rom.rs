use std::fs;

use super::{CreateError, Device, WriteError};
use std::cmp::min;

pub struct Rom {
    start: u16,
    data: Vec<u8>,
}

impl Rom {
    pub fn new_file(start: u16, file: &str) -> Result<Self, CreateError> {
        let data = fs::read(file)?;
        Ok(Self { start, data })
    }
    pub fn from_vec(start: u16, data: Vec<u8>) -> Self {
        Self { start, data }
    }
    pub fn interrupts(nmi: u16, reset: u16, irq: u16) -> Self {
        Self::from_vec(
            0xFFFA,
            vec![
                nmi as u8,          // nmi low
                (nmi >> 8) as u8,   // nmi high
                reset as u8,        // reset low
                (reset >> 8) as u8, // reset high
                irq as u8,          // irq low
                (irq >> 8) as u8,   // irq high
            ],
        )
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

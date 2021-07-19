use super::{Device, WriteError};
use std::cmp::min;

pub struct Ram {
    start: u16,
    pub(crate) data: Vec<u8>,
}

impl Ram {
    pub fn new(start: u16, size: u16) -> Self {
        Self {
            start,
            data: vec![0; size as usize],
        }
    }
}

impl Device for Ram {
    fn read(&mut self, address: u16) -> Option<u8> {
        let cc = self.start + min(self.data.len() as u16, 0xFFFF);
        if address < self.start || address >= cc {
            None
        } else {
            self.data.get((address - self.start) as usize).copied()
        }
    }

    fn write(&mut self, address: u16, data: u8) -> Result<(), WriteError> {
        if address < self.start || address >= self.start + (self.data.len() as u16) {
            Err(WriteError::InvalidAddress)
        } else {
            self.data[(address - self.start) as usize] = data;
            Ok(())
        }
    }
}

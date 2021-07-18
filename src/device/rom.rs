use std::error::Error;
use std::fs;
use crate::device::{CreateError, Device};

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
    fn read(&self, address: u16) -> Option<u8> {
        if address < self.start || self.start + (self.data.len() as u16) >= address {
            None
        } else {
            Some(self.data[(address - self.start) as usize])
        }
    }

    fn write(&mut self, _: u16, _: u8) -> Result<(), super::WriteError> {
        Err(super::WriteError::NotWritable)
    }
}

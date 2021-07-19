use std::sync::{Arc, Mutex};

use thiserror::Error;

pub use ram::Ram;
pub use rom::Rom;

pub mod device_map;
pub mod ram;
pub mod rom;
pub mod vga;

#[derive(Debug, Error, Eq, PartialEq, Copy, Clone)]
pub enum WriteError {
    #[error("not writable")]
    NotWritable,
    #[error("invalid address")]
    InvalidAddress,
}

#[derive(Debug, Error)]
pub enum CreateError {
    #[error("filesystem error: {0}")]
    FsError(#[from] std::io::Error),
}

pub trait Device {
    fn read(&mut self, address: u16) -> Option<u8>;
    fn write(&mut self, address: u16, data: u8) -> Result<(), WriteError>;
}

impl<const N: usize> Device for [u8; N] {
    fn read(&mut self, address: u16) -> Option<u8> {
        self.get(address as usize).copied()
    }

    fn write(&mut self, address: u16, data: u8) -> Result<(), WriteError> {
        let d = self
            .get_mut(address as usize)
            .ok_or(WriteError::InvalidAddress)?;
        *d = data;
        Ok(())
    }
}

impl<T: Device> Device for Arc<Mutex<T>> {
    fn read(&mut self, address: u16) -> Option<u8> {
        let mut s = self.lock().unwrap();
        s.read(address)
    }

    fn write(&mut self, address: u16, data: u8) -> Result<(), WriteError> {
        let mut s = self.lock().unwrap();
        s.write(address, data)
    }
}

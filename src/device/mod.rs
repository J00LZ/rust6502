use thiserror::Error;
use std::fmt::{Display, Formatter};

pub mod ram;
pub mod rom;
pub mod device_map;

pub use ram::Ram;
pub use rom::Rom;

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
    fn read(&self, address: u16) -> Option<u8>;
    fn write(&mut self, address: u16, data: u8) -> Result<(), WriteError>;
}

impl<const N: usize> Device for [u8; N] {
    fn read(&self, address: u16) -> Option<u8> {
        Some(self[address as usize])
    }

    fn write(&mut self, address: u16, data: u8) -> Result<(), WriteError> {
        self[address as usize] = data;
        return Ok(());
    }
}

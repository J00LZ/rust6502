use std::error::Error;
use std::fmt::{Display, Formatter};

pub mod ram;
pub mod rom;
pub mod device_map;

pub use ram::Ram;
pub use rom::Rom;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub enum WriteError {
    NotWritable,
    InvalidAddress,
}

#[derive(Debug)]
pub enum CreateError {
    FsError(std::io::Error),
}

impl From<std::io::Error> for CreateError {
    fn from(e: std::io::Error) -> Self {
        Self::FsError(e)
    }
}

impl Display for CreateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for CreateError {}

impl Display for WriteError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for WriteError {}

pub trait Device {
    fn read(&self, address: u16) -> Option<u8>;
    fn write(&mut self, address: u16, data: u8) -> Result<(), WriteError>;
}

impl Device for [u8; 65536] {
    fn read(&self, address: u16) -> Option<u8> {
        Some(self[address as usize])
    }

    fn write(&mut self, address: u16, data: u8) -> Result<(), WriteError> {
        self[address as usize] = data;
        return Ok(());
    }
}

use crate::device::Device;

pub struct Ram {
    start: u16,
    data: Vec<u8>,
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
    fn read(&self, address: u16) -> Option<u8> {
        if address < self.start || self.start + (self.data.len() as u16) >= address {
            None
        } else {
            Some(self.data[(address - self.start) as usize])
        }
    }

    fn write(&mut self, address: u16, data: u8) -> Result<(), super::WriteError> {
        if address < self.start || self.start + (self.data.len() as u16) >= address {
            Err(super::WriteError::InvalidAddress)
        } else {
            self.data[(address - self.start) as usize] = data;
            Ok(())
        }
    }
}

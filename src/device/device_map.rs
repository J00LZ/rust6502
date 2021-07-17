use crate::device::WriteError;

pub struct DeviceMap<'t> {
    map: Vec<&'t dyn super::Device>,
}

impl DeviceMap<'_> {
    pub fn new() -> Self {
        Self { map: vec![] }
    }

    pub fn add<T: super::Device>(&mut self, dev: &T) {
        self.map.push(dev)
    }
}

impl super::Device for DeviceMap<'_> {
    fn read(&self, address: u16) -> Option<u8> {
        for d in self.map {
            let r = d.read(address);
            if r != None {
                return r;
            }
        }
        None
    }

    fn write(&mut self, address: u16, data: u8) -> Result<(), WriteError> {
        for mut d in self.map {
            let r = d.write(address, data);
            match r {
                Ok(_) => {}
                Err(WriteError::InvalidAddress) => {}
                Err(WriteError::NotWritable) => {}
            }
        }

        Ok(())
    }
}

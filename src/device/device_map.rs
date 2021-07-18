use super::*;

pub struct DeviceMap {
    devices: Vec<Box<dyn Device>>,
}

impl<'t> DeviceMap {
    pub fn new() -> Self {
        Self { devices: vec![] }
    }

    pub fn add<T: 'static + Device>(&mut self, ram: T) {
        self.devices.push(Box::new(ram));
    }
}

impl<'t> Device for DeviceMap {
    fn read(&self, address: u16) -> Option<u8> {
        for dev in &self.devices {
            match dev.read(address) {
                None => {}
                Some(x) => return x.into(),
            }
        }
        None
    }

    fn write(&mut self, address: u16, data: u8) -> Result<(), WriteError> {
        for dev in &mut self.devices {
            let _ = dev.write(address, data);
        }
        Ok(())
    }
}

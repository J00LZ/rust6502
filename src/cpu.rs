use bitflags::bitflags;
use std::num::Wrapping;

pub mod instructions;

#[derive(Debug)]
pub struct CPU {
    pub pc: u16,
    pub ac: u8,
    pub x: u8,
    pub y: u8,
    pub sr: StatusRegister,
    pub sp: u8,
    pub pins: Pins,
    ir: u16,
    nmi_pip: u16,
    irq_pip: u16,
    brk_flags: BreakFlags,
    bcd_enabled: bool,
    adl_adh: u16,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ReadWrite {
    Read,
    Write,
}

#[derive(Copy, Clone, Debug)]
pub enum PinFlags {
    Sync,
    Irq,
    Rdy,
    Aec,
    Res,
}

#[derive(Copy, Clone, Debug)]
pub struct Pins {
    pub data: u8,
    pub address: u16,
    pub rw: ReadWrite,
    pub sync: bool,
    pub irq: bool,
    pub rdy: bool,
    pub aec: bool,
    pub res: bool,
    pub nmi: bool,
}

impl Pins {
    #[must_use]
    pub fn new() -> Pins {
        Pins {
            data: 0,
            address: 0,
            rw: ReadWrite::Read,
            sync: true,
            irq: false,
            rdy: false,
            aec: false,
            res: true,
            nmi: false,
        }
    }
}

impl Default for Pins {
    fn default() -> Self {
        Self::new()
    }
}

bitflags! {
    pub struct BreakFlags:u8{
        const IRQ = 1<<0;
        const NMI = 1<<1;
        const RESET = 1<<2;
    }
}

bitflags! {
    pub struct StatusRegister: u8{
        const N = 0b1000_0000;
        const V = 0b0100_0000;
        const X = 0b0010_0000;
        const B = 0b0001_0000;
        const D = 0b0000_1000;
        const I = 0b0000_0100;
        const Z = 0b0000_0010;
        const C = 0b0000_0001;
    }
}

fn fetch(mut pins: &mut Pins, pc: u16) {
    sa(&mut pins, pc);
    on(&mut pins, PinFlags::Sync);
}

fn sa(mut pins: &mut Pins, addr: u16) {
    pins.address = addr;
}

fn ga(pins: &Pins) -> u16 {
    pins.address
}

fn sad(mut pins: &mut Pins, addr: u16, data: u8) {
    pins.address = addr;
    pins.data = data;
}

fn sd(mut pins: &mut Pins, data: u8) {
    pins.data = data;
}

fn gd(pins: &Pins) -> u8 {
    pins.data
}

fn on(mut pins: &mut Pins, x: PinFlags) {
    match x {
        PinFlags::Sync => pins.sync = true,
        PinFlags::Irq => pins.irq = true,
        PinFlags::Rdy => pins.rdy = true,
        PinFlags::Aec => pins.aec = true,
        PinFlags::Res => pins.res = true,
    }
}

fn off(mut pins: &mut Pins, x: PinFlags) {
    match x {
        PinFlags::Sync => pins.sync = false,
        PinFlags::Irq => pins.irq = false,
        PinFlags::Rdy => pins.rdy = false,
        PinFlags::Aec => pins.aec = false,
        PinFlags::Res => pins.res = false,
    }
}

fn rd(mut pins: &mut Pins) {
    pins.rw = ReadWrite::Read;
}

fn wr(mut pins: &mut Pins) {
    pins.rw = ReadWrite::Write;
}

impl Default for CPU {
    fn default() -> Self {
        Self::new()
    }
}

impl CPU {
    #[must_use]
    pub fn new() -> CPU {
        CPU {
            pc: 0,
            ac: 0,
            x: 0,
            y: 0,
            sr: StatusRegister::Z,
            sp: 0,
            ir: 0,
            pins: Pins::new(),
            nmi_pip: 0,
            irq_pip: 0,
            brk_flags: BreakFlags::empty(),
            bcd_enabled: false,
            adl_adh: 0,
        }
    }
    fn nz(&mut self, value: u8) {
        let x = if value == 0 {
            StatusRegister::Z.bits
        } else {
            value & StatusRegister::N.bits
        };
        self.sr = StatusRegister::from_bits(
            (self.sr.bits & !(StatusRegister::N.bits | StatusRegister::Z.bits)) | x,
        )
        .expect("Why, how??");
    }

    pub fn tick(&mut self, mut pins: Pins) -> Pins {
        if pins.sync | pins.irq | pins.nmi | pins.rdy | pins.res {
            if !self.pins.nmi && pins.nmi {
                self.nmi_pip |= 1;
            }
            if pins.irq && !self.sr.contains(StatusRegister::I) {
                self.irq_pip |= 1;
            }
            if pins.rw == ReadWrite::Read && pins.rdy {
                self.pins = pins;
                self.irq_pip <<= 1;
                return pins;
            }
            if pins.sync {
                self.ir = (gd(&pins) as u16) << 3;
                off(&mut pins, PinFlags::Sync);
                if self.irq_pip & 4 != 0 {
                    self.brk_flags.insert(BreakFlags::IRQ);
                }
                if self.nmi_pip & 0xFFFC != 0 {
                    self.brk_flags.insert(BreakFlags::NMI);
                }
                if pins.res {
                    self.brk_flags.insert(BreakFlags::RESET);
                }
                self.irq_pip &= 3;
                self.nmi_pip &= 3;
                if self.brk_flags.is_empty() {
                    self.pc += 1;
                } else {
                    self.ir = 0;
                    self.sr.remove(StatusRegister::B);
                    pins.res = false;
                }
            }
        }
        rd(&mut pins);

        println!(
            "going to execute {:#04X} step {}",
            self.ir >> 3,
            self.ir & 7
        );
        // println!("new ad: {:#06X}, {:#04X}", pins.address, pins.data);
        self.the_match_statement(&mut pins);
        // println!("new ad: {:#06X}, {:#04X}", pins.address, pins.data);

        println!("self={:?}, pins={:?}", self, pins);

        self.ir += 1;

        self.pins = pins;
        self.irq_pip <<= 1;
        self.nmi_pip <<= 1;
        pins
    }

    fn adc(&mut self, val: u8) {
        if self.bcd_enabled && self.sr.contains(StatusRegister::D) {
            let c = if self.sr.contains(StatusRegister::C) {
                1_u8
            } else {
                0
            };
            self.sr.remove(
                StatusRegister::N | StatusRegister::V | StatusRegister::Z | StatusRegister::V,
            );
            let mut al = (self.ac & 0x0F) + (val & 0x0F) + c;
            if al > 9 {
                al += 6;
            }
            let mut ah = (self.ac >> 4) + (val >> 4) + if al > 0x0F { 1 } else { 0 };
            if 0 == ((u16::from(self.ac) + u16::from(val) + u16::from(c)) & 0xFF) {
                self.sr.insert(StatusRegister::Z);
            } else if ah & 0x80 != 0 {
                self.sr.insert(StatusRegister::N);
            }
            if (!(self.ac ^ val) & (self.ac ^ (ah << 4)) & 0x80) != 0 {
                self.sr.insert(StatusRegister::V);
            }
            if ah > 9 {
                ah += 6;
            }
            if ah > 15 {
                self.sr.insert(StatusRegister::C);
            }
            self.ac = (ah << 4) | (al & 0xF);
        } else {
            let sum = self.ac as u16
                + val as u16
                + if self.sr.contains(StatusRegister::C) {
                    1
                } else {
                    0
                };
            self.sr.remove(StatusRegister::V | StatusRegister::C);
            self.nz(sum as u8);
            if (!(self.ac ^ val) & (self.ac ^ (sum as u8)) & 0x80) != 0 {
                self.sr.insert(StatusRegister::V);
            }
            if sum & 0xFF00 != 0 {
                self.sr.insert(StatusRegister::C);
            }
            self.ac = sum as u8;
        }
    }

    fn sbc(&mut self, val: u8) {
        let c = if self.sr.contains(StatusRegister::C) {
            1_u16
        } else {
            0
        };
        if self.bcd_enabled && self.sr.contains(StatusRegister::D) {
            self.sr.remove(
                StatusRegister::N | StatusRegister::V | StatusRegister::Z | StatusRegister::C,
            );
            let diff = (Wrapping(self.ac as u16) - Wrapping(val as u16) - Wrapping(c)).0;
            let mut al = Wrapping(self.ac & 0x0F) - Wrapping(val & 0x0F) - Wrapping(c as u8);
            if (al.0 as i8) < 0 {
                al -= Wrapping(6);
            }
            let mut ah = Wrapping(self.ac >> 4)
                - Wrapping(val >> 4)
                - Wrapping(if (al.0 as i8) < 0 { 1 } else { 0 });
            if (diff as u8) == 0 {
                self.sr.insert(StatusRegister::Z);
            } else if ah.0 & 0x80 != 0 {
                self.sr.insert(StatusRegister::N);
            }
            if (!(self.ac ^ val) & (self.ac ^ (diff as u8)) & 0x80) != 0 {
                self.sr.insert(StatusRegister::V);
            }
            if !(diff & 0xFF00) != 0 {
                self.sr.insert(StatusRegister::C);
            }
            if ah.0 & 0x80 != 0 {
                ah -= Wrapping(6);
            }
            self.ac = (ah.0 << 4) | (al.0 | 0xF);
        } else {
            let diff = (Wrapping(self.ac as u16) - Wrapping(val as u16) - Wrapping(c)).0;
            self.sr.remove(StatusRegister::C | StatusRegister::V);
            self.nz(diff as u8);
            if (!(self.ac ^ val) & (self.ac ^ (diff as u8)) & 0x80) != 0 {
                self.sr.insert(StatusRegister::V);
            }
            if !(diff & 0xFF00) != 0 {
                self.sr.insert(StatusRegister::C);
            }
            self.ac = (diff & 0xFF) as u8;
        }
    }

    fn cmp(&mut self, r: u8, v: u8) {
        let diff = (Wrapping(r as u16) - Wrapping(v as u16)).0;
        self.nz(diff as u8);
        self.sr.set(StatusRegister::C, (diff & 0xFF00) != 0);
    }

    fn sbx(&mut self, v: u8) {
        let x = (Wrapping((self.ac & self.x) as u16) - Wrapping(v as u16)).0;
        self.nz(x as u8);
        self.sr.set(StatusRegister::C, (x & 0xFF00) != 0);
        self.x = x as u8;
    }

    fn asl(&mut self, v: u8) -> u8 {
        self.nz(v << 1);
        self.sr.set(StatusRegister::C, v & 0x80 != 0);
        v << 1
    }

    fn lsr(&mut self, v: u8) -> u8 {
        self.nz(v >> 1);
        self.sr.set(StatusRegister::C, v & 0x01 != 0);
        v >> 1
    }

    fn rol(&mut self, mut v: u8) -> u8 {
        let carry = self.sr.contains(StatusRegister::C);
        self.sr
            .remove(StatusRegister::N | StatusRegister::Z | StatusRegister::C);
        if v & 0x80 != 0 {
            self.sr.insert(StatusRegister::C);
        }
        v <<= 1;
        if carry {
            v |= 1;
        }
        self.nz(v);
        v
    }
    fn ror(&mut self, mut v: u8) -> u8 {
        let carry = self.sr.contains(StatusRegister::C);
        self.sr
            .remove(StatusRegister::N | StatusRegister::Z | StatusRegister::C);
        if v & 0x01 != 0 {
            self.sr.insert(StatusRegister::C);
        }
        v >>= 1;
        if carry {
            v |= 0x80;
        }
        self.nz(v);
        v
    }

    fn arr(&mut self) {
        let carry = self.sr.contains(StatusRegister::C);
        self.sr
            .remove(StatusRegister::N | StatusRegister::V | StatusRegister::Z | StatusRegister::C);
        if self.bcd_enabled && self.sr.contains(StatusRegister::D) {
            let mut a = self.ac >> 1;
            if carry {
                a |= 0x80;
            }
            self.nz(a);
            if ((a ^ self.ac) & 0x40) != 0 {
                self.sr.insert(StatusRegister::V);
            }
            if (self.ac & 0xF) >= 5 {
                a = ((a + 6) & 0xF) | (a & 0xF0);
            }
            if (self.ac & 0xF0) >= 0x50 {
                a += 0x60;
                self.sr.insert(StatusRegister::C);
            }
            self.ac = a;
        } else {
            self.ac >>= 1;
            if carry {
                self.ac |= 0x80;
            }
            self.nz(self.ac);
            if (self.ac & 0x40) != 0 {
                self.sr.insert(StatusRegister::V | StatusRegister::C);
            }
            if (self.ac & 0x20) != 0 {
                self.sr.bits ^= StatusRegister::V.bits;
            }
        }
    }

    fn bit(&mut self, v: u8) {
        let t = self.ac & v;
        self.sr
            .remove(StatusRegister::N | StatusRegister::Z | StatusRegister::V);
        if t == 0 {
            self.sr.insert(StatusRegister::Z);
        }
        let v2 = v & (StatusRegister::N.bits | StatusRegister::V.bits);
        self.sr.bits |= v2;
    }
}

use codegen::codegen;
codegen!();

#[cfg(test)]
mod tests {
    use crate::cpu::{StatusRegister, CPU};

    #[test]
    fn memes() {
        let mut c = CPU::new();
        let mut sr = StatusRegister::empty();
        assert_eq!(c.sr, sr);
        c.nz(0);
        sr.toggle(StatusRegister::Z);
        assert_eq!(c.sr, sr);
        let x: i8 = -1;
        c.nz(x as u8);
        sr.remove(StatusRegister::Z);
        sr.toggle(StatusRegister::N);
        assert_eq!(c.sr, sr);
    }
}

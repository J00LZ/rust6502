use bitflags::bitflags;
use std::num::Wrapping;

pub mod instructions;
// pub mod opcodes;
mod out;

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

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ReadWrite {
    Read,
    Write,
}

#[derive(Copy, Clone)]
pub enum PinFlags {
    Sync,
    Irq,
    Rdy,
    Aec,
    Res,
}

#[derive(Copy, Clone)]
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

fn fetch(pins: &mut Pins, pc: u16) {
    sa(pins, pc);
    on(pins, PinFlags::Sync);
}

fn sa(pins: &mut Pins, addr: u16) {
    pins.address = addr;
}

fn ga(pins: &Pins) -> u16 {
    pins.address
}

fn sad(pins: &mut Pins, addr: u16, data: u8) {
    pins.address = addr;
    pins.data = data;
}

fn sd(pins: &mut Pins, data: u8) {
    pins.data = data;
}

fn gd(pins: &Pins) -> u8 {
    pins.data
}

fn on(pins: &mut Pins, x: PinFlags) {
    match x {
        PinFlags::Sync => pins.sync = true,
        PinFlags::Irq => pins.irq = true,
        PinFlags::Rdy => pins.rdy = true,
        PinFlags::Aec => pins.aec = true,
        PinFlags::Res => pins.res = true,
    }
}

fn off(pins: &mut Pins, x: PinFlags) {
    match x {
        PinFlags::Sync => pins.sync = false,
        PinFlags::Irq => pins.irq = false,
        PinFlags::Rdy => pins.rdy = false,
        PinFlags::Aec => pins.aec = false,
        PinFlags::Res => pins.res = false,
    }
}

fn rd(pins: &mut Pins) {
    pins.rw = ReadWrite::Read;
}

fn wr(pins: &mut Pins) {
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
                self.ir = pins.address << 3;
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

        self.the_match_statement(pins);

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

// impl CPU {
//     pub fn the_match_statement(&mut self, mut pins: Pins) {
//         match 0 {
//             /* BRK  (0x00) */
//             _ if self.ir == 0 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1 => {
//                 if !self.brk_flags.contains(BreakFlags::NMI)
//                     && !self.brk_flags.contains(BreakFlags::IRQ)
//                 {
//                     self.pc += 1;
//                 }
//                 sad(&mut pins, 0x0100 | self.sp as u16, (self.pc >> 8) as u8);
//                 self.sp = (Wrapping(self.sp) - Wrapping(1)).0;
//                 if !self.brk_flags.contains(BreakFlags::RESET) {
//                     wr(&mut pins)
//                 }
//             }
//             _ if self.ir == 2 => {
//                 sad(&mut pins, 0x0100 | self.sp as u16, (self.pc) as u8);
//                 self.sp = (Wrapping(self.sp) - Wrapping(1)).0;
//                 if !self.brk_flags.contains(BreakFlags::RESET) {
//                     wr(&mut pins)
//                 }
//             }
//             _ if self.ir == 3 => {
//                 sad(
//                     &mut pins,
//                     0x0100 | self.sp as u16,
//                     self.sr.bits | StatusRegister::X.bits,
//                 );
//                 self.sp = (Wrapping(self.sp) - Wrapping(1)).0;
//                 if self.brk_flags.contains(BreakFlags::RESET) {
//                     self.adl_adh = 0xFFFC;
//                 } else {
//                     wr(&mut pins);
//                     if self.brk_flags.contains(BreakFlags::NMI) {
//                         self.adl_adh = 0xFFFA
//                     } else {
//                         self.adl_adh = 0xFFFE
//                     }
//                 }
//             }
//             _ if self.ir == 4 => {
//                 sa(&mut pins, self.adl_adh);
//                 self.adl_adh += 1;
//                 self.sr.set(StatusRegister::I | StatusRegister::B, true);
//                 self.brk_flags = BreakFlags::empty();
//             }
//             _ if self.ir == 5 => {
//                 sa(&mut pins, self.adl_adh);
//                 self.adl_adh = gd(&pins) as u16; /* NMI "half-hijacking" not possible */
//             }
//             _ if self.ir == 6 => {
//                 self.pc = ((gd(&pins) as u16) << 8) | self.adl_adh;
//                 fetch(&mut pins, self.pc);
//             }
//             /* ORA (zp,X) (0x01) */
//             _ if self.ir == 8 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 9 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 10 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 11 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 12 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 13 => {
//                 self.ac |= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* JAM INVALID (undoc) (0x02) */
//             _ if self.ir == 16 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 17 => {
//                 sad(&mut pins, 0xFFFF, 0xFF);
//                 self.ir -= 1;
//             }
//             /* SLO (zp,X) (undoc) (0x03) */
//             _ if self.ir == 24 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 25 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 26 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 27 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 28 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 29 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 30 => {
//                 self.adl_adh = self.asl(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac |= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 31 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP zp (undoc) (0x04) */
//             _ if self.ir == 32 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 33 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 34 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ORA zp (0x05) */
//             _ if self.ir == 40 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 41 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 42 => {
//                 self.ac |= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ASL zp (0x06) */
//             _ if self.ir == 48 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 49 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 50 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 51 => {
//                 sd(&mut pins, self.asl(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 52 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SLO zp (undoc) (0x07) */
//             _ if self.ir == 56 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 57 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 58 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 59 => {
//                 self.adl_adh = self.asl(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac |= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 60 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* PHP  (0x08) */
//             _ if self.ir == 64 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 65 => {
//                 sad(
//                     &mut pins,
//                     0x0100 | (self.sp as u16),
//                     self.sr.bits | StatusRegister::X.bits,
//                 );
//                 self.sp -= 1;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 66 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ORA # (0x09) */
//             _ if self.ir == 72 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 73 => {
//                 self.ac |= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ASLA  (0x0A) */
//             _ if self.ir == 80 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 81 => {
//                 self.ac = self.asl(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ANC # (undoc) (0x0B) */
//             _ if self.ir == 88 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 89 => {
//                 self.ac = gd(&pins);
//                 self.nz(self.ac);
//                 self.sr.set(StatusRegister::C, (self.ac & 0x80) != 0);
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP abs (undoc) (0x0C) */
//             _ if self.ir == 96 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 97 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 98 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 99 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ORA abs (0x0D) */
//             _ if self.ir == 104 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 105 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 106 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 107 => {
//                 self.ac |= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ASL abs (0x0E) */
//             _ if self.ir == 112 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 113 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 114 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 115 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 116 => {
//                 sd(&mut pins, self.asl(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 117 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SLO abs (undoc) (0x0F) */
//             _ if self.ir == 120 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 121 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 122 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 123 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 124 => {
//                 self.adl_adh = self.asl(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac |= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 125 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* BPL # (0x10) */
//             _ if self.ir == 128 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 129 => {
//                 sa(&mut pins, self.pc);
//                 self.adl_adh = self.pc + (gd(&pins) as u16);
//                 if self.sr.bitand(StatusRegister::N).bits != 0x0 {
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 130 => {
//                 sa(&mut pins, (self.pc & 0xFF00) | (self.adl_adh & 0x00FF));
//                 if self.adl_adh & 0xFF00 == self.pc & 0xFF00 {
//                     self.pc = self.adl_adh;
//                     self.irq_pip >>= 1;
//                     self.nmi_pip >>= 1;
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 131 => {
//                 self.pc = self.adl_adh;
//                 fetch(&mut pins, self.pc);
//             }
//             /* ORA (zp),Y (0x11) */
//             _ if self.ir == 136 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 137 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 138 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 139 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 140 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 141 => {
//                 self.ac |= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* JAM INVALID (undoc) (0x12) */
//             _ if self.ir == 144 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 145 => {
//                 sad(&mut pins, 0xFFFF, 0xFF);
//                 self.ir -= 1;
//             }
//             /* SLO (zp),Y (undoc) (0x13) */
//             _ if self.ir == 152 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 153 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 154 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 155 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 156 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 157 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 158 => {
//                 self.adl_adh = self.asl(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac |= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 159 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP zp,X (undoc) (0x14) */
//             _ if self.ir == 160 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 161 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 162 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 163 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ORA zp,X (0x15) */
//             _ if self.ir == 168 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 169 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 170 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 171 => {
//                 self.ac |= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ASL zp,X (0x16) */
//             _ if self.ir == 176 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 177 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 178 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 179 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 180 => {
//                 sd(&mut pins, self.asl(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 181 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SLO zp,X (undoc) (0x17) */
//             _ if self.ir == 184 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 185 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 186 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 187 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 188 => {
//                 self.adl_adh = self.asl(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac |= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 189 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* CLC  (0x18) */
//             _ if self.ir == 192 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 193 => {
//                 self.sr.set(StatusRegister::C, false);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ORA abs,Y (0x19) */
//             _ if self.ir == 200 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 201 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 202 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 203 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 204 => {
//                 self.ac |= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP  (undoc) (0x1A) */
//             _ if self.ir == 208 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 209 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SLO abs,Y (undoc) (0x1B) */
//             _ if self.ir == 216 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 217 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 218 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 219 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 220 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 221 => {
//                 self.adl_adh = self.asl(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac |= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 222 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP abs,X (undoc) (0x1C) */
//             _ if self.ir == 224 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 225 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 226 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 227 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 228 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ORA abs,X (0x1D) */
//             _ if self.ir == 232 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 233 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 234 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 235 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 236 => {
//                 self.ac |= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ASL abs,X (0x1E) */
//             _ if self.ir == 240 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 241 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 242 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 243 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 244 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 245 => {
//                 sd(&mut pins, self.asl(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 246 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SLO abs,X (undoc) (0x1F) */
//             _ if self.ir == 248 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 249 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 250 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 251 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 252 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 253 => {
//                 self.adl_adh = self.asl(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac |= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 254 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* JSR  (0x20) */
//             _ if self.ir == 256 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 257 => {
//                 sa(&mut pins, 0x0100 | (self.sp as u16));
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 258 => {
//                 sad(&mut pins, 0x0100 | (self.sp as u16), (self.pc >> 8) as u8);
//                 self.sp -= 1;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 259 => {
//                 sad(&mut pins, 0x0100 | (self.sp as u16), self.pc as u8);
//                 self.sp -= 1;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 260 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 261 => {
//                 self.pc = ((gd(&pins) as u16) << 8) | self.adl_adh;
//                 fetch(&mut pins, self.pc);
//             }
//             /* AND (zp,X) (0x21) */
//             _ if self.ir == 264 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 265 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 266 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 267 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 268 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 269 => {
//                 self.ac &= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* JAM INVALID (undoc) (0x22) */
//             _ if self.ir == 272 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 273 => {
//                 sad(&mut pins, 0xFFFF, 0xFF);
//                 self.ir -= 1;
//             }
//             /* RLA (zp,X) (undoc) (0x23) */
//             _ if self.ir == 280 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 281 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 282 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 283 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 284 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 285 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 286 => {
//                 self.adl_adh = self.rol(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac &= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 287 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* BIT zp (0x24) */
//             _ if self.ir == 288 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 289 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 290 => {
//                 self.bit(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* AND zp (0x25) */
//             _ if self.ir == 296 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 297 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 298 => {
//                 self.ac &= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ROL zp (0x26) */
//             _ if self.ir == 304 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 305 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 306 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 307 => {
//                 sd(&mut pins, self.rol(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 308 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* RLA zp (undoc) (0x27) */
//             _ if self.ir == 312 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 313 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 314 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 315 => {
//                 self.adl_adh = self.rol(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac &= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 316 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* PLP  (0x28) */
//             _ if self.ir == 320 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 321 => {
//                 sa(&mut pins, 0x0100 | (self.sp as u16));
//                 self.sp += 1;
//             }
//             _ if self.ir == 322 => {
//                 sa(&mut pins, 0x0100 | (self.sp as u16));
//             }
//             _ if self.ir == 323 => {
//                 self.sr = StatusRegister::from_bits_truncate(
//                     (gd(&pins) | StatusRegister::B.bits) & !StatusRegister::X.bits,
//                 );
//                 fetch(&mut pins, self.pc);
//             }
//             /* AND # (0x29) */
//             _ if self.ir == 328 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 329 => {
//                 self.ac &= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ROLA  (0x2A) */
//             _ if self.ir == 336 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 337 => {
//                 self.ac = self.rol(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ANC # (undoc) (0x2B) */
//             _ if self.ir == 344 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 345 => {
//                 self.ac = gd(&pins);
//                 self.nz(self.ac);
//                 self.sr.set(StatusRegister::C, (self.ac & 0x80) != 0);
//                 fetch(&mut pins, self.pc);
//             }
//             /* BIT abs (0x2C) */
//             _ if self.ir == 352 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 353 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 354 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 355 => {
//                 self.bit(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* AND abs (0x2D) */
//             _ if self.ir == 360 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 361 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 362 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 363 => {
//                 self.ac &= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ROL abs (0x2E) */
//             _ if self.ir == 368 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 369 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 370 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 371 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 372 => {
//                 sd(&mut pins, self.rol(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 373 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* RLA abs (undoc) (0x2F) */
//             _ if self.ir == 376 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 377 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 378 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 379 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 380 => {
//                 self.adl_adh = self.rol(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac &= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 381 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* BMI # (0x30) */
//             _ if self.ir == 384 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 385 => {
//                 sa(&mut pins, self.pc);
//                 self.adl_adh = self.pc + (gd(&pins) as u16);
//                 if self.sr.bitand(StatusRegister::N).bits != 0x80 {
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 386 => {
//                 sa(&mut pins, (self.pc & 0xFF00) | (self.adl_adh & 0x00FF));
//                 if self.adl_adh & 0xFF00 == self.pc & 0xFF00 {
//                     self.pc = self.adl_adh;
//                     self.irq_pip >>= 1;
//                     self.nmi_pip >>= 1;
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 387 => {
//                 self.pc = self.adl_adh;
//                 fetch(&mut pins, self.pc);
//             }
//             /* AND (zp),Y (0x31) */
//             _ if self.ir == 392 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 393 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 394 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 395 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 396 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 397 => {
//                 self.ac &= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* JAM INVALID (undoc) (0x32) */
//             _ if self.ir == 400 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 401 => {
//                 sad(&mut pins, 0xFFFF, 0xFF);
//                 self.ir -= 1;
//             }
//             /* RLA (zp),Y (undoc) (0x33) */
//             _ if self.ir == 408 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 409 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 410 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 411 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 412 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 413 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 414 => {
//                 self.adl_adh = self.rol(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac &= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 415 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP zp,X (undoc) (0x34) */
//             _ if self.ir == 416 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 417 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 418 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 419 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* AND zp,X (0x35) */
//             _ if self.ir == 424 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 425 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 426 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 427 => {
//                 self.ac &= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ROL zp,X (0x36) */
//             _ if self.ir == 432 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 433 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 434 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 435 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 436 => {
//                 sd(&mut pins, self.rol(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 437 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* RLA zp,X (undoc) (0x37) */
//             _ if self.ir == 440 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 441 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 442 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 443 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 444 => {
//                 self.adl_adh = self.rol(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac &= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 445 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SEC  (0x38) */
//             _ if self.ir == 448 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 449 => {
//                 self.sr.set(StatusRegister::C, true);
//                 fetch(&mut pins, self.pc);
//             }
//             /* AND abs,Y (0x39) */
//             _ if self.ir == 456 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 457 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 458 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 459 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 460 => {
//                 self.ac &= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP  (undoc) (0x3A) */
//             _ if self.ir == 464 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 465 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* RLA abs,Y (undoc) (0x3B) */
//             _ if self.ir == 472 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 473 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 474 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 475 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 476 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 477 => {
//                 self.adl_adh = self.rol(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac &= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 478 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP abs,X (undoc) (0x3C) */
//             _ if self.ir == 480 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 481 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 482 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 483 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 484 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* AND abs,X (0x3D) */
//             _ if self.ir == 488 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 489 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 490 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 491 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 492 => {
//                 self.ac &= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ROL abs,X (0x3E) */
//             _ if self.ir == 496 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 497 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 498 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 499 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 500 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 501 => {
//                 sd(&mut pins, self.rol(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 502 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* RLA abs,X (undoc) (0x3F) */
//             _ if self.ir == 504 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 505 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 506 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 507 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 508 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 509 => {
//                 self.adl_adh = self.rol(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac &= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 510 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* RTI  (0x40) */
//             _ if self.ir == 512 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 513 => {
//                 sa(&mut pins, 0x0100 | (self.sp as u16));
//                 self.sp += 1;
//             }
//             _ if self.ir == 514 => {
//                 sa(&mut pins, 0x0100 | (self.sp as u16));
//                 self.sp += 1;
//             }
//             _ if self.ir == 515 => {
//                 sa(&mut pins, 0x0100 | (self.sp as u16));
//                 self.sp += 1;
//                 self.sr = StatusRegister::from_bits_truncate(
//                     (gd(&pins) | StatusRegister::B.bits) & !StatusRegister::X.bits,
//                 );
//             }
//             _ if self.ir == 516 => {
//                 sa(&mut pins, 0x0100 | (self.sp as u16));
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 517 => {
//                 self.pc = ((gd(&pins) as u16) << 8) | self.adl_adh;
//                 fetch(&mut pins, self.pc);
//             }
//             /* EOR (zp,X) (0x41) */
//             _ if self.ir == 520 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 521 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 522 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 523 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 524 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 525 => {
//                 self.ac ^= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* JAM INVALID (undoc) (0x42) */
//             _ if self.ir == 528 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 529 => {
//                 sad(&mut pins, 0xFFFF, 0xFF);
//                 self.ir -= 1;
//             }
//             /* SRE (zp,X) (undoc) (0x43) */
//             _ if self.ir == 536 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 537 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 538 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 539 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 540 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 541 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 542 => {
//                 self.adl_adh = self.lsr(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac ^= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 543 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP zp (undoc) (0x44) */
//             _ if self.ir == 544 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 545 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 546 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* EOR zp (0x45) */
//             _ if self.ir == 552 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 553 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 554 => {
//                 self.ac ^= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LSR zp (0x46) */
//             _ if self.ir == 560 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 561 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 562 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 563 => {
//                 sd(&mut pins, self.lsr(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 564 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SRE zp (undoc) (0x47) */
//             _ if self.ir == 568 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 569 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 570 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 571 => {
//                 self.adl_adh = self.lsr(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac ^= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 572 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* PHA  (0x48) */
//             _ if self.ir == 576 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 577 => {
//                 sad(&mut pins, 0x0100 | (self.sp as u16), self.ac);
//                 self.sp -= 1;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 578 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* EOR # (0x49) */
//             _ if self.ir == 584 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 585 => {
//                 self.ac ^= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LSRA  (0x4A) */
//             _ if self.ir == 592 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 593 => {
//                 self.ac = self.asl(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ASR # (undoc) (0x4B) */
//             _ if self.ir == 600 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 601 => {
//                 self.ac = gd(&pins);
//                 self.ac = self.asl(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* JMP  (0x4C) */
//             _ if self.ir == 608 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 609 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 610 => {
//                 self.pc = ((gd(&pins) as u16) << 8) | self.adl_adh;
//                 fetch(&mut pins, self.pc);
//             }
//             /* EOR abs (0x4D) */
//             _ if self.ir == 616 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 617 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 618 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 619 => {
//                 self.ac ^= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LSR abs (0x4E) */
//             _ if self.ir == 624 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 625 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 626 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 627 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 628 => {
//                 sd(&mut pins, self.lsr(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 629 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SRE abs (undoc) (0x4F) */
//             _ if self.ir == 632 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 633 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 634 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 635 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 636 => {
//                 self.adl_adh = self.lsr(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac ^= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 637 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* BVC # (0x50) */
//             _ if self.ir == 640 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 641 => {
//                 sa(&mut pins, self.pc);
//                 self.adl_adh = self.pc + (gd(&pins) as u16);
//                 if self.sr.bitand(StatusRegister::V).bits != 0x0 {
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 642 => {
//                 sa(&mut pins, (self.pc & 0xFF00) | (self.adl_adh & 0x00FF));
//                 if self.adl_adh & 0xFF00 == self.pc & 0xFF00 {
//                     self.pc = self.adl_adh;
//                     self.irq_pip >>= 1;
//                     self.nmi_pip >>= 1;
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 643 => {
//                 self.pc = self.adl_adh;
//                 fetch(&mut pins, self.pc);
//             }
//             /* EOR (zp),Y (0x51) */
//             _ if self.ir == 648 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 649 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 650 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 651 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 652 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 653 => {
//                 self.ac ^= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* JAM INVALID (undoc) (0x52) */
//             _ if self.ir == 656 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 657 => {
//                 sad(&mut pins, 0xFFFF, 0xFF);
//                 self.ir -= 1;
//             }
//             /* SRE (zp),Y (undoc) (0x53) */
//             _ if self.ir == 664 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 665 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 666 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 667 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 668 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 669 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 670 => {
//                 self.adl_adh = self.lsr(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac ^= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 671 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP zp,X (undoc) (0x54) */
//             _ if self.ir == 672 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 673 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 674 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 675 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* EOR zp,X (0x55) */
//             _ if self.ir == 680 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 681 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 682 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 683 => {
//                 self.ac ^= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LSR zp,X (0x56) */
//             _ if self.ir == 688 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 689 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 690 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 691 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 692 => {
//                 sd(&mut pins, self.lsr(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 693 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SRE zp,X (undoc) (0x57) */
//             _ if self.ir == 696 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 697 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 698 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 699 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 700 => {
//                 self.adl_adh = self.lsr(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac ^= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 701 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* CLI  (0x58) */
//             _ if self.ir == 704 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 705 => {
//                 self.sr.set(StatusRegister::I, false);
//                 fetch(&mut pins, self.pc);
//             }
//             /* EOR abs,Y (0x59) */
//             _ if self.ir == 712 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 713 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 714 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 715 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 716 => {
//                 self.ac ^= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP  (undoc) (0x5A) */
//             _ if self.ir == 720 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 721 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SRE abs,Y (undoc) (0x5B) */
//             _ if self.ir == 728 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 729 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 730 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 731 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 732 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 733 => {
//                 self.adl_adh = self.lsr(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac ^= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 734 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP abs,X (undoc) (0x5C) */
//             _ if self.ir == 736 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 737 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 738 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 739 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 740 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* EOR abs,X (0x5D) */
//             _ if self.ir == 744 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 745 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 746 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 747 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 748 => {
//                 self.ac ^= gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LSR abs,X (0x5E) */
//             _ if self.ir == 752 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 753 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 754 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 755 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 756 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 757 => {
//                 sd(&mut pins, self.lsr(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 758 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SRE abs,X (undoc) (0x5F) */
//             _ if self.ir == 760 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 761 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 762 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 763 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 764 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 765 => {
//                 self.adl_adh = self.lsr(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.ac ^= self.adl_adh as u8;
//                 self.nz(self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 766 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* RTS  (0x60) */
//             _ if self.ir == 768 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 769 => {
//                 sa(&mut pins, 0x0100 | (self.sp as u16));
//                 self.sp += 1;
//             }
//             _ if self.ir == 770 => {
//                 sa(&mut pins, 0x0100 | (self.sp as u16));
//                 self.sp += 1;
//             }
//             _ if self.ir == 771 => {
//                 sa(&mut pins, 0x0100 | (self.sp as u16));
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 772 => {
//                 self.pc = ((gd(&pins) as u16) << 8) | self.adl_adh;
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 773 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ADC (zp,X) (0x61) */
//             _ if self.ir == 776 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 777 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 778 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 779 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 780 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 781 => {
//                 self.adc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* JAM INVALID (undoc) (0x62) */
//             _ if self.ir == 784 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 785 => {
//                 sad(&mut pins, 0xFFFF, 0xFF);
//                 self.ir -= 1;
//             }
//             /* RRA (zp,X) (undoc) (0x63) */
//             _ if self.ir == 792 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 793 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 794 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 795 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 796 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 797 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 798 => {
//                 self.adl_adh = self.ror(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.adc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 799 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP zp (undoc) (0x64) */
//             _ if self.ir == 800 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 801 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 802 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ADC zp (0x65) */
//             _ if self.ir == 808 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 809 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 810 => {
//                 self.adc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* ROR zp (0x66) */
//             _ if self.ir == 816 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 817 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 818 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 819 => {
//                 sd(&mut pins, self.ror(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 820 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* RRA zp (undoc) (0x67) */
//             _ if self.ir == 824 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 825 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 826 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 827 => {
//                 self.adl_adh = self.ror(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.adc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 828 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* PLA  (0x68) */
//             _ if self.ir == 832 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 833 => {
//                 sa(&mut pins, 0x0100 | (self.sp as u16));
//                 self.sp += 1;
//             }
//             _ if self.ir == 834 => {
//                 sa(&mut pins, 0x0100 | (self.sp as u16));
//             }
//             _ if self.ir == 835 => {
//                 self.ac = gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ADC # (0x69) */
//             _ if self.ir == 840 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 841 => {
//                 self.adc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* RORA  (0x6A) */
//             _ if self.ir == 848 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 849 => {
//                 self.ac = self.ror(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ARR # (undoc) (0x6B) */
//             _ if self.ir == 856 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 857 => {
//                 self.ac = gd(&pins);
//                 self.arr();
//                 fetch(&mut pins, self.pc);
//             }
//             /* JMPI  (0x6C) */
//             _ if self.ir == 864 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 865 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 866 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 867 => {
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + 1) & 0x00FF),
//                 );
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 868 => {
//                 self.pc = ((gd(&pins) as u16) << 8) | self.adl_adh;
//                 fetch(&mut pins, self.pc);
//             }
//             /* ADC abs (0x6D) */
//             _ if self.ir == 872 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 873 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 874 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 875 => {
//                 self.adc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* ROR abs (0x6E) */
//             _ if self.ir == 880 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 881 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 882 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 883 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 884 => {
//                 sd(&mut pins, self.ror(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 885 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* RRA abs (undoc) (0x6F) */
//             _ if self.ir == 888 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 889 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 890 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 891 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 892 => {
//                 self.adl_adh = self.ror(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.adc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 893 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* BVS # (0x70) */
//             _ if self.ir == 896 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 897 => {
//                 sa(&mut pins, self.pc);
//                 self.adl_adh = self.pc + (gd(&pins) as u16);
//                 if self.sr.bitand(StatusRegister::V).bits != 0x40 {
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 898 => {
//                 sa(&mut pins, (self.pc & 0xFF00) | (self.adl_adh & 0x00FF));
//                 if self.adl_adh & 0xFF00 == self.pc & 0xFF00 {
//                     self.pc = self.adl_adh;
//                     self.irq_pip >>= 1;
//                     self.nmi_pip >>= 1;
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 899 => {
//                 self.pc = self.adl_adh;
//                 fetch(&mut pins, self.pc);
//             }
//             /* ADC (zp),Y (0x71) */
//             _ if self.ir == 904 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 905 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 906 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 907 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 908 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 909 => {
//                 self.adc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* JAM INVALID (undoc) (0x72) */
//             _ if self.ir == 912 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 913 => {
//                 sad(&mut pins, 0xFFFF, 0xFF);
//                 self.ir -= 1;
//             }
//             /* RRA (zp),Y (undoc) (0x73) */
//             _ if self.ir == 920 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 921 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 922 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 923 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 924 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 925 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 926 => {
//                 self.adl_adh = self.ror(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.adc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 927 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP zp,X (undoc) (0x74) */
//             _ if self.ir == 928 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 929 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 930 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 931 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ADC zp,X (0x75) */
//             _ if self.ir == 936 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 937 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 938 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 939 => {
//                 self.adc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* ROR zp,X (0x76) */
//             _ if self.ir == 944 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 945 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 946 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 947 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 948 => {
//                 sd(&mut pins, self.ror(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 949 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* RRA zp,X (undoc) (0x77) */
//             _ if self.ir == 952 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 953 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 954 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 955 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 956 => {
//                 self.adl_adh = self.ror(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.adc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 957 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SEI  (0x78) */
//             _ if self.ir == 960 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 961 => {
//                 self.sr.set(StatusRegister::I, true);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ADC abs,Y (0x79) */
//             _ if self.ir == 968 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 969 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 970 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 971 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 972 => {
//                 self.adc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP  (undoc) (0x7A) */
//             _ if self.ir == 976 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 977 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* RRA abs,Y (undoc) (0x7B) */
//             _ if self.ir == 984 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 985 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 986 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 987 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 988 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 989 => {
//                 self.adl_adh = self.ror(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.adc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 990 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP abs,X (undoc) (0x7C) */
//             _ if self.ir == 992 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 993 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 994 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 995 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 996 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ADC abs,X (0x7D) */
//             _ if self.ir == 1000 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1001 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1002 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1003 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 1004 => {
//                 self.adc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* ROR abs,X (0x7E) */
//             _ if self.ir == 1008 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1009 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1010 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1011 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 1012 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1013 => {
//                 sd(&mut pins, self.ror(self.adl_adh as u8));
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1014 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* RRA abs,X (undoc) (0x7F) */
//             _ if self.ir == 1016 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1017 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1018 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1019 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 1020 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1021 => {
//                 self.adl_adh = self.ror(self.adl_adh as u8) as u16;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.adc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1022 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP # (undoc) (0x80) */
//             _ if self.ir == 1024 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1025 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* STA (zp,X) (0x81) */
//             _ if self.ir == 1032 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1033 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1034 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1035 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1036 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//                 sd(&mut pins, self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1037 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP # (undoc) (0x82) */
//             _ if self.ir == 1040 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1041 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SAX (zp,X) (undoc) (0x83) */
//             _ if self.ir == 1048 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1049 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1050 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1051 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1052 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//                 sd(&mut pins, self.ac & self.x);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1053 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* STY zp (0x84) */
//             _ if self.ir == 1056 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1057 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//                 sd(&mut pins, self.y);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1058 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* STA zp (0x85) */
//             _ if self.ir == 1064 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1065 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//                 sd(&mut pins, self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1066 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* STX zp (0x86) */
//             _ if self.ir == 1072 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1073 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//                 sd(&mut pins, self.x);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1074 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SAX zp (undoc) (0x87) */
//             _ if self.ir == 1080 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1081 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//                 sd(&mut pins, self.ac & self.x);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1082 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* DEY  (0x88) */
//             _ if self.ir == 1088 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1089 => {
//                 self.y -= 1;
//                 self.nz(self.y);
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP # (undoc) (0x89) */
//             _ if self.ir == 1096 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1097 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* TXA  (0x8A) */
//             _ if self.ir == 1104 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1105 => {
//                 self.ac = self.x;
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* ANE # (undoc) (0x8B) */
//             _ if self.ir == 1112 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1113 => {
//                 self.ac = (self.ac | 0xEE) & self.x & gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* STY abs (0x8C) */
//             _ if self.ir == 1120 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1121 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1122 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//                 sd(&mut pins, self.y);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1123 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* STA abs (0x8D) */
//             _ if self.ir == 1128 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1129 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1130 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//                 sd(&mut pins, self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1131 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* STX abs (0x8E) */
//             _ if self.ir == 1136 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1137 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1138 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//                 sd(&mut pins, self.x);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1139 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SAX abs (undoc) (0x8F) */
//             _ if self.ir == 1144 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1145 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1146 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//                 sd(&mut pins, self.ac & self.x);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1147 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* BCC # (0x90) */
//             _ if self.ir == 1152 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1153 => {
//                 sa(&mut pins, self.pc);
//                 self.adl_adh = self.pc + (gd(&pins) as u16);
//                 if self.sr.bitand(StatusRegister::C).bits != 0x0 {
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 1154 => {
//                 sa(&mut pins, (self.pc & 0xFF00) | (self.adl_adh & 0x00FF));
//                 if self.adl_adh & 0xFF00 == self.pc & 0xFF00 {
//                     self.pc = self.adl_adh;
//                     self.irq_pip >>= 1;
//                     self.nmi_pip >>= 1;
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 1155 => {
//                 self.pc = self.adl_adh;
//                 fetch(&mut pins, self.pc);
//             }
//             /* STA (zp),Y (0x91) */
//             _ if self.ir == 1160 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1161 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1162 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1163 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1164 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//                 sd(&mut pins, self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1165 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* JAM INVALID (undoc) (0x92) */
//             _ if self.ir == 1168 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1169 => {
//                 sad(&mut pins, 0xFFFF, 0xFF);
//                 self.ir -= 1;
//             }
//             /* SHA (zp),Y (undoc) (0x93) */
//             _ if self.ir == 1176 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1177 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1178 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1179 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1180 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//                 let zz = ((ga(&pins) >> 8) + 1) as u8;
//                 sd(&mut pins, self.ac & self.x & zz);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1181 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* STY zp,X (0x94) */
//             _ if self.ir == 1184 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1185 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1186 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//                 sd(&mut pins, self.y);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1187 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* STA zp,X (0x95) */
//             _ if self.ir == 1192 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1193 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1194 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//                 sd(&mut pins, self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1195 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* STX zp,Y (0x96) */
//             _ if self.ir == 1200 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1201 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1202 => {
//                 sa(&mut pins, (self.adl_adh + (self.y as u16)) & 0x00FF);
//                 sd(&mut pins, self.x);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1203 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SAX zp,Y (undoc) (0x97) */
//             _ if self.ir == 1208 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1209 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1210 => {
//                 sa(&mut pins, (self.adl_adh + (self.y as u16)) & 0x00FF);
//                 sd(&mut pins, self.ac & self.x);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1211 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* TYA  (0x98) */
//             _ if self.ir == 1216 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1217 => {
//                 self.ac = self.x;
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* STA abs,Y (0x99) */
//             _ if self.ir == 1224 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1225 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1226 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1227 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//                 sd(&mut pins, self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1228 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* TXS  (0x9A) */
//             _ if self.ir == 1232 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1233 => {
//                 self.sp = self.x;
//                 fetch(&mut pins, self.pc);
//             }
//             /* SHS abs,Y (undoc) (0x9B) */
//             _ if self.ir == 1240 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1241 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1242 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1243 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//                 self.sp = self.ac & self.x;
//                 let zz = ((ga(&pins) >> 8) + 1) as u8;
//                 sd(&mut pins, self.sp & zz);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1244 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SHY abs,X (undoc) (0x9C) */
//             _ if self.ir == 1248 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1249 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1250 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1251 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//                 let zz = ((ga(&pins) >> 8) + 1) as u8;
//                 sd(&mut pins, self.y & zz);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1252 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* STA abs,X (0x9D) */
//             _ if self.ir == 1256 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1257 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1258 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1259 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//                 sd(&mut pins, self.ac);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1260 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SHX abs,Y (undoc) (0x9E) */
//             _ if self.ir == 1264 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1265 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1266 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1267 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//                 let zz = ((ga(&pins) >> 8) + 1) as u8;
//                 sd(&mut pins, self.x & zz);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1268 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SHA abs,Y (undoc) (0x9F) */
//             _ if self.ir == 1272 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1273 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1274 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1275 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//                 let zz = ((ga(&pins) >> 8) + 1) as u8;
//                 sd(&mut pins, self.ac & self.x & zz);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1276 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDY # (0xA0) */
//             _ if self.ir == 1280 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1281 => {
//                 self.y = gd(&pins);
//                 self.nz(self.y);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDA (zp,X) (0xA1) */
//             _ if self.ir == 1288 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1289 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1290 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1291 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1292 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1293 => {
//                 self.ac = gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDX # (0xA2) */
//             _ if self.ir == 1296 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1297 => {
//                 self.x = gd(&pins);
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LAX (zp,X) (undoc) (0xA3) */
//             _ if self.ir == 1304 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1305 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1306 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1307 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1308 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1309 => {
//                 self.ac = gd(&pins);
//                 self.x = gd(&pins);
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDY zp (0xA4) */
//             _ if self.ir == 1312 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1313 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 1314 => {
//                 self.y = gd(&pins);
//                 self.nz(self.y);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDA zp (0xA5) */
//             _ if self.ir == 1320 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1321 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 1322 => {
//                 self.ac = gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDX zp (0xA6) */
//             _ if self.ir == 1328 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1329 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 1330 => {
//                 self.x = gd(&pins);
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LAX zp (undoc) (0xA7) */
//             _ if self.ir == 1336 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1337 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 1338 => {
//                 self.ac = gd(&pins);
//                 self.x = gd(&pins);
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* TAY  (0xA8) */
//             _ if self.ir == 1344 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1345 => {
//                 self.y = self.ac;
//                 self.nz(self.y);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDA # (0xA9) */
//             _ if self.ir == 1352 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1353 => {
//                 self.ac = gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* TAX  (0xAA) */
//             _ if self.ir == 1360 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1361 => {
//                 self.x = self.ac;
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LXA # (undoc) (0xAB) */
//             _ if self.ir == 1368 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1369 => {
//                 let zz = (self.ac | 0xEE) & gd(&pins);
//                 self.ac = zz;
//                 self.x = zz;
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDY abs (0xAC) */
//             _ if self.ir == 1376 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1377 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1378 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1379 => {
//                 self.y = gd(&pins);
//                 self.nz(self.y);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDA abs (0xAD) */
//             _ if self.ir == 1384 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1385 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1386 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1387 => {
//                 self.ac = gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDX abs (0xAE) */
//             _ if self.ir == 1392 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1393 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1394 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1395 => {
//                 self.x = gd(&pins);
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LAX abs (undoc) (0xAF) */
//             _ if self.ir == 1400 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1401 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1402 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1403 => {
//                 self.ac = gd(&pins);
//                 self.x = gd(&pins);
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* BCS # (0xB0) */
//             _ if self.ir == 1408 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1409 => {
//                 sa(&mut pins, self.pc);
//                 self.adl_adh = self.pc + (gd(&pins) as u16);
//                 if self.sr.bitand(StatusRegister::C).bits != 0x1 {
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 1410 => {
//                 sa(&mut pins, (self.pc & 0xFF00) | (self.adl_adh & 0x00FF));
//                 if self.adl_adh & 0xFF00 == self.pc & 0xFF00 {
//                     self.pc = self.adl_adh;
//                     self.irq_pip >>= 1;
//                     self.nmi_pip >>= 1;
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 1411 => {
//                 self.pc = self.adl_adh;
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDA (zp),Y (0xB1) */
//             _ if self.ir == 1416 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1417 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1418 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1419 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1420 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 1421 => {
//                 self.ac = gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* JAM INVALID (undoc) (0xB2) */
//             _ if self.ir == 1424 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1425 => {
//                 sad(&mut pins, 0xFFFF, 0xFF);
//                 self.ir -= 1;
//             }
//             /* LAX (zp),Y (undoc) (0xB3) */
//             _ if self.ir == 1432 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1433 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1434 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1435 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1436 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 1437 => {
//                 self.ac = gd(&pins);
//                 self.x = gd(&pins);
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDY zp,X (0xB4) */
//             _ if self.ir == 1440 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1441 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1442 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 1443 => {
//                 self.y = gd(&pins);
//                 self.nz(self.y);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDA zp,X (0xB5) */
//             _ if self.ir == 1448 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1449 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1450 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 1451 => {
//                 self.ac = gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDX zp,Y (0xB6) */
//             _ if self.ir == 1456 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1457 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1458 => {
//                 sa(&mut pins, (self.adl_adh + (self.y as u16)) & 0x00FF);
//             }
//             _ if self.ir == 1459 => {
//                 self.x = gd(&pins);
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LAX zp,Y (undoc) (0xB7) */
//             _ if self.ir == 1464 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1465 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1466 => {
//                 sa(&mut pins, (self.adl_adh + (self.y as u16)) & 0x00FF);
//             }
//             _ if self.ir == 1467 => {
//                 self.ac = gd(&pins);
//                 self.x = gd(&pins);
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* CLV  (0xB8) */
//             _ if self.ir == 1472 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1473 => {
//                 self.sr.set(StatusRegister::V, false);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDA abs,Y (0xB9) */
//             _ if self.ir == 1480 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1481 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1482 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1483 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 1484 => {
//                 self.ac = gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* TSX  (0xBA) */
//             _ if self.ir == 1488 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1489 => {
//                 self.x = self.sp;
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LAS abs,Y (undoc) (0xBB) */
//             _ if self.ir == 1496 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1497 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1498 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1499 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 1500 => {
//                 let zz = gd(&pins) & self.sp;
//                 self.ac = zz;
//                 self.x = zz;
//                 self.sp = zz;
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDY abs,X (0xBC) */
//             _ if self.ir == 1504 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1505 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1506 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1507 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 1508 => {
//                 self.y = gd(&pins);
//                 self.nz(self.y);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDA abs,X (0xBD) */
//             _ if self.ir == 1512 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1513 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1514 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1515 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 1516 => {
//                 self.ac = gd(&pins);
//                 self.nz(self.ac);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LDX abs,Y (0xBE) */
//             _ if self.ir == 1520 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1521 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1522 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1523 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 1524 => {
//                 self.x = gd(&pins);
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* LAX abs,Y (undoc) (0xBF) */
//             _ if self.ir == 1528 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1529 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1530 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1531 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 1532 => {
//                 self.ac = gd(&pins);
//                 self.x = gd(&pins);
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* CPY # (0xC0) */
//             _ if self.ir == 1536 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1537 => {
//                 self.cmp(self.y, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* CMP (zp,X) (0xC1) */
//             _ if self.ir == 1544 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1545 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1546 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1547 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1548 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1549 => {
//                 self.cmp(self.ac, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP # (undoc) (0xC2) */
//             _ if self.ir == 1552 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1553 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* DCP (zp,X) (undoc) (0xC3) */
//             _ if self.ir == 1560 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1561 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1562 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1563 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1564 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1565 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1566 => {
//                 self.adl_adh -= 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.cmp(self.ac, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1567 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* CPY zp (0xC4) */
//             _ if self.ir == 1568 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1569 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 1570 => {
//                 self.cmp(self.y, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* CMP zp (0xC5) */
//             _ if self.ir == 1576 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1577 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 1578 => {
//                 self.cmp(self.ac, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* DEC zp (0xC6) */
//             _ if self.ir == 1584 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1585 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 1586 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1587 => {
//                 self.adl_adh -= 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1588 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* DCP zp (undoc) (0xC7) */
//             _ if self.ir == 1592 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1593 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 1594 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1595 => {
//                 self.adl_adh -= 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.cmp(self.ac, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1596 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* INY  (0xC8) */
//             _ if self.ir == 1600 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1601 => {
//                 self.y += 1;
//                 self.nz(self.y);
//                 fetch(&mut pins, self.pc);
//             }
//             /* CMP # (0xC9) */
//             _ if self.ir == 1608 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1609 => {
//                 self.cmp(self.ac, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* DEX  (0xCA) */
//             _ if self.ir == 1616 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1617 => {
//                 self.x -= 1;
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* SBX # (undoc) (0xCB) */
//             _ if self.ir == 1624 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1625 => {
//                 self.sbx(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* CPY abs (0xCC) */
//             _ if self.ir == 1632 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1633 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1634 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1635 => {
//                 self.cmp(self.y, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* CMP abs (0xCD) */
//             _ if self.ir == 1640 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1641 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1642 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1643 => {
//                 self.cmp(self.ac, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* DEC abs (0xCE) */
//             _ if self.ir == 1648 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1649 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1650 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1651 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1652 => {
//                 self.adl_adh -= 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1653 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* DCP abs (undoc) (0xCF) */
//             _ if self.ir == 1656 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1657 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1658 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1659 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1660 => {
//                 self.adl_adh -= 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.cmp(self.ac, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1661 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* BNE # (0xD0) */
//             _ if self.ir == 1664 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1665 => {
//                 sa(&mut pins, self.pc);
//                 self.adl_adh = self.pc + (gd(&pins) as u16);
//                 if self.sr.bitand(StatusRegister::Z).bits != 0x0 {
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 1666 => {
//                 sa(&mut pins, (self.pc & 0xFF00) | (self.adl_adh & 0x00FF));
//                 if self.adl_adh & 0xFF00 == self.pc & 0xFF00 {
//                     self.pc = self.adl_adh;
//                     self.irq_pip >>= 1;
//                     self.nmi_pip >>= 1;
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 1667 => {
//                 self.pc = self.adl_adh;
//                 fetch(&mut pins, self.pc);
//             }
//             /* CMP (zp),Y (0xD1) */
//             _ if self.ir == 1672 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1673 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1674 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1675 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1676 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 1677 => {
//                 self.cmp(self.ac, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* JAM INVALID (undoc) (0xD2) */
//             _ if self.ir == 1680 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1681 => {
//                 sad(&mut pins, 0xFFFF, 0xFF);
//                 self.ir -= 1;
//             }
//             /* DCP (zp),Y (undoc) (0xD3) */
//             _ if self.ir == 1688 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1689 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1690 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1691 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1692 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 1693 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1694 => {
//                 self.adl_adh -= 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.cmp(self.ac, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1695 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP zp,X (undoc) (0xD4) */
//             _ if self.ir == 1696 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1697 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1698 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 1699 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* CMP zp,X (0xD5) */
//             _ if self.ir == 1704 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1705 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1706 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 1707 => {
//                 self.cmp(self.ac, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* DEC zp,X (0xD6) */
//             _ if self.ir == 1712 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1713 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1714 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 1715 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1716 => {
//                 self.adl_adh -= 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1717 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* DCP zp,X (undoc) (0xD7) */
//             _ if self.ir == 1720 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1721 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1722 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 1723 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1724 => {
//                 self.adl_adh -= 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.cmp(self.ac, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1725 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* CLD  (0xD8) */
//             _ if self.ir == 1728 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1729 => {
//                 self.sr.set(StatusRegister::D, false);
//                 fetch(&mut pins, self.pc);
//             }
//             /* CMP abs,Y (0xD9) */
//             _ if self.ir == 1736 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1737 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1738 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1739 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 1740 => {
//                 self.cmp(self.ac, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP  (undoc) (0xDA) */
//             _ if self.ir == 1744 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1745 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* DCP abs,Y (undoc) (0xDB) */
//             _ if self.ir == 1752 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1753 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1754 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1755 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 1756 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1757 => {
//                 self.adl_adh -= 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.cmp(self.ac, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1758 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP abs,X (undoc) (0xDC) */
//             _ if self.ir == 1760 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1761 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1762 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1763 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 1764 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* CMP abs,X (0xDD) */
//             _ if self.ir == 1768 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1769 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1770 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1771 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 1772 => {
//                 self.cmp(self.ac, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* DEC abs,X (0xDE) */
//             _ if self.ir == 1776 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1777 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1778 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1779 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 1780 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1781 => {
//                 self.adl_adh -= 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1782 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* DCP abs,X (undoc) (0xDF) */
//             _ if self.ir == 1784 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1785 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1786 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1787 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 1788 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1789 => {
//                 self.adl_adh -= 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.cmp(self.ac, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1790 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* CPX # (0xE0) */
//             _ if self.ir == 1792 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1793 => {
//                 self.cmp(self.x, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* SBC (zp,X) (0xE1) */
//             _ if self.ir == 1800 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1801 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1802 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1803 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1804 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1805 => {
//                 self.sbc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP # (undoc) (0xE2) */
//             _ if self.ir == 1808 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1809 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ISB (zp,X) (undoc) (0xE3) */
//             _ if self.ir == 1816 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1817 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1818 => {
//                 self.adl_adh = (self.adl_adh + (self.x as u16)) & 0xFF;
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1819 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1820 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1821 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1822 => {
//                 self.adl_adh += 1;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.sbc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1823 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* CPX zp (0xE4) */
//             _ if self.ir == 1824 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1825 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 1826 => {
//                 self.cmp(self.x, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* SBC zp (0xE5) */
//             _ if self.ir == 1832 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1833 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 1834 => {
//                 self.sbc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* INC zp (0xE6) */
//             _ if self.ir == 1840 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1841 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 1842 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1843 => {
//                 self.adl_adh += 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1844 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ISB zp (undoc) (0xE7) */
//             _ if self.ir == 1848 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1849 => {
//                 let zz = ga(&pins);
//                 sa(&mut pins, zz);
//             }
//             _ if self.ir == 1850 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1851 => {
//                 self.adl_adh += 1;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.sbc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1852 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* INX  (0xE8) */
//             _ if self.ir == 1856 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1857 => {
//                 self.x += 1;
//                 self.nz(self.x);
//                 fetch(&mut pins, self.pc);
//             }
//             /* SBC # (0xE9) */
//             _ if self.ir == 1864 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1865 => {
//                 self.sbc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP  (0xEA) */
//             _ if self.ir == 1872 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1873 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SBC # (undoc) (0xEB) */
//             _ if self.ir == 1880 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1881 => {
//                 self.sbc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* CPX abs (0xEC) */
//             _ if self.ir == 1888 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1889 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1890 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1891 => {
//                 self.cmp(self.x, gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* SBC abs (0xED) */
//             _ if self.ir == 1896 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1897 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1898 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1899 => {
//                 self.sbc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* INC abs (0xEE) */
//             _ if self.ir == 1904 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1905 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1906 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1907 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1908 => {
//                 self.adl_adh += 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1909 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ISB abs (undoc) (0xEF) */
//             _ if self.ir == 1912 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1913 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1914 => {
//                 let zz = gd(&pins);
//                 sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);
//             }
//             _ if self.ir == 1915 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1916 => {
//                 self.adl_adh += 1;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.sbc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1917 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* BEQ # (0xF0) */
//             _ if self.ir == 1920 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1921 => {
//                 sa(&mut pins, self.pc);
//                 self.adl_adh = self.pc + (gd(&pins) as u16);
//                 if self.sr.bitand(StatusRegister::Z).bits != 0x2 {
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 1922 => {
//                 sa(&mut pins, (self.pc & 0xFF00) | (self.adl_adh & 0x00FF));
//                 if self.adl_adh & 0xFF00 == self.pc & 0xFF00 {
//                     self.pc = self.adl_adh;
//                     self.irq_pip >>= 1;
//                     self.nmi_pip >>= 1;
//                     fetch(&mut pins, self.pc)
//                 }
//             }
//             _ if self.ir == 1923 => {
//                 self.pc = self.adl_adh;
//                 fetch(&mut pins, self.pc);
//             }
//             /* SBC (zp),Y (0xF1) */
//             _ if self.ir == 1928 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1929 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1930 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1931 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1932 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 1933 => {
//                 self.sbc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* JAM INVALID (undoc) (0xF2) */
//             _ if self.ir == 1936 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1937 => {
//                 sad(&mut pins, 0xFFFF, 0xFF);
//                 self.ir -= 1;
//             }
//             /* ISB (zp),Y (undoc) (0xF3) */
//             _ if self.ir == 1944 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1945 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1946 => {
//                 sa(&mut pins, (self.adl_adh + 1) & 0xFF);
//                 self.adl_adh = ga(&pins);
//             }
//             _ if self.ir == 1947 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 1948 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 1949 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1950 => {
//                 self.adl_adh += 1;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.sbc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1951 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP zp,X (undoc) (0xF4) */
//             _ if self.ir == 1952 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1953 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1954 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 1955 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SBC zp,X (0xF5) */
//             _ if self.ir == 1960 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1961 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1962 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 1963 => {
//                 self.sbc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* INC zp,X (0xF6) */
//             _ if self.ir == 1968 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1969 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1970 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 1971 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1972 => {
//                 self.adl_adh += 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1973 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ISB zp,X (undoc) (0xF7) */
//             _ if self.ir == 1976 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1977 => {
//                 self.adl_adh = ga(&pins);
//                 sa(&mut pins, self.adl_adh);
//             }
//             _ if self.ir == 1978 => {
//                 sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);
//             }
//             _ if self.ir == 1979 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1980 => {
//                 self.adl_adh += 1;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.sbc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 1981 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SED  (0xF8) */
//             _ if self.ir == 1984 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 1985 => {
//                 self.sr.set(StatusRegister::D, true);
//                 fetch(&mut pins, self.pc);
//             }
//             /* SBC abs,Y (0xF9) */
//             _ if self.ir == 1992 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 1993 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 1994 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 1995 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 1996 => {
//                 self.sbc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP  (undoc) (0xFA) */
//             _ if self.ir == 2000 => {
//                 sa(&mut pins, self.pc);
//             }
//             _ if self.ir == 2001 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ISB abs,Y (undoc) (0xFB) */
//             _ if self.ir == 2008 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 2009 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 2010 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.y as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 2011 => {
//                 sa(&mut pins, self.adl_adh + (self.y as u16));
//             }
//             _ if self.ir == 2012 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 2013 => {
//                 self.adl_adh += 1;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.sbc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 2014 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* NOP abs,X (undoc) (0xFC) */
//             _ if self.ir == 2016 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 2017 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 2018 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 2019 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 2020 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* SBC abs,X (0xFD) */
//             _ if self.ir == 2024 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 2025 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 2026 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//                 self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8))) & 1;
//             }
//             _ if self.ir == 2027 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 2028 => {
//                 self.sbc(gd(&pins));
//                 fetch(&mut pins, self.pc);
//             }
//             /* INC abs,X (0xFE) */
//             _ if self.ir == 2032 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 2033 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 2034 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 2035 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 2036 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 2037 => {
//                 self.adl_adh += 1;
//                 self.nz(self.adl_adh as u8);
//                 sd(&mut pins, self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 2038 => {
//                 fetch(&mut pins, self.pc);
//             }
//             /* ISB abs,X (undoc) (0xFF) */
//             _ if self.ir == 2040 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//             }
//             _ if self.ir == 2041 => {
//                 sa(&mut pins, self.pc);
//                 self.pc += 1;
//                 self.adl_adh = gd(&pins) as u16;
//             }
//             _ if self.ir == 2042 => {
//                 self.adl_adh |= (gd(&pins) as u16) << 8;
//                 sa(
//                     &mut pins,
//                     (self.adl_adh & 0xFF00) | ((self.adl_adh + (self.x as u16)) & 0xFF),
//                 );
//             }
//             _ if self.ir == 2043 => {
//                 sa(&mut pins, self.adl_adh + (self.x as u16));
//             }
//             _ if self.ir == 2044 => {
//                 self.adl_adh = gd(&pins) as u16;
//                 wr(&mut pins);
//             }
//             _ if self.ir == 2045 => {
//                 self.adl_adh += 1;
//                 sd(&mut pins, self.adl_adh as u8);
//                 self.sbc(self.adl_adh as u8);
//                 wr(&mut pins);
//             }
//             _ if self.ir == 2046 => {
//                 fetch(&mut pins, self.pc);
//             }
//
//             _ => panic!(
//                 "This instruction does not exist: {:#04X}|{}!",
//                 self.ir >> 3,
//                 self.ir & 7
//             ),
//         }
//     }
// }

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

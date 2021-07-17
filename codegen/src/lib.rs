use proc_macro::{self, TokenStream};
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum StatusFlag {
    N,
    V,
    _X,
    _B,
    D,
    I,
    Z,
    C,
}

impl Display for StatusFlag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                StatusFlag::N => "N",
                StatusFlag::V => "V",
                StatusFlag::_X => "X",
                StatusFlag::_B => "B",
                StatusFlag::D => "D",
                StatusFlag::I => "I",
                StatusFlag::Z => "Z",
                StatusFlag::C => "C",
            }
        )
    }
}

fn flag_name<'z>(f: StatusFlag) -> &'z str {
    match f {
        StatusFlag::N => "N",
        StatusFlag::V => "V",
        StatusFlag::_X => "X",
        StatusFlag::_B => "B",
        StatusFlag::D => "D",
        StatusFlag::I => "I",
        StatusFlag::Z => "Z",
        StatusFlag::C => "C",
    }
}

fn branch_name(f: StatusFlag, nf: bool) -> String {
    match f {
        StatusFlag::N => (if nf { "BPL" } else { "BMI" }),
        StatusFlag::V => (if nf { "BVC" } else { "BVS" }),
        StatusFlag::C => (if nf { "BCC" } else { "BCS" }),
        StatusFlag::Z => (if nf { "BNE" } else { "BEQ" }),
        _ => panic!("Register {} does not have a branch operation!", f),
    }
    .to_owned()
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum AddressingMode {
    None,
    Imm,
    Zp,
    ZpX,
    ZpY,
    Abs,
    AbsX,
    AbsY,
    IdX,
    IdY,
    Jmp,
    Jsr,
    Invalid,
}

impl Display for AddressingMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AddressingMode::None => "",
                AddressingMode::Imm => "#",
                AddressingMode::Zp => "zp",
                AddressingMode::ZpX => "zp,X",
                AddressingMode::ZpY => "zp,Y",
                AddressingMode::Abs => "abs",
                AddressingMode::AbsX => "abs,X",
                AddressingMode::AbsY => "abs,Y",
                AddressingMode::IdX => "(zp,X)",
                AddressingMode::IdY => "(zp),Y",
                AddressingMode::Jmp => "",
                AddressingMode::Jsr => "",
                AddressingMode::Invalid => "INVALID",
            }
        )
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
enum MemoryAccess {
    None,
    R,
    W,
    RW,
}

const OPS: [[[(AddressingMode, MemoryAccess); 8]; 8]; 4] = [
    [
        [
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::Jsr, MemoryAccess::R),
            (AddressingMode::None, MemoryAccess::R),
            (AddressingMode::None, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
        ],
        [
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::W),
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::R),
        ],
        [
            (AddressingMode::None, MemoryAccess::W),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::W),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
        ],
        [
            (AddressingMode::Abs, MemoryAccess::R),
            (AddressingMode::Abs, MemoryAccess::R),
            (AddressingMode::Jmp, MemoryAccess::R),
            (AddressingMode::Jmp, MemoryAccess::R),
            (AddressingMode::Abs, MemoryAccess::W),
            (AddressingMode::Abs, MemoryAccess::R),
            (AddressingMode::Abs, MemoryAccess::R),
            (AddressingMode::Abs, MemoryAccess::R),
        ],
        [
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
        ],
        [
            (AddressingMode::ZpX, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::W),
            (AddressingMode::ZpX, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::R),
        ],
        [
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
        ],
        [
            (AddressingMode::AbsX, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::W),
            (AddressingMode::AbsX, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::R),
        ],
    ],
    [
        [
            (AddressingMode::IdX, MemoryAccess::R),
            (AddressingMode::IdX, MemoryAccess::R),
            (AddressingMode::IdX, MemoryAccess::R),
            (AddressingMode::IdX, MemoryAccess::R),
            (AddressingMode::IdX, MemoryAccess::W),
            (AddressingMode::IdX, MemoryAccess::R),
            (AddressingMode::IdX, MemoryAccess::R),
            (AddressingMode::IdX, MemoryAccess::R),
        ],
        [
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::W),
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::R),
        ],
        [
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
        ],
        [
            (AddressingMode::Abs, MemoryAccess::R),
            (AddressingMode::Abs, MemoryAccess::R),
            (AddressingMode::Abs, MemoryAccess::R),
            (AddressingMode::Abs, MemoryAccess::R),
            (AddressingMode::Abs, MemoryAccess::W),
            (AddressingMode::Abs, MemoryAccess::R),
            (AddressingMode::Abs, MemoryAccess::R),
            (AddressingMode::Abs, MemoryAccess::R),
        ],
        [
            (AddressingMode::IdY, MemoryAccess::R),
            (AddressingMode::IdY, MemoryAccess::R),
            (AddressingMode::IdY, MemoryAccess::R),
            (AddressingMode::IdY, MemoryAccess::R),
            (AddressingMode::IdY, MemoryAccess::W),
            (AddressingMode::IdY, MemoryAccess::R),
            (AddressingMode::IdY, MemoryAccess::R),
            (AddressingMode::IdY, MemoryAccess::R),
        ],
        [
            (AddressingMode::ZpX, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::W),
            (AddressingMode::ZpX, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::R),
        ],
        [
            (AddressingMode::AbsY, MemoryAccess::R),
            (AddressingMode::AbsY, MemoryAccess::R),
            (AddressingMode::AbsY, MemoryAccess::R),
            (AddressingMode::AbsY, MemoryAccess::R),
            (AddressingMode::AbsY, MemoryAccess::W),
            (AddressingMode::AbsY, MemoryAccess::R),
            (AddressingMode::AbsY, MemoryAccess::R),
            (AddressingMode::AbsY, MemoryAccess::R),
        ],
        [
            (AddressingMode::AbsX, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::W),
            (AddressingMode::AbsX, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::R),
        ],
    ],
    [
        [
            (AddressingMode::Invalid, MemoryAccess::RW),
            (AddressingMode::Invalid, MemoryAccess::RW),
            (AddressingMode::Invalid, MemoryAccess::RW),
            (AddressingMode::Invalid, MemoryAccess::RW),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
        ],
        [
            (AddressingMode::Zp, MemoryAccess::RW),
            (AddressingMode::Zp, MemoryAccess::RW),
            (AddressingMode::Zp, MemoryAccess::RW),
            (AddressingMode::Zp, MemoryAccess::RW),
            (AddressingMode::Zp, MemoryAccess::W),
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::RW),
            (AddressingMode::Zp, MemoryAccess::RW),
        ],
        [
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
        ],
        [
            (AddressingMode::Abs, MemoryAccess::RW),
            (AddressingMode::Abs, MemoryAccess::RW),
            (AddressingMode::Abs, MemoryAccess::RW),
            (AddressingMode::Abs, MemoryAccess::RW),
            (AddressingMode::Abs, MemoryAccess::W),
            (AddressingMode::Abs, MemoryAccess::R),
            (AddressingMode::Abs, MemoryAccess::RW),
            (AddressingMode::Abs, MemoryAccess::RW),
        ],
        [
            (AddressingMode::Invalid, MemoryAccess::RW),
            (AddressingMode::Invalid, MemoryAccess::RW),
            (AddressingMode::Invalid, MemoryAccess::RW),
            (AddressingMode::Invalid, MemoryAccess::RW),
            (AddressingMode::Invalid, MemoryAccess::W),
            (AddressingMode::Invalid, MemoryAccess::R),
            (AddressingMode::Invalid, MemoryAccess::RW),
            (AddressingMode::Invalid, MemoryAccess::RW),
        ],
        [
            (AddressingMode::ZpX, MemoryAccess::RW),
            (AddressingMode::ZpX, MemoryAccess::RW),
            (AddressingMode::ZpX, MemoryAccess::RW),
            (AddressingMode::ZpX, MemoryAccess::RW),
            (AddressingMode::ZpY, MemoryAccess::W),
            (AddressingMode::ZpY, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::RW),
            (AddressingMode::ZpX, MemoryAccess::RW),
        ],
        [
            (AddressingMode::None, MemoryAccess::R),
            (AddressingMode::None, MemoryAccess::R),
            (AddressingMode::None, MemoryAccess::R),
            (AddressingMode::None, MemoryAccess::R),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::None),
            (AddressingMode::None, MemoryAccess::R),
            (AddressingMode::None, MemoryAccess::R),
        ],
        [
            (AddressingMode::AbsX, MemoryAccess::RW),
            (AddressingMode::AbsX, MemoryAccess::RW),
            (AddressingMode::AbsX, MemoryAccess::RW),
            (AddressingMode::AbsX, MemoryAccess::RW),
            (AddressingMode::AbsY, MemoryAccess::W),
            (AddressingMode::AbsY, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::RW),
            (AddressingMode::AbsX, MemoryAccess::RW),
        ],
    ],
    [
        [
            (AddressingMode::IdX, MemoryAccess::RW),
            (AddressingMode::IdX, MemoryAccess::RW),
            (AddressingMode::IdX, MemoryAccess::RW),
            (AddressingMode::IdX, MemoryAccess::RW),
            (AddressingMode::IdX, MemoryAccess::W),
            (AddressingMode::IdX, MemoryAccess::R),
            (AddressingMode::IdX, MemoryAccess::RW),
            (AddressingMode::IdX, MemoryAccess::RW),
        ],
        [
            (AddressingMode::Zp, MemoryAccess::RW),
            (AddressingMode::Zp, MemoryAccess::RW),
            (AddressingMode::Zp, MemoryAccess::RW),
            (AddressingMode::Zp, MemoryAccess::RW),
            (AddressingMode::Zp, MemoryAccess::W),
            (AddressingMode::Zp, MemoryAccess::R),
            (AddressingMode::Zp, MemoryAccess::RW),
            (AddressingMode::Zp, MemoryAccess::RW),
        ],
        [
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
            (AddressingMode::Imm, MemoryAccess::R),
        ],
        [
            (AddressingMode::Abs, MemoryAccess::RW),
            (AddressingMode::Abs, MemoryAccess::RW),
            (AddressingMode::Abs, MemoryAccess::RW),
            (AddressingMode::Abs, MemoryAccess::RW),
            (AddressingMode::Abs, MemoryAccess::W),
            (AddressingMode::Abs, MemoryAccess::R),
            (AddressingMode::Abs, MemoryAccess::RW),
            (AddressingMode::Abs, MemoryAccess::RW),
        ],
        [
            (AddressingMode::IdY, MemoryAccess::RW),
            (AddressingMode::IdY, MemoryAccess::RW),
            (AddressingMode::IdY, MemoryAccess::RW),
            (AddressingMode::IdY, MemoryAccess::RW),
            (AddressingMode::IdY, MemoryAccess::RW),
            (AddressingMode::IdY, MemoryAccess::R),
            (AddressingMode::IdY, MemoryAccess::RW),
            (AddressingMode::IdY, MemoryAccess::RW),
        ],
        [
            (AddressingMode::ZpX, MemoryAccess::RW),
            (AddressingMode::ZpX, MemoryAccess::RW),
            (AddressingMode::ZpX, MemoryAccess::RW),
            (AddressingMode::ZpX, MemoryAccess::RW),
            (AddressingMode::ZpY, MemoryAccess::W),
            (AddressingMode::ZpY, MemoryAccess::R),
            (AddressingMode::ZpX, MemoryAccess::RW),
            (AddressingMode::ZpX, MemoryAccess::RW),
        ],
        [
            (AddressingMode::AbsY, MemoryAccess::RW),
            (AddressingMode::AbsY, MemoryAccess::RW),
            (AddressingMode::AbsY, MemoryAccess::RW),
            (AddressingMode::AbsY, MemoryAccess::RW),
            (AddressingMode::AbsY, MemoryAccess::W),
            (AddressingMode::AbsY, MemoryAccess::R),
            (AddressingMode::AbsY, MemoryAccess::RW),
            (AddressingMode::AbsY, MemoryAccess::RW),
        ],
        [
            (AddressingMode::AbsX, MemoryAccess::RW),
            (AddressingMode::AbsX, MemoryAccess::RW),
            (AddressingMode::AbsX, MemoryAccess::RW),
            (AddressingMode::AbsX, MemoryAccess::RW),
            (AddressingMode::AbsY, MemoryAccess::W),
            (AddressingMode::AbsY, MemoryAccess::R),
            (AddressingMode::AbsX, MemoryAccess::RW),
            (AddressingMode::AbsX, MemoryAccess::RW),
        ],
    ],
];

struct Opcode {
    code: usize,
    cmt: String,
    i: usize,
    src: [String; 8],
}

const STR_VAL: String = String::new();

impl Opcode {
    fn new(op: usize) -> Self {
        Opcode {
            code: op,
            cmt: "".to_owned(),
            i: 0,
            src: [STR_VAL; 8],
        }
    }

    fn t(&mut self, src: &str) {
        self.src[self.i] = src.to_owned();
        self.i += 1;
    }

    fn ta(&mut self, src: &str) {
        self.src[self.i - 1] += src;
    }

    fn write_op(&mut self, string: &mut String) {
        //"This instruction does not exist: {:#04X}|{}!",
        l(
            string,
            format!(
                "/* {} (0x{:#04X}) */",
                if self.cmt.is_empty() {
                    "???".to_owned()
                } else {
                    self.cmt.to_owned()
                },
                self.code
            ),
        );
        for t in 0..8 {
            if t < self.i {
                let zz = (self.code << 3) | t;
                l(
                    string,
                    format!("_ if self.ir == {} => {{{}}}, ", zz, self.src[t]),
                )
            }
        }
    }

    fn cmt(&mut self, cmd: &str) {
        let cc = self.code & 3;
        let bbb = (self.code >> 2) & 7;
        let aaa = (self.code >> 5) & 7;
        let (addr_mode, _) = OPS[cc][bbb][aaa];
        if addr_mode != AddressingMode::None
            && addr_mode != AddressingMode::Jmp
            && addr_mode != AddressingMode::Jsr
        {
            self.cmt = format!("{} {}", cmd, addr_mode);
        } else {
            self.cmt = cmd.to_owned();
        }
    }

    fn u_cmt(&mut self, cmd: &str) {
        self.cmt(cmd);
        self.cmt += " (undoc)"
    }

    fn invalid_opcode(&self) -> bool {
        let cc = self.code & 3;
        let bbb = (self.code >> 2) & 7;
        let aaa = (self.code >> 5) & 7;
        let (addr_mode, _) = OPS[cc][bbb][aaa];
        addr_mode == AddressingMode::Invalid
    }

    fn enc_addr(&mut self, addr_mode: AddressingMode, mem_access: MemoryAccess) {
        match addr_mode {
            // no addressing, this still puts the PC on the address bus without
            // incrementing the PC
            AddressingMode::None => self.t("sa(&mut pins, self.pc);"),
            AddressingMode::Imm => self.t("sa(&mut pins, self.pc);self.pc+=1;"),
            AddressingMode::Zp => {
                self.t("sa(&mut pins, self.pc);self.pc+=1;");
                self.t("let zz = ga(&pins);sa(&mut pins, zz);");
            }
            AddressingMode::ZpX => {
                self.t("sa(&mut pins, self.pc);self.pc+=1;");
                self.t("self.adl_adh = ga(&pins);sa(&mut pins, self.adl_adh);");
                self.t("sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);");
            }
            AddressingMode::ZpY => {
                self.t("sa(&mut pins, self.pc);self.pc+=1;");
                self.t("self.adl_adh = ga(&pins);sa(&mut pins, self.adl_adh);");
                self.t("sa(&mut pins, (self.adl_adh+(self.y as u16))&0x00FF);");
            }
            AddressingMode::Abs => {
                self.t("sa(&mut pins, self.pc);self.pc+=1;");
                self.t("sa(&mut pins, self.pc);self.pc+=1;self.adl_adh = gd(&pins) as u16;");
                self.t("let zz = gd(&pins);sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);");
            }
            AddressingMode::AbsX => {
                // absolute + X
                // this needs to check if a page boundary is crossed, which costs
                // and additional cycle, but this early-out only happens when the
                // instruction doesn"t need to write back to memory
                self.t("sa(&mut pins, self.pc);self.pc+=1;");
                self.t("sa(&mut pins, self.pc);self.pc+=1;self.adl_adh = gd(&pins) as u16;");
                self.t("self.adl_adh|=(gd(&pins)as u16)<<8;sa(&mut pins, (self.adl_adh&0xFF00)|((self.adl_adh+(self.x as u16))&0xFF));");
                if mem_access == MemoryAccess::R {
                    self.ta("self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8)))&1;");
                }
                self.t("sa(&mut pins, self.adl_adh+(self.x as u16));");
            }
            AddressingMode::AbsY => {
                self.t("sa(&mut pins, self.pc);self.pc+=1;");
                self.t("sa(&mut pins, self.pc);self.pc+=1;self.adl_adh = gd(&pins) as u16;");
                self.t("self.adl_adh|=(gd(&pins)as u16)<<8;sa(&mut pins, (self.adl_adh&0xFF00)|((self.adl_adh+(self.y as u16))&0xFF));");
                if mem_access == MemoryAccess::R {
                    // skip next tick if read access and page not crossed
                    self.ta("self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8)))&1;");
                }
                self.t("sa(&mut pins, self.adl_adh+(self.y as u16));");
            }
            AddressingMode::IdX => {
                self.t("sa(&mut pins, self.pc);self.pc+=1;");
                self.t("self.adl_adh = ga(&pins);sa(&mut pins, self.adl_adh);");
                self.t("self.adl_adh = (self.adl_adh+(self.x as u16))&0xFF;sa(&mut pins, self.adl_adh);");
                self.t("sa(&mut pins, (self.adl_adh+1) & 0xFF); self.adl_adh = ga(&pins);");
                self.t("let zz = gd(&pins);sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);");
            }
            AddressingMode::IdY => {
                self.t("sa(&mut pins, self.pc);self.pc+=1;");
                self.t("self.adl_adh = ga(&pins);sa(&mut pins, self.adl_adh);");
                self.t("sa(&mut pins, (self.adl_adh+1) & 0xFF); self.adl_adh = ga(&pins);");
                self.t("self.adl_adh|=(gd(&pins)as u16)<<8;sa(&mut pins, (self.adl_adh&0xFF00)|((self.adl_adh+(self.y as u16))&0xFF));");
                if mem_access == MemoryAccess::R {
                    // skip next tick if read access and page not crossed
                    self.ta("self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8)))&1;");
                }
                self.t("sa(&mut pins, self.adl_adh+(self.y as u16));");
            }
            AddressingMode::Jmp => {}
            AddressingMode::Jsr => {}
            AddressingMode::Invalid => {}
        }
    }
}

impl Opcode {
    //-------------------------------------------------------------------------------
    fn i_brk(&mut self) {
        self.cmt("BRK");
        self.t("if !self.brk_flags.contains(BreakFlags::NMI) && !self.brk_flags.contains(BreakFlags::IRQ) { self.pc += 1;                }                sad(&mut pins, 0x0100 | self.sp as u16, (self.pc >> 8) as u8);                self.sp = (Wrapping(self.sp) - Wrapping(1)).0;                if !self.brk_flags.contains(BreakFlags::RESET) {                    wr(&mut pins)                }           ");
        self.t("sad(&mut pins, 0x0100 | self.sp as u16, (self.pc) as u8);self.sp = (Wrapping(self.sp) - Wrapping(1)).0;if !self.brk_flags.contains(BreakFlags::RESET) {wr(&mut pins)}");
        self.t("sad(&mut pins, 0x0100 | self.sp as u16, self.sr.bits | StatusRegister::X.bits);self.sp = (Wrapping(self.sp) - Wrapping(1)).0;if self.brk_flags.contains(BreakFlags::RESET) {self.adl_adh = 0xFFFC;} else {wr(&mut pins);if self.brk_flags.contains(BreakFlags::NMI) {self.adl_adh = 0xFFFA} else {self.adl_adh = 0xFFFE}}");
        self.t("sa(&mut pins, self.adl_adh);self.adl_adh += 1;self.sr.set(StatusRegister::I | StatusRegister::B, true);self.brk_flags = BreakFlags::empty();");
        self.t("sa(&mut pins, self.adl_adh);self.adl_adh = gd(&pins) as u16; /* NMI \"half-hijacking\" not possible */");
        self.t("self.pc = ((gd(&pins) as u16) << 8) | self.adl_adh;");
    }
    //-------------------------------------------------------------------------------
    fn i_nop(&mut self) {
        self.cmt("NOP");
        self.t("");
    }
    //-------------------------------------------------------------------------------
    fn u_nop(&mut self) {
        self.u_cmt("NOP");
        self.t("");
    }
    //-------------------------------------------------------------------------------
    fn i_lda(&mut self) {
        self.cmt("LDA");
        self.t("self.ac=gd(&pins);self.nz(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn i_ldx(&mut self) {
        self.cmt("LDX");
        self.t("self.x=gd(&pins);self.nz(self.x);");
    }
    //-------------------------------------------------------------------------------
    fn i_ldy(&mut self) {
        self.cmt("LDY");
        self.t("self.y=gd(&pins);self.nz(self.y);");
    }
    //-------------------------------------------------------------------------------
    fn u_lax(&mut self) {
        self.u_cmt("LAX");
        self.t("self.ac = gd(&pins); self.x = gd(&pins);self.nz(self.x);");
    }
    //-------------------------------------------------------------------------------
    fn x_lxa(&mut self) {
        //undocumented LXA
        //and immediate byte with A, then load X with A
        self.u_cmt("LXA");
        self.t("let zz = (self.ac|0xEE)&gd(&pins);self.ac=zz;self.x=zz;self.nz(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn i_sta(&mut self) {
        self.cmt("STA");
        self.ta("sd(&mut pins, self.ac); wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_stx(&mut self) {
        self.cmt("STX");
        self.ta("sd(&mut pins, self.x); wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_sty(&mut self) {
        self.cmt("STY");
        self.ta("sd(&mut pins, self.y); wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn u_sax(&mut self) {
        self.u_cmt("SAX");
        self.ta("sd(&mut pins, self.ac&self.x); wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_tax(&mut self) {
        self.cmt("TAX");
        self.t("self.x=self.ac;self.nz(self.x);");
    }
    //-------------------------------------------------------------------------------
    fn i_tay(&mut self) {
        self.cmt("TAY");
        self.t("self.y=self.ac;self.nz(self.y);");
    }
    //-------------------------------------------------------------------------------
    fn i_txa(&mut self) {
        self.cmt("TXA");
        self.t("self.ac=self.x;self.nz(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn i_tya(&mut self) {
        self.cmt("TYA");
        self.t("self.ac=self.x;self.nz(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn i_txs(&mut self) {
        self.cmt("TXS");
        self.t("self.sp=self.x;");
    }
    //-------------------------------------------------------------------------------
    fn i_tsx(&mut self) {
        self.cmt("TSX");
        self.t("self.x=self.sp;self.nz(self.x);");
    }
    //-------------------------------------------------------------------------------
    fn i_php(&mut self) {
        self.cmt("PHP");
        self.t("sad(&mut pins, 0x0100|(self.sp as u16), self.sr.bits|StatusRegister::X.bits);self.sp-=1;wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_plp(&mut self) {
        self.cmt("PLP");
        self.t("sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;"); //read junk byte from current SP
        self.t("sa(&mut pins, 0x0100|(self.sp as u16));"); //read actual byte
        self.t("self.sr = StatusRegister::from_bits_truncate((gd(&pins)|StatusRegister::B.bits)&!StatusRegister::X.bits);");
    }
    //-------------------------------------------------------------------------------
    fn i_pha(&mut self) {
        self.cmt("PHA");
        self.t("sad(&mut pins, 0x0100|(self.sp as u16), self.ac);self.sp-=1;wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_pla(&mut self) {
        self.cmt("PLA");
        self.t("sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;"); //read junk byte from current SP
        self.t("sa(&mut pins, 0x0100|(self.sp as u16));"); //read actual byte
        self.t("self.ac=gd(&pins);self.nz(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn i_se(&mut self, f: StatusFlag) {
        self.cmt(("SE".to_owned() + flag_name(f)).as_str());
        self.t(("self.sr.set(StatusRegister::".to_owned() + flag_name(f) + ", true);").as_str());
    }
    //-------------------------------------------------------------------------------
    fn i_cl(&mut self, f: StatusFlag) {
        self.cmt(("CL".to_owned() + flag_name(f)).as_str());
        self.t(("self.sr.set(StatusRegister::".to_owned() + flag_name(f) + ", false);").as_str());
    }
    //-------------------------------------------------------------------------------
    fn i_br(&mut self, f: StatusFlag, nf: bool) {
        self.cmt(branch_name(f, nf).as_str());
        //if branch not taken?
        self.t(("sa(&mut pins, self.pc);self.adl_adh=self.pc+(gd(&pins) as u16); if self.sr.bitand(StatusRegister::".to_owned() + flag_name(f)+").bits !="+if nf { "1" }else {"0"} +" { fetch(&mut pins, self.pc) }").as_str());
        //branch taken: shortcut if page not crossed, "branchquirk" interrupt fix
        self.t("sa(&mut pins, (self.pc & 0xFF00)|(self.adl_adh&0x00FF));if self.adl_adh&0xFF00 == self.pc &0xFF00 { self.pc=self.adl_adh;self.irq_pip>>=1;self.nmi_pip>>=1;fetch(&mut pins, self.pc) }");
        //page crossed extra cycle{
        self.t("self.pc=self.adl_adh;");
    }
    //-------------------------------------------------------------------------------
    fn i_jmp(&mut self) {
        self.cmt("JMP");
        self.t("sa(&mut pins, self.pc);self.pc+=1;");
        self.t("sa(&mut pins, self.pc);self.pc+=1;self.adl_adh = gd(&pins) as u16;");
        self.t("self.pc = ((gd(&pins) as u16)<<8)|self.adl_adh;");
    }
    //-------------------------------------------------------------------------------
    fn i_jmpi(&mut self) {
        self.cmt("JMPI");
        self.t("sa(&mut pins, self.pc);self.pc+=1;");
        self.t("sa(&mut pins, self.pc);self.pc+=1;self.adl_adh = gd(&pins) as u16;");
        self.t("self.adl_adh|=(gd(&pins) as u16)<<8;sa(&mut pins, self.adl_adh);");
        self.t("sa(&mut pins, (self.adl_adh&0xFF00)|((self.adl_adh+1)&0x00FF));self.adl_adh = gd(&pins) as u16;");
        self.t("self.pc = ((gd(&pins) as u16)<<8)|self.adl_adh;");
    }
    //-------------------------------------------------------------------------------
    fn i_jsr(&mut self) {
        self.cmt("JSR");
        //read low byte of target address
        self.t("sa(&mut pins, self.pc);self.pc+=1;");
        //put SP on addr bus, next cycle is a junk read
        self.t("sa(&mut pins, 0x0100|(self.sp as u16));self.adl_adh = gd(&pins) as u16;");
        //write PC high byte to stack
        self.t(
            "sad(&mut pins, 0x0100|(self.sp as u16), (self.pc>>8) as u8);self.sp-=1;wr(&mut pins);",
        );
        //write PC low byte to stack
        self.t("sad(&mut pins, 0x0100|(self.sp as u16), self.pc as u8);self.sp-=1;wr(&mut pins);");
        //load target address high byte
        self.t("sa(&mut pins, self.pc);");
        //load PC and done
        self.t("self.pc = ((gd(&pins) as u16)<<8)|self.adl_adh;");
    }
    //-------------------------------------------------------------------------------
    fn i_rts(&mut self) {
        self.cmt("RTS");
        //put SP on stack and do a junk read
        self.t("sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;");
        //load return address low byte from stack
        self.t("sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;");
        //load return address high byte from stack
        self.t("sa(&mut pins, 0x0100|(self.sp as u16));self.adl_adh = gd(&pins) as u16;");
        //put return address in PC, this is one byte before next self, do junk read from PC
        self.t("self.pc = ((gd(&pins) as u16)<<8)|self.adl_adh;sa(&mut pins, self.pc);self.pc+=1;");
        //next tick is selfcode fetch
        self.t("");
    }
    //-------------------------------------------------------------------------------
    fn i_rti(&mut self) {
        self.cmt("RTI");
        //put SP on stack and do a junk read
        self.t("sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;");
        //load processor status flag from stack
        self.t("sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;");
        //load return address low byte from stack
        self.t("sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;self.sr = StatusRegister::from_bits_truncate((gd(&pins)|StatusRegister::B.bits)&!StatusRegister::X.bits);");
        //load return address high byte from stack
        self.t("sa(&mut pins, 0x0100|(self.sp as u16));self.adl_adh = gd(&pins) as u16;");
        //update PC (which is already placed on the right return-to instruction);
        self.t("self.pc = ((gd(&pins) as u16)<<8)|self.adl_adh;");
    }
    //-------------------------------------------------------------------------------
    fn i_ora(&mut self) {
        self.cmt("ORA");
        self.t("self.ac|=gd(&pins);self.nz(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn i_and(&mut self) {
        self.cmt("AND");
        self.t("self.ac&=gd(&pins);self.nz(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn i_eor(&mut self) {
        self.cmt("EOR");
        self.t("self.ac^=gd(&pins);self.nz(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn i_adc(&mut self) {
        self.cmt("ADC");
        self.t("self.adc(gd(&pins));");
    }
    //-------------------------------------------------------------------------------
    fn i_sbc(&mut self) {
        self.cmt("SBC");
        self.t("self.sbc(gd(&pins));");
    }
    //-------------------------------------------------------------------------------
    fn u_sbc(&mut self) {
        self.u_cmt("SBC");
        self.t("self.sbc(gd(&pins));");
    }
    //-------------------------------------------------------------------------------
    fn i_cmp(&mut self) {
        self.cmt("CMP");
        self.t("self.cmp(self.ac, gd(&pins));");
    }
    //-------------------------------------------------------------------------------
    fn i_cpx(&mut self) {
        self.cmt("CPX");
        self.t("self.cmp(self.x, gd(&pins));");
    }
    //-------------------------------------------------------------------------------
    fn i_cpy(&mut self) {
        self.cmt("CPY");
        self.t("self.cmp(self.y, gd(&pins));");
    }
    //-------------------------------------------------------------------------------
    fn u_dcp(&mut self) {
        //undocumented "decrement and compare"
        self.u_cmt("DCP");
        self.t("self.adl_adh = gd(&pins) as u16;wr(&mut pins);");
        self.t("self.adl_adh-=1;self.nz(self.adl_adh as u8);sd(&mut pins, self.adl_adh as u8);self.cmp(self.ac, self.adl_adh as u8);wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn x_sbx(&mut self) {
        self.u_cmt("SBX");
        self.t("self.sbx(gd(&pins));");
    }
    //-------------------------------------------------------------------------------
    fn i_dec(&mut self) {
        self.cmt("DEC");
        self.t("self.adl_adh = gd(&pins) as u16;wr(&mut pins);");
        self.t("self.adl_adh-=1; self.nz(self.adl_adh as u8);sd(&mut pins, self.adl_adh as u8);wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_inc(&mut self) {
        self.cmt("INC");
        self.t("self.adl_adh = gd(&pins) as u16;wr(&mut pins);");
        self.t("self.adl_adh+=1; self.nz(self.adl_adh as u8);sd(&mut pins, self.adl_adh as u8);wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_dex(&mut self) {
        self.cmt("DEX");
        self.t("self.x-=1; self.nz(self.x);");
    }
    //-------------------------------------------------------------------------------
    fn i_dey(&mut self) {
        self.cmt("DEY");
        self.t("self.y-=1; self.nz(self.y);");
    }
    //-------------------------------------------------------------------------------
    fn i_inx(&mut self) {
        self.cmt("INX");
        self.t("self.x+=1; self.nz(self.x);");
    }
    //-------------------------------------------------------------------------------
    fn i_iny(&mut self) {
        self.cmt("INY");
        self.t("self.y+=1; self.nz(self.y);");
    }
    //-------------------------------------------------------------------------------
    fn u_isb(&mut self) {
        //undocumented INC+SBC instruction
        self.u_cmt("ISB");
        self.t("self.adl_adh = gd(&pins) as u16;wr(&mut pins);");
        self.t("self.adl_adh+=1;sd(&mut pins, self.adl_adh as u8); self.sbc(self.adl_adh as u8);wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_asl(&mut self) {
        self.cmt("ASL");
        self.t("self.adl_adh = gd(&pins) as u16;wr(&mut pins);");
        self.t("sd(&mut pins, self.asl(self.adl_adh as u8));wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_asla(&mut self) {
        self.cmt("ASLA");
        self.t("self.ac = self.asl(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn i_lsr(&mut self) {
        self.cmt("LSR");
        self.t("self.adl_adh = gd(&pins) as u16;wr(&mut pins);");
        self.t("sd(&mut pins, self.lsr(self.adl_adh as u8));wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_lsra(&mut self) {
        self.cmt("LSRA");
        self.t("self.ac = self.asl(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn u_slo(&mut self) {
        //undocumented ASL+OR
        self.u_cmt("SLO");
        self.t("self.adl_adh = gd(&pins) as u16;wr(&mut pins);");
        self.t("self.adl_adh = self.asl(self.adl_adh as u8) as u16;sd(&mut pins, self.adl_adh as u8); self.ac|=self.adl_adh as u8;self.nz(self.ac);wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn x_asr(&mut self) {
        //undocumented AND+LSR
        self.u_cmt("ASR");
        self.t("self.ac=gd(&pins);self.ac = self.asl(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn u_sre(&mut self) {
        //undocumented LSR+EOR
        self.u_cmt("SRE");
        self.t("self.adl_adh = gd(&pins) as u16;wr(&mut pins);");
        self.t("self.adl_adh = self.lsr(self.adl_adh as u8) as u16;sd(&mut pins, self.adl_adh as u8); self.ac^=self.adl_adh as u8;self.nz(self.ac);wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_rol(&mut self) {
        self.cmt("ROL");
        self.t("self.adl_adh = gd(&pins) as u16;wr(&mut pins);");
        self.t("sd(&mut pins, self.rol(self.adl_adh as u8));wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_rola(&mut self) {
        self.cmt("ROLA");
        self.t("self.ac=self.rol(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn u_rla(&mut self) {
        //uncodumented ROL+AND
        self.u_cmt("RLA");
        self.t("self.adl_adh = gd(&pins) as u16;wr(&mut pins);");
        self.t("self.adl_adh = self.rol(self.adl_adh as u8) as u16;sd(&mut pins, self.adl_adh as u8);self.ac &=self.adl_adh as u8;self.nz(self.ac);wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_ror(&mut self) {
        self.cmt("ROR");
        self.t("self.adl_adh = gd(&pins) as u16;wr(&mut pins);");
        self.t("sd(&mut pins, self.ror(self.adl_adh as u8));wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn i_rora(&mut self) {
        self.cmt("RORA");
        self.t("self.ac=self.ror(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn u_rra(&mut self) {
        //undocumented ROR+ADC
        self.u_cmt("RRA");
        self.t("self.adl_adh = gd(&pins) as u16;wr(&mut pins);");
        self.t("self.adl_adh = self.ror(self.adl_adh as u8) as u16;sd(&mut pins, self.adl_adh as u8);self.adc(self.adl_adh as u8);wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn x_arr(&mut self) {
        //undocumented AND+ROR
        self.u_cmt("ARR");
        self.t("self.ac=gd(&pins);self.arr();");
    }
    //-------------------------------------------------------------------------------
    fn x_ane(&mut self) {
        //undocumented ANE
        self.u_cmt("ANE");
        self.t("self.ac = (self.ac|0xEE)&self.x&gd(&pins);self.nz(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn x_sha(&mut self) {
        //undocumented SHA
        // stores the result of A AND X AND the high byte of the target address of
        // the selferand +1 in memory
        self.u_cmt("SHA");
        self.ta("let zz = ((ga(&pins) >> 8) + 1) as u8;sd(&mut pins, self.ac & self.x & zz);wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn x_shx(&mut self) {
        //undocumented SHX
        //AND X register with the high byte of the target address of the
        //argument + 1. Store the result in memory.
        self.u_cmt("SHX");
        self.ta("let zz = ((ga(&pins) >> 8) + 1) as u8;sd(&mut pins, self.x & zz);wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn x_shy(&mut self) {
        //undocumented SHX
        //AND Y register with the high byte of the target address of the
        //argument + 1. Store the result in memory.
        self.u_cmt("SHY");
        self.ta("let zz = ((ga(&pins) >> 8) + 1) as u8;sd(&mut pins, self.y & zz);wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn x_shs(&mut self) {
        //undocumented SHS
        //AND X register with accumulator and store result in stack pointer, then
        //AND stack pointer with the high byte of the target address of the
        //argument + 1. Store result in memory.
        self.u_cmt("SHS");
        self.ta("self.sp=self.ac & self.x;let zz = ((ga(&pins) >> 8) + 1) as u8;sd(&mut pins, self.sp&zz);wr(&mut pins);");
    }
    //-------------------------------------------------------------------------------
    fn x_anc(&mut self) {
        //undocumented ANC
        //AND byte with accumulator. If result is negative then carry is set.
        self.u_cmt("ANC");
        self.t("self.ac = gd(&pins);self.nz(self.ac);self.sr.set(StatusRegister::C, (self.ac&0x80)!=0);");
    }
    //-------------------------------------------------------------------------------
    fn x_las(&mut self) {
        //undocumented LAS
        //AND memory with stack pointer, transfer result to accumulator, X
        //register and stack pointer.
        self.u_cmt("LAS");
        self.t("let zz = gd(&pins)&self.sp;self.ac=zz;self.x=zz;self.sp=zz;self.nz(self.ac);");
    }
    //-------------------------------------------------------------------------------
    fn x_jam(&mut self) {
        //undocumented JAM, next selfcode byte read, data and addr bus set to all 1, execution stselfs
        self.u_cmt("JAM");
        self.t("sa(&mut pins, self.pc);");
        self.t("sad(&mut pins, 0xFFFF,0xFF);self.ir-=1;");
    }
    //-------------------------------------------------------------------------------
    fn i_bit(&mut self) {
        self.cmt("BIT");
        self.t("self.bit(gd(&pins));");
    }
}

fn l(string: &mut String, l: String) {
    string.push_str(&l);
}

fn enc_op(op: usize) -> Opcode {
    let mut o = Opcode::new(op);
    if o.invalid_opcode() {
        o.x_jam();
        return o;
    }
    let cc = o.code & 3;
    let bbb = (o.code >> 2) & 7;
    let aaa = (o.code >> 5) & 7;
    let (addr_mode, mem_access) = OPS[cc][bbb][aaa];
    o.enc_addr(addr_mode, mem_access);
    // yeet

    if cc == 0 {
        if aaa == 0 {
            if bbb == 0 {
                o.i_brk();
            } else if bbb == 2 {
                o.i_php();
            } else if bbb == 4 {
                o.i_br(StatusFlag::N, false) // BPL
            } else if bbb == 6 {
                o.i_cl(StatusFlag::C)
            } else {
                o.u_nop();
            }
        } else if aaa == 1 {
            if bbb == 0 {
                o.i_jsr();
            } else if bbb == 2 {
                o.i_plp();
            } else if bbb == 4 {
                o.i_br(StatusFlag::N, true) // BMI
            } else if bbb == 6 {
                o.i_se(StatusFlag::C)
            } else if bbb == 5 || bbb == 6 || bbb == 7 {
                o.u_nop();
            } else {
                o.i_bit();
            }
        } else if aaa == 2 {
            if bbb == 0 {
                o.i_rti();
            } else if bbb == 2 {
                o.i_pha();
            } else if bbb == 3 {
                o.i_jmp();
            } else if bbb == 4 {
                o.i_br(StatusFlag::V, false) // BVC
            } else if bbb == 6 {
                o.i_cl(StatusFlag::I)
            } else {
                o.u_nop();
            }
        } else if aaa == 3 {
            if bbb == 0 {
                o.i_rts();
            } else if bbb == 2 {
                o.i_pla();
            } else if bbb == 3 {
                o.i_jmpi();
            } else if bbb == 4 {
                o.i_br(StatusFlag::V, true) // BVS
            } else if bbb == 6 {
                o.i_se(StatusFlag::I)
            } else {
                o.u_nop();
            }
        } else if aaa == 4 {
            if bbb == 0 {
                o.u_nop();
            } else if bbb == 2 {
                o.i_dey();
            } else if bbb == 4 {
                o.i_br(StatusFlag::C, false) // BCC
            } else if bbb == 6 {
                o.i_tya();
            } else if bbb == 7 {
                o.x_shy();
            } else {
                o.i_sty();
            }
        } else if aaa == 5 {
            if bbb == 2 {
                o.i_tay();
            } else if bbb == 4 {
                o.i_br(StatusFlag::C, true) // BCS
            } else if bbb == 6 {
                o.i_cl(StatusFlag::V)
            } else {
                o.i_ldy();
            }
        } else if aaa == 6 {
            if bbb == 2 {
                o.i_iny();
            } else if bbb == 4 {
                o.i_br(StatusFlag::Z, false) // BNE
            } else if bbb == 6 {
                o.i_cl(StatusFlag::D)
            } else if bbb == 5 || bbb == 6 || bbb == 7 {
                o.u_nop();
            } else {
                o.i_cpy();
            }
        } else if aaa == 7 {
            if bbb == 2 {
                o.i_inx();
            } else if bbb == 4 {
                o.i_br(StatusFlag::Z, true) // BEQ
            } else if bbb == 6 {
                o.i_se(StatusFlag::D)
            } else if bbb == 5 || bbb == 6 || bbb == 7 {
                o.u_nop();
            } else {
                o.i_cpx();
            }
        }
    } else if cc == 1 {
        if aaa == 0 {
            o.i_ora();
        } else if aaa == 1 {
            o.i_and();
        } else if aaa == 2 {
            o.i_eor();
        } else if aaa == 3 {
            o.i_adc();
        } else if aaa == 4 {
            if bbb == 2 {
                o.u_nop();
            } else {
                o.i_sta();
            }
        } else if aaa == 5 {
            o.i_lda();
        } else if aaa == 6 {
            o.i_cmp();
        } else {
            o.i_sbc();
        }
    } else if cc == 2 {
        if aaa == 0 {
            if bbb == 2 {
                o.i_asla();
            } else if bbb == 6 {
                o.u_nop();
            } else {
                o.i_asl();
            }
        } else if aaa == 1 {
            if bbb == 2 {
                o.i_rola();
            } else if bbb == 6 {
                o.u_nop();
            } else {
                o.i_rol();
            }
        } else if aaa == 2 {
            if bbb == 2 {
                o.i_lsra();
            } else if bbb == 6 {
                o.u_nop();
            } else {
                o.i_lsr();
            }
        } else if aaa == 3 {
            if bbb == 2 {
                o.i_rora();
            } else if bbb == 6 {
                o.u_nop();
            } else {
                o.i_ror();
            }
        } else if aaa == 4 {
            if bbb == 0 {
                o.u_nop();
            } else if bbb == 2 {
                o.i_txa();
            } else if bbb == 6 {
                o.i_txs();
            } else if bbb == 7 {
                o.x_shx();
            } else {
                o.i_stx();
            }
        } else if aaa == 5 {
            if bbb == 2 {
                o.i_tax();
            } else if bbb == 6 {
                o.i_tsx();
            } else {
                o.i_ldx();
            }
        } else if aaa == 6 {
            if bbb == 2 {
                o.i_dex();
            } else if bbb != 7 {
                o.u_nop();
            } else {
                o.i_dec();
            }
        } else if aaa == 7 {
            if bbb == 2 {
                o.i_nop();
            } else if bbb != 7 {
                o.u_nop();
            } else {
                o.i_inc();
            }
        }
    } else if cc == 3 {
        // undocumented block
        if aaa == 0 {
            if bbb == 2 {
                o.x_anc();
            } else {
                o.u_slo();
            }
        } else if aaa == 1 {
            if bbb == 2 {
                o.x_anc();
            } else {
                o.u_rla();
            }
        } else if aaa == 2 {
            if bbb == 2 {
                o.x_asr();
            } else {
                o.u_sre();
            }
        } else if aaa == 3 {
            if bbb == 2 {
                o.x_arr();
            } else {
                o.u_rra();
            }
        } else if aaa == 4 {
            if bbb == 2 {
                o.x_ane();
            } else if bbb == 6 {
                o.x_shs();
            } else if bbb == 4 || bbb == 5 || bbb == 6 || bbb == 7 {
                o.x_sha();
            } else {
                o.u_sax();
            }
        } else if aaa == 5 {
            if bbb == 2 {
                o.x_lxa();
            } else if bbb == 6 {
                o.x_las();
            } else {
                o.u_lax();
            }
        } else if aaa == 6 {
            if bbb == 2 {
                o.x_sbx();
            } else {
                o.u_dcp();
            }
        } else if aaa == 7 {
            if bbb == 2 {
                o.u_sbc();
            } else {
                o.u_isb();
            }
        }
    }

    if mem_access == MemoryAccess::R || mem_access == MemoryAccess::None {
        o.ta("fetch(&mut pins, self.pc);")
    } else {
        o.t("fetch(&mut pins, self.pc);")
    }
    o
}

#[proc_macro]
pub fn codegen(_: TokenStream) -> TokenStream {
    let mut code = String::new();
    for op in 0..256 {
        enc_op(op).write_op(&mut code);
    }
    ("use std::ops::BitAnd;impl CPU {\n    fn the_match_statement(&mut self, mut pins: Pins){\n    match 0 {\n        ".to_owned() + code.as_str() +"\n         _ => panic!(\n        \"This instruction does not exist: {:#04X}|{}!\",\n        self.ir >> 3,\n        self.ir & 7\n      ),\n    }\n  }\n}").parse().unwrap()
}

#[cfg(test)]
mod tests {
    #[test]
    fn itWorks() {
        assert_eq!(2 + 2, 4);
    }
}

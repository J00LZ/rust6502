#-------------------------------------------------------------------------------
#   m6502_decoder.py
#   Generate instruction decoder for m6502.h emulator.
#   Borrowed (stolen) from https://github.com/floooh/chips/blob/master/codegen/m6502_gen.py
#-------------------------------------------------------------------------------
import sys
from string import Template

InpPath = 'in.rs'
OutPath = 'out.rs'

# flag bits
CF = (1<<0)
ZF = (1<<1)
IF = (1<<2)
DF = (1<<3)
BF = (1<<4)
XF = (1<<5)
VF = (1<<6)
NF = (1<<7)

def flag_name(f):
    if f == CF: return 'C'
    elif f == ZF: return 'Z'
    elif f == IF: return 'I'
    elif f == DF: return 'D'
    elif f == BF: return 'B'
    elif f == XF: return 'X'
    elif f == VF: return 'V'
    elif f == NF: return 'N'

def branch_name(m, v):
    if m == NF:
        return 'BPL' if v==0 else 'BMI'
    elif m == VF:
        return 'BVC' if v==0 else 'BVS'
    elif m == CF:
        return 'BCC' if v==0 else 'BCS'
    elif m == ZF:
        return 'BNE' if v==0 else 'BEQ'

# addressing mode constants
A____ = 0       # no addressing mode
A_IMM = 1       # immediate
A_ZER = 2       # zero-page
A_ZPX = 3       # zp,X
A_ZPY = 4       # zp,Y
A_ABS = 5       # abs
A_ABX = 6       # abs,X
A_ABY = 7       # abs,Y
A_IDX = 8       # (zp,X)
A_IDY = 9       # (zp),Y
A_JMP = 10      # special JMP abs
A_JSR = 11      # special JSR abs
A_INV = 12      # an invalid instruction

# addressing mode strings
addr_mode_str = ['', '#', 'zp', 'zp,X', 'zp,Y', 'abs', 'abs,X', 'abs,Y', '(zp,X)', '(zp),Y', '', '', 'INVALID']

# memory access modes
M___ = 0        # no memory access
M_R_ = 1        # read access
M__W = 2        # write access
M_RW = 3        # read-modify-write

# addressing-modes and memory accesses for each instruction
ops = [
    # cc = 00
    [
        # ---         BIT          JMP          JMP()        STY          LDY          CPY          CPX
        [[A____,M___],[A_JSR,M_R_],[A____,M_R_],[A____,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_]],
        [[A_ZER,M_R_],[A_ZER,M_R_],[A_ZER,M_R_],[A_ZER,M_R_],[A_ZER,M__W],[A_ZER,M_R_],[A_ZER,M_R_],[A_ZER,M_R_]],
        [[A____,M__W],[A____,M___],[A____,M__W],[A____,M___],[A____,M___],[A____,M___],[A____,M___],[A____,M___]],
        [[A_ABS,M_R_],[A_ABS,M_R_],[A_JMP,M_R_],[A_JMP,M_R_],[A_ABS,M__W],[A_ABS,M_R_],[A_ABS,M_R_],[A_ABS,M_R_]],
        [[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_]],  # relative branches
        [[A_ZPX,M_R_],[A_ZPX,M_R_],[A_ZPX,M_R_],[A_ZPX,M_R_],[A_ZPX,M__W],[A_ZPX,M_R_],[A_ZPX,M_R_],[A_ZPX,M_R_]],
        [[A____,M___],[A____,M___],[A____,M___],[A____,M___],[A____,M___],[A____,M___],[A____,M___],[A____,M___]],
        [[A_ABX,M_R_],[A_ABX,M_R_],[A_ABX,M_R_],[A_ABX,M_R_],[A_ABX,M__W],[A_ABX,M_R_],[A_ABX,M_R_],[A_ABX,M_R_]]        
    ],
    # cc = 01
    [
        # ORA         AND          EOR          ADC          STA          LDA          CMP          SBC
        [[A_IDX,M_R_],[A_IDX,M_R_],[A_IDX,M_R_],[A_IDX,M_R_],[A_IDX,M__W],[A_IDX,M_R_],[A_IDX,M_R_],[A_IDX,M_R_]],
        [[A_ZER,M_R_],[A_ZER,M_R_],[A_ZER,M_R_],[A_ZER,M_R_],[A_ZER,M__W],[A_ZER,M_R_],[A_ZER,M_R_],[A_ZER,M_R_]],
        [[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_]],
        [[A_ABS,M_R_],[A_ABS,M_R_],[A_ABS,M_R_],[A_ABS,M_R_],[A_ABS,M__W],[A_ABS,M_R_],[A_ABS,M_R_],[A_ABS,M_R_]],
        [[A_IDY,M_R_],[A_IDY,M_R_],[A_IDY,M_R_],[A_IDY,M_R_],[A_IDY,M__W],[A_IDY,M_R_],[A_IDY,M_R_],[A_IDY,M_R_]],
        [[A_ZPX,M_R_],[A_ZPX,M_R_],[A_ZPX,M_R_],[A_ZPX,M_R_],[A_ZPX,M__W],[A_ZPX,M_R_],[A_ZPX,M_R_],[A_ZPX,M_R_]],
        [[A_ABY,M_R_],[A_ABY,M_R_],[A_ABY,M_R_],[A_ABY,M_R_],[A_ABY,M__W],[A_ABY,M_R_],[A_ABY,M_R_],[A_ABY,M_R_]],
        [[A_ABX,M_R_],[A_ABX,M_R_],[A_ABX,M_R_],[A_ABX,M_R_],[A_ABX,M__W],[A_ABX,M_R_],[A_ABX,M_R_],[A_ABX,M_R_]]
    ],
    # cc = 02
    [
        # ASL         ROL          LSR          ROR          STX          LDX          DEC          INC
        [[A_INV,M_RW],[A_INV,M_RW],[A_INV,M_RW],[A_INV,M_RW],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_]],
        [[A_ZER,M_RW],[A_ZER,M_RW],[A_ZER,M_RW],[A_ZER,M_RW],[A_ZER,M__W],[A_ZER,M_R_],[A_ZER,M_RW],[A_ZER,M_RW]],
        [[A____,M___],[A____,M___],[A____,M___],[A____,M___],[A____,M___],[A____,M___],[A____,M___],[A____,M___]],
        [[A_ABS,M_RW],[A_ABS,M_RW],[A_ABS,M_RW],[A_ABS,M_RW],[A_ABS,M__W],[A_ABS,M_R_],[A_ABS,M_RW],[A_ABS,M_RW]],
        [[A_INV,M_RW],[A_INV,M_RW],[A_INV,M_RW],[A_INV,M_RW],[A_INV,M__W],[A_INV,M_R_],[A_INV,M_RW],[A_INV,M_RW]],
        [[A_ZPX,M_RW],[A_ZPX,M_RW],[A_ZPX,M_RW],[A_ZPX,M_RW],[A_ZPY,M__W],[A_ZPY,M_R_],[A_ZPX,M_RW],[A_ZPX,M_RW]],
        [[A____,M_R_],[A____,M_R_],[A____,M_R_],[A____,M_R_],[A____,M___],[A____,M___],[A____,M_R_],[A____,M_R_]],
        [[A_ABX,M_RW],[A_ABX,M_RW],[A_ABX,M_RW],[A_ABX,M_RW],[A_ABY,M__W],[A_ABY,M_R_],[A_ABX,M_RW],[A_ABX,M_RW]]
    ],
    # cc = 03
    [
        [[A_IDX,M_RW],[A_IDX,M_RW],[A_IDX,M_RW],[A_IDX,M_RW],[A_IDX,M__W],[A_IDX,M_R_],[A_IDX,M_RW],[A_IDX,M_RW]],
        [[A_ZER,M_RW],[A_ZER,M_RW],[A_ZER,M_RW],[A_ZER,M_RW],[A_ZER,M__W],[A_ZER,M_R_],[A_ZER,M_RW],[A_ZER,M_RW]],
        [[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_],[A_IMM,M_R_]],
        [[A_ABS,M_RW],[A_ABS,M_RW],[A_ABS,M_RW],[A_ABS,M_RW],[A_ABS,M__W],[A_ABS,M_R_],[A_ABS,M_RW],[A_ABS,M_RW]],
        [[A_IDY,M_RW],[A_IDY,M_RW],[A_IDY,M_RW],[A_IDY,M_RW],[A_IDY,M_RW],[A_IDY,M_R_],[A_IDY,M_RW],[A_IDY,M_RW]],
        [[A_ZPX,M_RW],[A_ZPX,M_RW],[A_ZPX,M_RW],[A_ZPX,M_RW],[A_ZPY,M__W],[A_ZPY,M_R_],[A_ZPX,M_RW],[A_ZPX,M_RW]],
        [[A_ABY,M_RW],[A_ABY,M_RW],[A_ABY,M_RW],[A_ABY,M_RW],[A_ABY,M__W],[A_ABY,M_R_],[A_ABY,M_RW],[A_ABY,M_RW]],
        [[A_ABX,M_RW],[A_ABX,M_RW],[A_ABX,M_RW],[A_ABX,M_RW],[A_ABY,M__W],[A_ABY,M_R_],[A_ABX,M_RW],[A_ABX,M_RW]]
    ]
]

class opcode:
    def __init__(self, op):
        self.code = op
        self.cmt = None
        self.i = 0
        self.src = [None] * 8
    def t(self, src):
        self.src[self.i] = src
        self.i += 1
    def ta(self, src):
        self.src[self.i-1] += src

#-------------------------------------------------------------------------------
#   output a src line
#
out_lines = ''
def l(s) :
    global out_lines
    out_lines += s + '\n'
    # print(out_lines)

#-------------------------------------------------------------------------------
def write_op(op):
    if not op.cmt:
        op.cmt = '???'
    l('    /* {} */'.format(op.cmt if op.cmt else '???'))
    for t in range(0, 8):
        if t < op.i:
            l('        _ if self.ir == (0x{:02X}<<3)|{} => {{{}}},'.format(op.code, t, op.src[t]))
        

#-------------------------------------------------------------------------------
def cmt(o,cmd):
    cc = o.code & 3
    bbb = (o.code>>2) & 7
    aaa = (o.code>>5) & 7
    addr_mode = ops[cc][bbb][aaa][0]
    o.cmt = cmd;
    if addr_mode != '':
        o.cmt += ' '+addr_mode_str[addr_mode]

#-------------------------------------------------------------------------------
def u_cmt(o,cmd):
    cmt(o,cmd)
    o.cmt += ' (undoc)'

#-------------------------------------------------------------------------------
def invalid_opcode(op):
    cc = op & 3
    bbb = (op>>2) & 7
    aaa = (op>>5) & 7
    addr_mode = ops[cc][bbb][aaa][0]
    return addr_mode == A_INV

#-------------------------------------------------------------------------------
def enc_addr(op, addr_mode, mem_access):
    if addr_mode == A____:
        # no addressing, this still puts the PC on the address bus without 
        # incrementing the PC
        op.t('sa(&mut pins, self.pc);')
    elif addr_mode == A_IMM:
        # immediate mode
        op.t('sa(&mut pins, self.pc);self.pc+=1;')
    elif addr_mode == A_ZER:
        # zero page
        op.t('sa(&mut pins, self.pc);self.pc+=1;')
        op.t('let zz = ga(&pins);sa(&mut pins, zz);')
    elif addr_mode == A_ZPX:
        # zero page + X
        op.t('sa(&mut pins, self.pc);self.pc+=1;')
        op.t('self.adl_adh = ga(&pins);sa(&mut pins, self.adl_adh);')
        op.t('sa(&mut pins, (self.adl_adh + (self.x as u16)) & 0x00FF);')
    elif addr_mode == A_ZPY:
        # zero page + Y
        op.t('sa(&mut pins, self.pc);self.pc+=1;')
        op.t('self.adl_adh = ga(&pins);sa(&mut pins, self.adl_adh);')
        op.t('sa(&mut pins, (self.adl_adh+(self.y as u16))&0x00FF);')
    elif addr_mode == A_ABS:
        # absolute
        op.t('sa(&mut pins, self.pc);self.pc+=1;')
        op.t('sa(&mut pins, self.pc);self.pc+=1;self.adl_adh = gd(&pins) as u16;')
        op.t('let zz = gd(&pins);sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);')
    elif addr_mode == A_ABX:
        # absolute + X
        # this needs to check if a page boundary is crossed, which costs
        # and additional cycle, but this early-out only happens when the
        # instruction doesn't need to write back to memory
        op.t('sa(&mut pins, self.pc);self.pc+=1;')
        op.t('sa(&mut pins, self.pc);self.pc+=1;self.adl_adh = gd(&pins) as u16;')
        op.t('self.adl_adh|=((gd(&pins)as u16)<<8);sa(&mut pins, (self.adl_adh&0xFF00)|((self.adl_adh+(self.x as u16))&0xFF));')
        if mem_access == M_R_:
            # skip next tick if read access and page not crossed
            op.ta('self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.x as u16)) >> 8)))&1;')
        op.t('sa(&mut pins, self.adl_adh+(self.x as u16));')
    elif addr_mode == A_ABY:
        # absolute + Y
        # same page-boundary-crossed special case as absolute+X
        op.t('sa(&mut pins, self.pc);self.pc+=1;')
        op.t('sa(&mut pins, self.pc);self.pc+=1;self.adl_adh = gd(&pins) as u16;')
        op.t('self.adl_adh|=((gd(&pins)as u16)<<8);sa(&mut pins, (self.adl_adh&0xFF00)|((self.adl_adh+(self.y as u16))&0xFF));')
        if mem_access == M_R_:
            # skip next tick if read access and page not crossed
            op.ta('self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8)))&1;')
        op.t('sa(&mut pins, self.adl_adh+(self.y as u16));')
    elif addr_mode == A_IDX:
        # (zp,X)
        op.t('sa(&mut pins, self.pc);self.pc+=1;')
        op.t('self.adl_adh = ga(&pins);sa(&mut pins, self.adl_adh);')
        op.t('self.adl_adh = (self.adl_adh+(self.x as u16))&0xFF;sa(&mut pins, self.adl_adh);')
        op.t('sa(&mut pins, self.adl_adh+1 & 0xFF); self.adl_adh = ga(&pins);')
        op.t('let zz = gd(&pins);sa(&mut pins, ((zz as u16) << 8) | self.adl_adh);')
    elif addr_mode == A_IDY:
        # (zp),Y
        # same page-boundary-crossed special case as absolute+X
        op.t('sa(&mut pins, self.pc);self.pc+=1;')
        op.t('self.adl_adh = ga(&pins);sa(&mut pins, self.adl_adh);')
        op.t('sa(&mut pins, self.adl_adh+1 & 0xFF); self.adl_adh = ga(&pins);')
        op.t('self.adl_adh|=((gd(&pins)as u16)<<8);sa(&mut pins, (self.adl_adh&0xFF00)|((self.adl_adh+(self.y as u16))&0xFF));')
        if mem_access == M_R_:
            # skip next tick if read access and page not crossed
            op.ta('self.ir += (!((self.adl_adh >> 8) - ((self.adl_adh + (self.y as u16)) >> 8)))&1;')
        op.t('sa(&mut pins, self.adl_adh+(self.y as u16));')
    elif addr_mode == A_JMP:
        # jmp is completely handled in instruction decoding
        pass
    elif addr_mode == A_JSR:
        # jsr is completely handled in instruction decoding 
        pass
    else:
        # invalid instruction
        pass

#-------------------------------------------------------------------------------
def i_brk(o):
    cmt(o, 'BRK')
    o.t('if !self.brk_flags.contains(BreakFlags::NMI) && !self.brk_flags.contains(BreakFlags::IRQ) { self.pc += 1;                }                sad(&mut pins, (0x0100 | self.sp as u16), (self.pc >> 8) as u8);                self.sp -= 1;                if !self.brk_flags.contains(BreakFlags::RESET) {                    wr(&mut pins)                }           ')
    o.t('sad(&mut pins, (0x0100 | self.sp as u16), (self.pc) as u8);self.sp -= 1;if !self.brk_flags.contains(BreakFlags::RESET) {wr(&mut pins)}')
    o.t('sad(&mut pins, (0x0100 | self.sp as u16), (self.sr.bits | StatusRegister::X.bits));self.sp -= 1;if self.brk_flags.contains(BreakFlags::RESET) {self.adl_adh = 0xFFC;} else {wr(&mut pins);if self.brk_flags.contains(BreakFlags::NMI) {self.adl_adh = 0xFFFA} else {self.adl_adh = 0xFFFE}}')
    o.t('sa(&mut pins, self.adl_adh);self.adl_adh += 1;self.sr.set(StatusRegister::I | StatusRegister::B, true);self.brk_flags = BreakFlags::empty();')
    o.t('sa(&mut pins, self.adl_adh);self.adl_adh = gd(&pins) as u16; /* NMI "half-hijacking" not possible */')
    o.t('self.pc = ((gd(&pins) as u16) << 8) | self.adl_adh;')

#-------------------------------------------------------------------------------
def i_nop(o):
    cmt(o,'NOP')
    o.t('')

#-------------------------------------------------------------------------------
def u_nop(o):
    u_cmt(o,'NOP')
    o.t('')

#-------------------------------------------------------------------------------
def i_lda(o):
    cmt(o,'LDA')
    o.t('self.ac=gd(&pins);self.nz(self.ac);')

#-------------------------------------------------------------------------------
def i_ldx(o):
    cmt(o,'LDX')
    o.t('self.x=gd(&pins);self.nz(self.x);')

#-------------------------------------------------------------------------------
def i_ldy(o):
    cmt(o,'LDY')
    o.t('self.y=gd(&pins);self.nz(self.y);')

#-------------------------------------------------------------------------------
def u_lax(o):
    u_cmt(o,'LAX')
    o.t('self.ac = gd(&pins); self.x = gd(&pins);self.nz(self.x);')

#-------------------------------------------------------------------------------
def x_lxa(o):
    # undocumented LXA
    # and immediate byte with A, then load X with A
    u_cmt(o,'LXA')
    o.t('let zz = (self.ac|0xEE)&gd(&pins);self.ac=zz;self.x=zz;self.nz(self.ac);')

#-------------------------------------------------------------------------------
def i_sta(o):
    cmt(o,'STA')
    o.ta('sd(&mut pins, self.ac); wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_stx(o):
    cmt(o,'STX')
    o.ta('sd(&mut pins, self.x); wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_sty(o):
    cmt(o,'STY')
    o.ta('sd(&mut pins, self.y); wr(&mut pins);')

#-------------------------------------------------------------------------------
def u_sax(o):
    u_cmt(o,'SAX')
    o.ta('sd(&mut pins, self.ac&self.x); wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_tax(o):
    cmt(o,'TAX')
    o.t('self.x=self.ac;self.nz(self.x);')

#-------------------------------------------------------------------------------
def i_tay(o):
    cmt(o,'TAY')
    o.t('self.y=self.ac;self.nz(self.y);')

#-------------------------------------------------------------------------------
def i_txa(o):
    cmt(o,'TXA')
    o.t('self.ac=self.x;self.nz(self.ac);')

#-------------------------------------------------------------------------------
def i_tya(o):
    cmt(o,'TYA')
    o.t('self.ac=self.x;self.nz(self.ac);')

#-------------------------------------------------------------------------------
def i_txs(o):
    cmt(o,'TXS')
    o.t('self.sp=self.x;')

#-------------------------------------------------------------------------------
def i_tsx(o):
    cmt(o,'TSX')
    o.t('self.x=self.sp;self.nz(self.x);')

#-------------------------------------------------------------------------------
def i_php(o):
    cmt(o,'PHP')
    o.t('sad(&mut pins, 0x0100|(self.sp as u16), self.sr.bits|StatusRegister::X.bits);self.sp-=1;wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_plp(o):
    cmt(o,'PLP')
    o.t('sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;')   # read junk byte from current SP
    o.t('sa(&mut pins, 0x0100|(self.sp as u16));')     # read actual byte  
    o.t('self.sr = StatusRegister::from_bits_truncate((gd(&pins)|StatusRegister::B.bits)&!StatusRegister::X.bits);');

#-------------------------------------------------------------------------------
def i_pha(o):
    cmt(o,'PHA')
    o.t('sad(&mut pins, 0x0100|(self.sp as u16), self.ac);self.sp-=1;wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_pla(o):
    cmt(o,'PLA')
    o.t('sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;')   # read junk byte from current SP
    o.t('sa(&mut pins, 0x0100|(self.sp as u16));')     # read actual byte  
    o.t('self.ac=gd(&pins);self.nz(self.ac);')

#-------------------------------------------------------------------------------
def i_se(o, f):
    cmt(o,'SE'+flag_name(f))
    o.t('self.sr.set(StatusRegister::'+flag_name(f)+', true);')

#-------------------------------------------------------------------------------
def i_cl(o, f):
    cmt(o,'CL'+flag_name(f))
    o.t('self.sr.set(StatusRegister::'+flag_name(f)+', false);')

#-------------------------------------------------------------------------------
def i_br(o, m, v):
    cmt(o,branch_name(m,v))
    # if branch not taken?
    o.t('sa(&mut pins, self.pc);self.adl_adh=self.pc+(gd(&pins) as u16); if self.sr.bitand(StatusRegister::'+flag_name(m)+').bits !='+hex(v)+' { fetch(&mut pins, self.pc) }')
    # branch taken: shortcut if page not crossed, 'branchquirk' interrupt fix
    o.t('sa(&mut pins, (self.pc & 0xFF00)|(self.adl_adh&0x00FF));if self.adl_adh&0xFF00 == self.pc &0xFF00 { self.pc=self.adl_adh;self.irq_pip>>=1;self.nmi_pip>>=1;;fetch(&mut pins, self.pc) }')
    # page crossed extra cycle:
    o.t('self.pc=self.adl_adh;')

#-------------------------------------------------------------------------------
def i_jmp(o):
    cmt(o,'JMP')
    o.t('sa(&mut pins, self.pc);self.pc+=1;')
    o.t('sa(&mut pins, self.pc);self.pc+=1;self.adl_adh = gd(&pins) as u16;')
    o.t('self.pc = ((gd(&pins) as u16)<<8)|self.adl_adh;')

#-------------------------------------------------------------------------------
def i_jmpi(o):
    cmt(o,'JMPI')
    o.t('sa(&mut pins, self.pc);self.pc+=1;')
    o.t('sa(&mut pins, self.pc);self.pc+=1;self.adl_adh = gd(&pins) as u16;')
    o.t('self.adl_adh|=(gd(&pins) as u16)<<8;sa(&mut pins, self.adl_adh);')
    o.t('sa(&mut pins, (self.adl_adh&0xFF00)|((self.adl_adh+1)&0x00FF));;self.adl_adh = gd(&pins) as u16;')
    o.t('self.pc = ((gd(&pins) as u16)<<8)|self.adl_adh;')

#-------------------------------------------------------------------------------
def i_jsr(o):
    cmt(o,'JSR')
    # read low byte of target address
    o.t('sa(&mut pins, self.pc);self.pc+=1;')
    # put SP on addr bus, next cycle is a junk read
    o.t('sa(&mut pins, 0x0100|(self.sp as u16));self.adl_adh = gd(&pins) as u16;')
    # write PC high byte to stack
    o.t('sad(&mut pins, 0x0100|(self.sp as u16), (self.pc>>8) as u8);self.sp-=1;wr(&mut pins);')
    # write PC low byte to stack
    o.t('sad(&mut pins, 0x0100|(self.sp as u16), (self.pc as u8));self.sp-=1;wr(&mut pins);')
    # load target address high byte
    o.t('sa(&mut pins, self.pc);')
    # load PC and done
    o.t('self.pc = ((gd(&pins) as u16)<<8)|self.adl_adh;')

#-------------------------------------------------------------------------------
def i_rts(o):
    cmt(o,'RTS')
    # put SP on stack and do a junk read
    o.t('sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;')
    # load return address low byte from stack
    o.t('sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;')
    # load return address high byte from stack
    o.t('sa(&mut pins, 0x0100|(self.sp as u16));self.adl_adh = gd(&pins) as u16;')
    # put return address in PC, this is one byte before next op, do junk read from PC
    o.t('self.pc = ((gd(&pins) as u16)<<8)|self.adl_adh;sa(&mut pins, self.pc);self.pc+=1;')
    # next tick is opcode fetch
    o.t('');

#-------------------------------------------------------------------------------
def i_rti(o):
    cmt(o,'RTI')
    # put SP on stack and do a junk read
    o.t('sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;')
    # load processor status flag from stack
    o.t('sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;')
    # load return address low byte from stack
    o.t('sa(&mut pins, 0x0100|(self.sp as u16));self.sp+=1;self.sr = StatusRegister::from_bits_truncate((gd(&pins)|StatusRegister::B.bits)&!StatusRegister::X.bits);')
    # load return address high byte from stack
    o.t('sa(&mut pins, 0x0100|(self.sp as u16));self.adl_adh = gd(&pins) as u16;')
    # update PC (which is already placed on the right return-to instruction)
    o.t('self.pc = ((gd(&pins) as u16)<<8)|self.adl_adh;')

#-------------------------------------------------------------------------------
def i_ora(o):
    cmt(o,'ORA')
    o.t('self.ac|=gd(&pins);self.nz(self.ac);')

#-------------------------------------------------------------------------------
def i_and(o):
    cmt(o,'AND')
    o.t('self.ac&=gd(&pins);self.nz(self.ac);')

#-------------------------------------------------------------------------------
def i_eor(o):
    cmt(o,'EOR')
    o.t('self.ac^=gd(&pins);self.nz(self.ac);')

#-------------------------------------------------------------------------------
def i_adc(o):
    cmt(o,'ADC')
    o.t('self.adc(gd(&pins));')

#-------------------------------------------------------------------------------
def i_sbc(o):
    cmt(o,'SBC')
    o.t('self.sbc(gd(&pins));')

#-------------------------------------------------------------------------------
def u_sbc(o):
    u_cmt(o,'SBC')
    o.t('self.sbc(gd(&pins));')

#-------------------------------------------------------------------------------
def i_cmp(o):
    cmt(o,'CMP')
    o.t('self.cmp(self.ac, gd(&pins));')

#-------------------------------------------------------------------------------
def i_cpx(o):
    cmt(o,'CPX')
    o.t('self.cmp(self.x, gd(&pins));')

#-------------------------------------------------------------------------------
def i_cpy(o):
    cmt(o,'CPY')
    o.t('self.cmp(self.y, gd(&pins));')

#-------------------------------------------------------------------------------
def u_dcp(o):
    # undocumented 'decrement and compare'
    u_cmt(o,'DCP')
    o.t('self.adl_adh = gd(&pins) as u16;wr(&mut pins);')
    o.t('self.adl_adh-=1;self.nz(self.adl_adh as u8);sd(&mut pins, self.adl_adh as u8);self.cmp(self.ac, self.adl_adh as u8);wr(&mut pins);')

#-------------------------------------------------------------------------------
def x_sbx(o):
    u_cmt(o,'SBX')
    o.t('self.sbx(gd(&pins));')

#-------------------------------------------------------------------------------
def i_dec(o):
    cmt(o,'DEC')
    o.t('self.adl_adh = gd(&pins) as u16;wr(&mut pins);')
    o.t('self.adl_adh-=1; self.nz(self.adl_adh as u8);sd(&mut pins, self.adl_adh as u8);wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_inc(o):
    cmt(o,'INC')
    o.t('self.adl_adh = gd(&pins) as u16;wr(&mut pins);')
    o.t('self.adl_adh+=1; self.nz(self.adl_adh as u8);sd(&mut pins, self.adl_adh as u8);wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_dex(o):
    cmt(o,'DEX')
    o.t('self.x-=1; self.nz(self.x);')

#-------------------------------------------------------------------------------
def i_dey(o):
    cmt(o,'DEY')
    o.t('self.y-=1; self.nz(self.y);')

#-------------------------------------------------------------------------------
def i_inx(o):
    cmt(o,'INX')
    o.t('self.x+=1; self.nz(self.x);')

#-------------------------------------------------------------------------------
def i_iny(o):
    cmt(o,'INY')
    o.t('self.y+=1; self.nz(self.y);')

#-------------------------------------------------------------------------------
def u_isb(o):
    # undocumented INC+SBC instruction
    u_cmt(o,'ISB')
    o.t('self.adl_adh = gd(&pins) as u16;wr(&mut pins);')
    o.t('self.adl_adh+=1;sd(&mut pins, self.adl_adh as u8); self.sbc(self.adl_adh as u8);wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_asl(o):
    cmt(o,'ASL')
    o.t('self.adl_adh = gd(&pins) as u16;wr(&mut pins);')
    o.t('sd(&mut pins, self.asl(self.adl_adh as u8));wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_asla(o):
    cmt(o,'ASLA')
    o.t('self.ac = self.asl(self.ac);')

#-------------------------------------------------------------------------------
def i_lsr(o):
    cmt(o,'LSR')
    o.t('self.adl_adh = gd(&pins) as u16;wr(&mut pins);')
    o.t('sd(&mut pins, self.lsr(self.adl_adh as u8));wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_lsra(o):
    cmt(o,'LSRA')
    o.t('self.ac = self.asl(self.ac);')

#-------------------------------------------------------------------------------
def u_slo(o):
    # undocumented ASL+OR
    u_cmt(o,'SLO')
    o.t('self.adl_adh = gd(&pins) as u16;wr(&mut pins);')
    o.t('self.adl_adh = self.asl(self.adl_adh as u8) as u16;sd(&mut pins, self.adl_adh as u8); self.ac|=(self.adl_adh as u8);self.nz(self.ac);wr(&mut pins);')

#-------------------------------------------------------------------------------
def x_asr(o):
    # undocumented AND+LSR
    u_cmt(o, 'ASR')
    o.t('self.ac=gd(&pins);self.ac = self.asl(self.ac);')

#-------------------------------------------------------------------------------
def u_sre(o):
    # undocumented LSR+EOR
    u_cmt(o,'SRE')
    o.t('self.adl_adh = gd(&pins) as u16;wr(&mut pins);')
    o.t('self.adl_adh = self.lsr(self.adl_adh as u8) as u16;sd(&mut pins, self.adl_adh as u8); self.ac^=(self.adl_adh as u8);self.nz(self.ac);wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_rol(o):
    cmt(o,'ROL')
    o.t('self.adl_adh = gd(&pins) as u16;wr(&mut pins);')
    o.t('sd(&mut pins, self.rol(self.adl_adh as u8));wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_rola(o):
    cmt(o,'ROLA')
    o.t('self.ac=self.rol(self.ac);')

#-------------------------------------------------------------------------------
def u_rla(o):
    # uncodumented ROL+AND
    u_cmt(o,'RLA')
    o.t('self.adl_adh = gd(&pins) as u16;wr(&mut pins);')
    o.t('self.adl_adh = self.rol(self.adl_adh as u8) as u16;sd(&mut pins, self.adl_adh as u8);self.ac &=(self.adl_adh as u8);self.nz(self.ac);wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_ror(o):
    cmt(o,'ROR')
    o.t('self.adl_adh = gd(&pins) as u16;wr(&mut pins);')
    o.t('sd(&mut pins, self.ror(self.adl_adh as u8));wr(&mut pins);')

#-------------------------------------------------------------------------------
def i_rora(o):
    cmt(o,'RORA')
    o.t('self.ac=self.ror(self.ac);')

#-------------------------------------------------------------------------------
def u_rra(o):
    # undocumented ROR+ADC
    u_cmt(o,'RRA')
    o.t('self.adl_adh = gd(&pins) as u16;wr(&mut pins);')
    o.t('self.adl_adh = self.ror(self.adl_adh as u8) as u16;sd(&mut pins, self.adl_adh as u8);self.adc(self.adl_adh as u8);wr(&mut pins);')

#-------------------------------------------------------------------------------
def x_arr(o):
    # undocumented AND+ROR
    u_cmt(o,'ARR')
    o.t('self.ac=gd(&pins);self.arr();')

#-------------------------------------------------------------------------------
def x_ane(o):
    # undocumented ANE
    u_cmt(o,'ANE')
    o.t('self.ac = (self.ac|0xEE)&self.x&gd(&pins);self.nz(self.ac);')

#-------------------------------------------------------------------------------
def x_sha(o):
    # undocumented SHA
    #  stores the result of A AND X AND the high byte of the target address of
    #  the operand +1 in memory
    #
    u_cmt(o,'SHA')
    o.ta('let zz = (((ga(&mut pins) >> 8) + 1) as u8);sd(&mut pins, self.ac & self.x & zz);wr(&mut pins);')

#-------------------------------------------------------------------------------
def x_shx(o):
    # undocumented SHX
    # AND X register with the high byte of the target address of the 
    # argument + 1. Store the result in memory.
    #
    u_cmt(o, 'SHX')
    o.ta('let zz = (((ga(&mut pins) >> 8) + 1) as u8);sd(&mut pins, self.x & zz);wr(&mut pins);')

#-------------------------------------------------------------------------------
def x_shy(o):
    # undocumented SHX
    # AND Y register with the high byte of the target address of the
    # argument + 1. Store the result in memory.
    #
    u_cmt(o, 'SHY')
    o.ta('let zz = (((ga(&mut pins) >> 8) + 1) as u8);sd(&mut pins, self.y & zz);wr(&mut pins);')

#-------------------------------------------------------------------------------
def x_shs(o):
    # undocumented SHS
    # AND X register with accumulator and store result in stack pointer, then
    # AND stack pointer with the high byte of the target address of the
    # argument + 1. Store result in memory.
    #
    u_cmt(o, 'SHS')
    o.ta('self.sp=self.ac & self.x;let zz = (((ga(&mut pins) >> 8) + 1) as u8);sd(&mut pins, self.sp&zz);wr(&mut pins);')

#-------------------------------------------------------------------------------
def x_anc(o):
    # undocumented ANC
    # AND byte with accumulator. If result is negative then carry is set.
    #
    u_cmt(o, 'ANC')
    o.t('self.ac = gd(&pins);self.nz(self.ac);self.sr.set(StatusRegister::C, (self.ac&0x80)!=0);')

#-------------------------------------------------------------------------------
def x_las(o):
    # undocumented LAS
    # AND memory with stack pointer, transfer result to accumulator, X
    # register and stack pointer.
    #
    u_cmt(o, 'LAS')
    o.t('let zz = gd(&pins)&self.sp;self.ac=zz;self.x=zz;self.sp=zz;self.nz(self.ac);')

#-------------------------------------------------------------------------------
def x_jam(o):
    # undocumented JAM, next opcode byte read, data and addr bus set to all 1, execution stops
    u_cmt(o, 'JAM')
    o.t('sa(&mut pins, self.pc);')
    o.t('sad(&mut pins, 0xFFFF,0xFF);self.ir-=1;')

#-------------------------------------------------------------------------------
def i_bit(o):
    cmt(o,'BIT')
    o.t('self.bit(gd(&pins));')

#-------------------------------------------------------------------------------
def enc_op(op):
    o = opcode(op)
    if invalid_opcode(op):
        x_jam(o);
        return o

    # decode the opcode byte
    cc = op & 3
    bbb = (op>>2) & 7
    aaa = (op>>5) & 7
    addr_mode = ops[cc][bbb][aaa][0]
    mem_access = ops[cc][bbb][aaa][1]
    # addressing mode
    enc_addr(o, addr_mode, mem_access);
    # actual instruction
    if cc == 0:
        if aaa == 0:
            if bbb == 0:        i_brk(o)
            elif bbb == 2:      i_php(o)
            elif bbb == 4:      i_br(o, NF, 0)  # BPL
            elif bbb == 6:      i_cl(o, CF)
            else:               u_nop(o)
        elif aaa == 1:
            if bbb == 0:        i_jsr(o)
            elif bbb == 2:      i_plp(o)
            elif bbb == 4:      i_br(o, NF, NF) # BMI
            elif bbb == 6:      i_se(o, CF)
            elif bbb in [5, 7]: u_nop(o)
            else:               i_bit(o)
        elif aaa == 2:
            if bbb == 0:        i_rti(o)
            elif bbb == 2:      i_pha(o)
            elif bbb == 3:      i_jmp(o)
            elif bbb == 4:      i_br(o, VF, 0)  # BVC
            elif bbb == 6:      i_cl(o, IF)
            else:               u_nop(o)
        elif aaa == 3:
            if bbb == 0:        i_rts(o)
            elif bbb == 2:      i_pla(o)
            elif bbb == 3:      i_jmpi(o)
            elif bbb == 4:      i_br(o, VF, VF) # BVS
            elif bbb == 6:      i_se(o, IF)
            else:               u_nop(o)
        elif aaa == 4:
            if bbb == 0:        u_nop(o)
            elif bbb == 2:      i_dey(o)
            elif bbb == 4:      i_br(o, CF, 0)  # BCC
            elif bbb == 6:      i_tya(o)
            elif bbb == 7:      x_shy(o)
            else:               i_sty(o)
        elif aaa == 5:
            if bbb == 2:        i_tay(o)
            elif bbb == 4:      i_br(o, CF, CF) # BCS
            elif bbb == 6:      i_cl(o, VF)
            else:               i_ldy(o)
        elif aaa == 6:
            if bbb == 2:        i_iny(o)
            elif bbb == 4:      i_br(o, ZF, 0)  # BNE
            elif bbb == 6:      i_cl(o, DF)
            elif bbb in [5, 7]: u_nop(o)
            else:               i_cpy(o)
        elif aaa == 7:
            if bbb == 2:        i_inx(o)
            elif bbb == 4:      i_br(o, ZF, ZF) # BEQ
            elif bbb == 6:      i_se(o, DF)
            elif bbb in [5, 7]: u_nop(o)
            else:               i_cpx(o)
    elif cc == 1:
        if aaa == 0:    i_ora(o)
        elif aaa == 1:  i_and(o)
        elif aaa == 2:  i_eor(o)
        elif aaa == 3:  i_adc(o)
        elif aaa == 4:
            if bbb == 2:    u_nop(o)
            else:           i_sta(o)
        elif aaa == 5:  i_lda(o)
        elif aaa == 6:  i_cmp(o)
        else:           i_sbc(o)
    elif cc == 2:
        if aaa == 0:
            if bbb == 2:    i_asla(o)
            elif bbb == 6:  u_nop(o)
            else:           i_asl(o)
        elif aaa == 1:
            if bbb == 2:    i_rola(o)
            elif bbb == 6:  u_nop(o)
            else:           i_rol(o)
        elif aaa == 2:
            if bbb == 2:    i_lsra(o)
            elif bbb == 6:  u_nop(o)
            else:           i_lsr(o)
        elif aaa == 3:
            if bbb == 2:    i_rora(o)
            elif bbb == 6:  u_nop(o)
            else:           i_ror(o)
        elif aaa == 4:
            if bbb == 0:    u_nop(o)
            elif bbb == 2:  i_txa(o)
            elif bbb == 6:  i_txs(o)
            elif bbb == 7:  x_shx(o)
            else:           i_stx(o)
        elif aaa == 5:
            if bbb == 2:    i_tax(o)
            elif bbb == 6:  i_tsx(o)
            else:           i_ldx(o)
        elif aaa == 6:
            if bbb == 2:        i_dex(o)
            elif bbb in [0, 6]: u_nop(o)
            else:               i_dec(o)
        elif aaa == 7:
            if bbb == 2:        i_nop(o)
            elif bbb in [0, 6]: u_nop(o)
            else:               i_inc(o)
    elif cc == 3:
        # undocumented block
        if aaa == 0:    
            if bbb == 2:    x_anc(o)
            else:           u_slo(o)
        elif aaa == 1:  
            if bbb == 2:    x_anc(o)
            else:           u_rla(o)
        elif aaa == 2:
            if bbb == 2:    x_asr(o)
            else:           u_sre(o)
        elif aaa == 3:  
            if bbb == 2:    x_arr(o)
            else:           u_rra(o)
        elif aaa == 4:
            if bbb == 2:        x_ane(o)
            elif bbb == 6:      x_shs(o)
            elif bbb in [4,7]:  x_sha(o)
            else:               u_sax(o)
        elif aaa == 5:
            if bbb == 2:    x_lxa(o)
            elif bbb == 6:  x_las(o)
            else:           u_lax(o)
        elif aaa == 6:
            if bbb == 2:    x_sbx(o)
            else:           u_dcp(o)
        elif aaa == 7:
            if bbb == 2:    u_sbc(o)
            else:           u_isb(o)
    # fetch next opcode byte
    if mem_access in [M_R_,M___]:
        o.ta('fetch(&mut pins, self.pc);')
    else:
        o.t('fetch(&mut pins, self.pc);')
    return o

#-------------------------------------------------------------------------------
#   execution starts here
#
for op in range(0, 256):
    write_op(enc_op(op))

with open(InpPath, 'r') as inf:
    templ = Template(inf.read())
    c_src = templ.safe_substitute(decode_block=out_lines)
    with open(OutPath, 'w') as outf:
        outf.write(c_src)

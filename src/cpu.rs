pub mod opcodes;
use crate::bus::Bus;

#[derive(Clone)]
pub struct CPU {
    // dev flags
    debug: bool,
    brk_stop: bool,

    pub nmi_flag: bool,

    //cycle counter
    cycles: u8,
    pub tot_cycles: u32,

    // registers
    pub reg_a: u8,
    pub reg_x: u8,
    pub reg_y: u8,
    pub reg_stat: u8,
    pub reg_pc: u16,
    pub reg_sp: u8,

    // memory bus
    pub mem_bus: Bus,
}

#[derive(Debug, PartialEq)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect,
    Indirect_X,
    Indirect_Y,
    NoneAddressing,
}

pub trait Mem {
    fn mem_read(&mut self, addr: u16) -> u8;

    fn mem_write(&mut self, addr: u16, data: u8);

    fn mem_read16(&mut self, pos: u16) -> u16;

    fn mem_write16(&mut self, pos: u16, data: u16);
}

impl Mem for CPU {
    // read from memory address
    fn mem_read(&mut self, addr: u16) -> u8 {
        self.mem_bus.mem_read(addr)
    }
    // reads as little endian, returns big endian
    fn mem_read16(&mut self, addr: u16) -> u16 {
        self.mem_bus.mem_read16(addr)
    }

    // write to memory address
    fn mem_write(&mut self, addr: u16, val: u8) {
        self.mem_bus.mem_write(addr, val);
    }
    // reads value as big endian, writes as little endian
    fn mem_write16(&mut self, addr: u16, val: u16) {
        self.mem_bus.mem_write16(addr, val);
    }
}

impl CPU {
    pub fn new(mem_bus: Bus) -> Self {
        CPU {
            debug: false,
            brk_stop: false,
            nmi_flag: false,
            cycles: 0,
            tot_cycles: 0,
            reg_a: 0,       //accumulator
            reg_x: 0,       //X register
            reg_y: 0,       //Y register
            reg_stat: 0x30,    //status register
            reg_pc: 0,      //program counter
            reg_sp: 0xFF,
            mem_bus,
        }
    }

    pub fn get_op_addr(&mut self, mode: &AddressingMode) -> u16 {
        let arg = self.mem_read(self.reg_pc);
        let arg2 = self.mem_read(self.reg_pc.wrapping_add(1));
        let arg3 = self.mem_read16(self.reg_pc);
        match mode {
            AddressingMode::Immediate => self.reg_pc,
            AddressingMode::ZeroPage => self.mem_read(self.reg_pc) as u16,
            AddressingMode::Absolute => self.mem_read16(self.reg_pc),
            AddressingMode::ZeroPage_X => self.mem_read(self.reg_pc).wrapping_add(self.reg_x) as u16,
            AddressingMode::ZeroPage_Y => self.mem_read(self.reg_pc).wrapping_add(self.reg_y) as u16,
            AddressingMode::Absolute_X => self.mem_read16(self.reg_pc).wrapping_add(self.reg_x as u16),
            AddressingMode::Absolute_Y => self.mem_read16(self.reg_pc).wrapping_add(self.reg_y as u16),
            AddressingMode::Indirect => self.mem_read(arg3) as u16 + ((self.mem_read(arg.wrapping_add(1) as u16 + (((arg2 as u16) << 8))) as u16) << 8),
            AddressingMode::Indirect_X => self.mem_read(arg.wrapping_add(self.reg_x) as u16) as u16 + ((self.mem_read(arg.wrapping_add(self.reg_x).wrapping_add(1) as u16) as u16) << 8),
            AddressingMode::Indirect_Y => (self.mem_read(arg as u16) as u16 + ((self.mem_read(arg.wrapping_add(1) as u16) as u16) << 8)).wrapping_add(self.reg_y as u16),
            AddressingMode::NoneAddressing => {
                panic!("mode {:?} is not supported", mode);
            }
        }
    }

    //endian-ness conversion
    fn change_endian(&self, val: u16) -> u16 {
        let hi = val & 0x00FF;
        let lo = val >> 8;
        (hi << 8) | lo
    }

    // push byte onto stack
    fn stack_push(&mut self, val: u8) {
        self.mem_write(0x0100 + self.reg_sp as u16, val);
        if self.reg_sp == 0x00 {
            self.reg_sp = 0xFF;
        } else {
            self.reg_sp -= 1;
        }
    }
    // push two bytes onto stack
    fn stack_push16(&mut self, val: u16) {
        let lo = (val >> 8) as u8;
        let hi = (val & 0x00FF) as u8;
        self.stack_push(lo);
        self.stack_push(hi);
    }

    // pull byte from stack
    fn stack_pull(&mut self) -> u8 {
        if self.reg_sp == 0xFF {
            self.reg_sp = 0x00;
        } else {
            self.reg_sp += 1;
        }
        
        self.mem_read(0x0100 + self.reg_sp as u16)
    }
    fn stack_pull16(&mut self) -> u16 {
        let hi = self.stack_pull() as u16;
        let lo = self.stack_pull() as u16;

        (hi << 8) | lo
    }

    //Reset CPU Registers
    pub fn reset(&mut self) {
        self.reg_a = 0;
        self.reg_x = 0;
        self.reg_y = 0;
        self.reg_stat = 0x24;
        self.reg_sp = 0xFD;

        //reset program counter to specified address pointed to
        //by address 0xFFFC
        self.reg_pc = self.mem_read16(0xFFFC);
        self.tot_cycles = 7;
    }

    //Instructions

    //Load Accumulator
    fn lda(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("LDA: ");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let param = self.mem_read(addr);
        if self.debug {
            println!("\tData = {:x?}", param);
        }
        self.reg_a = param; //load accumulator with instr param

        self.update_nz(self.reg_a);
        
        if self.debug {
            println!("\tAccumulator = {:x?}", self.reg_a);
        }
        if self.debug {
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Load X Register
    fn ldx(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("LDX: ");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let param = self.mem_read(addr);
        if self.debug {
            println!("\tData = {:x?}", param);
        }
        self.reg_x = param; //load x register with instr param

        self.update_nz(self.reg_x);
        if self.debug {
            println!("\tX Register = {:x?}", self.reg_x);
        }
        if self.debug {
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Load Y Register
    fn ldy(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("LDY: ");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let param = self.mem_read(addr);
        if self.debug {
            println!("\tData = {:x?}", param);
        }
        self.reg_y = param; //load x register with instr param

        self.update_nz(self.reg_y);
        if self.debug {
            println!("\tY Register = {:x?}", self.reg_y);
        }
        if self.debug {
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }

    //Transfer Acc to X Register
    fn tax(&mut self) {
        if self.debug {
            println!("TAX:");
            println!("\tAccumulator = {:x?}", self.reg_a);
        }
        self.reg_x = self.reg_a;

        self.update_nz(self.reg_x);
        if self.debug {
            println!("\tX Register = {:x?}", self.reg_x);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Transfer Acc to Y Register
    fn tay(&mut self) {
        if self.debug {
            println!("TAY:");
            println!("\tAccumulator = {:x?}", self.reg_a);
        }
        self.reg_y = self.reg_a;

        self.update_nz(self.reg_y);
        if self.debug {
            println!("\tY Register = {:x?}", self.reg_y);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Transfer X Register to Acc
    fn txa(&mut self) {
        if self.debug {
            println!("TXA:");
            println!("\tX Register = {:x?}", self.reg_x);
        }
        self.reg_a = self.reg_x;

        self.update_nz(self.reg_a);
        if self.debug {
            println!("\tAccumulator = {:x?}", self.reg_a);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Transfer Y Register to Acc
    fn tya(&mut self) {
        if self.debug {
            println!("TXA:");
            println!("\tY Register = {:x?}", self.reg_y);
        }
        self.reg_a = self.reg_y;

        self.update_nz(self.reg_a);
        if self.debug {
            println!("\tAccumulator = {:x?}", self.reg_a);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }

    //Add with Carry
    fn adc(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("ADC: ");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let val = self.mem_read(addr);
        if self.debug {
            println!("\tData = {:x?}", val);
            println!("\tAccumulator(old) = {:x?}", self.reg_a);
            println!("\tStatus(old) = {:x?}", self.reg_stat);
        }
        let cin = self.reg_stat & 0x01;
        let res = self.reg_a as u16 + val as u16 + cin as u16;
        let cout = ((res & 0x0100) >> 8) as u8;
        self.reg_a = (res & 0xFF) as u8;
        if cout != 0 {
            self.reg_stat |= 0x01;
        } else {
            self.reg_stat &= 0xFE;
        }
        if cin + cout == 1 {
            self.reg_stat |= 0x40;
        } else {
            self.reg_stat &= 0xBF;
        }

        self.update_nz(self.reg_a);
        if self.debug {
            println!("\tAccumulator(new) = {:x?}", self.reg_a);
            println!("\tStatus(new) = {:x?}", self.reg_stat);
        }
    }
    //Subtract with Carry
    fn sbc(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("SBC: ");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let val = self.mem_read(addr);
        if self.debug {
            println!("\tData = {:x?}", val);
            println!("\tAccumulator(old) = {:x?}", self.reg_a);
        }
        let cin = self.reg_stat & 0x01;
        let res = self.reg_a as u16 + !val as u16 + cin as u16;
        let cout = ((res & 0x0100) >> 8) as u8;
        self.reg_a = (res & 0xFF) as u8;
        if cout != 0 {
            self.reg_stat |= 0x01;
        } else {
            self.reg_stat &= 0xFE;
        }
        if cin + cout == 1 {
            self.reg_stat |= 0x40;
        } else {
            self.reg_stat &= 0xBF;
        }

        self.update_nz(self.reg_a);
        if self.debug {
            println!("\tAccumulator(new) = {:x?}", self.reg_a);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }

    //Logical AND
    fn and(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("AND: ");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let val = self.mem_read(addr);
        if self.debug {
            println!("\tData = {:x?}", val);
            println!("\tAccumulator(old) = {:x?}", self.reg_a);
        }

        self.reg_a &= val;

        self.update_nz(self.reg_a);
        if self.debug {
            println!("\tAccumulator(new) = {:x?}", self.reg_a);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Logical OR
    fn ora(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("ORA: ");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let val = self.mem_read(addr);
        if self.debug {
            println!("\tData = {:x?}", val);
            println!("\tAccumulator(old) = {:x?}", self.reg_a);
        }

        self.reg_a |= val;

        self.update_nz(self.reg_a);
        if self.debug {
            println!("\tAccumulator(new) = {:x?}", self.reg_a);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Exclusive OR
    fn eor(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("EOR: ");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let val = self.mem_read(addr);
        if self.debug {
            println!("\tData = {:x?}", val);
            println!("\tAccumulator(old) = {:x?}", self.reg_a);
        }

        self.reg_a = (self.reg_a | val) & !(self.reg_a & val);

        self.update_nz(self.reg_a);
        if self.debug {
            println!("\tAccumulator(new) = {:x?}", self.reg_a);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }

    fn sax(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("SAX: ");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        if self.debug {
            println!("\tX Register = {:x?}", self.reg_x);
            println!("\tAccumulator = {:x?}", self.reg_a);
        }

        let val = self.reg_a & self.reg_x;

        self.mem_write(addr, val);
        if self.debug {
            println!("\tData = {:x?}", val);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }

    //Arithmetic Shift Left
    fn asl(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("ASL:");
        }
        match mode {
            AddressingMode::NoneAddressing => {
                if self.debug {
                    println!("\tAccumulator(old) = {:x?}", self.reg_a);
                }
                if self.reg_a & 0x80 != 0 {
                    self.reg_stat |= 0x01;
                } else {
                    self.reg_stat &= 0xFE;
                }
                self.reg_a <<= 1;

                self.update_nz(self.reg_a);
                if self.debug {
                    println!("\tAccumulator(new) = {:x?}", self.reg_a);
                    println!("\tStatus = {:x?}", self.reg_stat);
                }
            }
            _ => {
                let addr = self.get_op_addr(mode);
                if self.debug {
                    println!("\tMem Address = {:x?}", addr);
                }
                let val = self.mem_read(addr);
                if self.debug {
                    println!("\tData(old) = {:x?}", val);
                }

                if val & 0x80 != 0 {
                    self.reg_stat |= 0x01;
                } else {
                    self.reg_stat &= 0xFE;
                }
                self.mem_write(addr, val << 1);

                self.update_nz(val << 1);
                if self.debug {
                    println!("\tData(new) = {:x?}", self.mem_read(addr));
                    println!("\tStatus = {:x?}", self.reg_stat);
                }
            }
        }
    }
    //Logical Shift Right
    fn lsr(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("LSR:");
        }
        match mode {
            AddressingMode::NoneAddressing => {
                if self.debug {
                    println!("\tAccumulator(old) = {:x?}", self.reg_a);
                }
                if self.reg_a & 0x01 != 0 {
                    self.reg_stat |= 0x01;
                } else {
                    self.reg_stat &= 0xFE;
                }
                self.reg_a >>= 1;

                self.update_nz(self.reg_a);
                if self.debug {
                    println!("\tAccumulator(new) = {:x?}", self.reg_a);
                    println!("\tStatus = {:x?}", self.reg_stat);
                }
            }
            _ => {
                let addr = self.get_op_addr(mode);
                if self.debug {
                    println!("\tMem Address = {:x?}", addr);
                }
                let val = self.mem_read(addr);
                if self.debug {
                    println!("\tData(old) = {:x?}", val);
                }

                if val & 0x01 != 0 {
                    self.reg_stat |= 0x01;
                } else {
                    self.reg_stat &= 0xFE;
                }
                self.mem_write(addr, val >> 1);

                self.update_nz(val >> 1);
                if self.debug {
                    println!("\tData(new) = {:x?}", self.mem_read(addr));
                    println!("\tStatus = {:x?}", self.reg_stat);
                }
            }
        }
    }
    //Rotate Left
    fn rol(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("ROL:");
        }
        match mode {
            AddressingMode::NoneAddressing => {
                if self.debug {
                    println!("\tAccumulator(old) = {:x?}", self.reg_a);
                    println!("\tStatus(old) = {:x?}", self.reg_stat);
                }
                let mut oldcarry: u8 = 0;
                if self.reg_stat & 0x01 != 0 {
                    oldcarry = 1;
                }
                if self.reg_a & 0x80 != 0 {
                    self.reg_stat |= 0x01;
                } else {
                    self.reg_stat &= 0xFE;
                }
                self.reg_a <<= 1;
                self.reg_a |= oldcarry;

                self.update_nz(self.reg_a);
                
                if self.debug {
                    println!("\tAccumulator(new) = {:x?}", self.reg_a);
                    println!("\tStatus(new) = {:x?}", self.reg_stat);
                }
            }
            _ => {
                let addr = self.get_op_addr(mode);
                if self.debug {
                    println!("\tMem Address = {:x?}", addr);
                }
                let val = self.mem_read(addr);
                if self.debug {
                    println!("\tData(old) = {:x?}", val);
                    println!("\tStatus(old) = {:x?}", self.reg_stat);
                }

                let mut oldcarry: u8 = 0;
                if self.reg_stat & 0x01 != 0 {
                    oldcarry = 1;
                }
                if val & 0x80 != 0 {
                    self.reg_stat |= 0x01;
                } else {
                    self.reg_stat &= 0xFE;
                }
                self.mem_write(addr, (val << 1) | oldcarry);

                self.update_nz((val << 1) | oldcarry);
                if self.debug {
                    println!("\tData(new) = {:x?}", self.mem_read(addr));
                    println!("\tStatus = {:x?}", self.reg_stat);
                }
            }
        }
    }
    //Rotate Right
    fn ror(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("ROR:");
        }
        match mode {
            AddressingMode::NoneAddressing => {
                if self.debug {
                    println!("\tAccumulator(old) = {:x?}", self.reg_a);
                    println!("\tStatus(old) = {:x?}", self.reg_stat);
                }
                let mut oldcarry: u8 = 0;
                if self.reg_stat & 0x01 != 0 {
                    oldcarry = 1;
                }
                if self.reg_a & 0x01 != 0 {
                    self.reg_stat |= 0x01;
                } else {
                    self.reg_stat &= 0xFE;
                }
                self.reg_a >>= 1;
                self.reg_a |= oldcarry * 0x80;

                self.update_nz(self.reg_a);
                
                if self.debug {
                    println!("\tAccumulator(new) = {:x?}", self.reg_a);
                    println!("\tStatus(new) = {:x?}", self.reg_stat);
                }
            }
            _ => {
                let addr = self.get_op_addr(mode);
                if self.debug {
                    println!("\tMem Address = {:x?}", addr);
                }
                let val = self.mem_read(addr);
                if self.debug {
                    println!("\tData(old) = {:x?}", val);
                    println!("\tStatus(old) = {:x?}", self.reg_stat);
                }

                let mut oldcarry: u8 = 0;
                if self.reg_stat & 0x01 != 0 {
                    oldcarry = 1;
                }
                if val & 0x01 != 0 {
                    self.reg_stat |= 0x01;
                } else {
                    self.reg_stat &= 0xFE;
                }
                self.mem_write(addr, (val >> 1) | (oldcarry * 0x80));

                self.update_nz((val >> 1) | (oldcarry * 0x80));
                if self.debug {
                    println!("\tData(new) = {:x?}", self.mem_read(addr));
                    println!("\tStatus = {:x?}", self.reg_stat);
                }
            }
        }
    }

    //Branch if Carry Clear
    fn bcc(&mut self) {
        if self.debug {
            println!("BCC:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
        if self.reg_stat & 0x01 == 0 {
            if self.debug {
                println!("\tProgram Counter(old) = {:x?}", self.reg_pc);
                println!("\tOffset = {:x?}", self.mem_read(self.reg_pc));
            }
            let mut offset = self.mem_read(self.reg_pc) as u16;
            if offset &0x0080 != 0 {
                offset |= 0xFF00;
            }
            self.reg_pc = self.reg_pc.wrapping_add(offset).wrapping_add(0x0001_u16);
            if self.debug {
                println!("\tProgram Counter(new) = {:x?}", self.reg_pc);
            }
        }
    }
    //Branch if Carry Set
    fn bcs(&mut self) {
        if self.debug {
            println!("BCS:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
        if self.reg_stat & 0x01 != 0 {
            if self.debug {
                println!("\tProgram Counter(old) = {:x?}", self.reg_pc);
                println!("\tOffset = {:x?}", self.mem_read(self.reg_pc));
            }
            let mut offset = self.mem_read(self.reg_pc) as u16;
            if offset &0x0080 != 0 {
                offset |= 0xFF00;
            }
            self.reg_pc = self.reg_pc.wrapping_add(offset).wrapping_add(0x0001_u16);
            if self.debug {
                println!("\tProgram Counter(new) = {:x?}", self.reg_pc);
            }
        }
    }
    //Branch if Equal
    fn beq(&mut self) {
        if self.debug {
            println!("BEQ:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
        if self.reg_stat & 0x02 != 0 {
            if self.debug {
                println!("\tProgram Counter(old) = {:x?}", self.reg_pc);
                println!("\tOffset = {:x?}", self.mem_read(self.reg_pc));
            }
            let mut offset = self.mem_read(self.reg_pc) as u16;
            if offset &0x0080 != 0 {
                offset |= 0xFF00;
            }
            self.reg_pc = self.reg_pc.wrapping_add(offset).wrapping_add(0x0001_u16);
            if self.debug {
                println!("\tProgram Counter(new) = {:x?}", self.reg_pc);
            }
        }
    }
    //Bit Test
    fn bit(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("BIT:");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let val = self.mem_read(addr);
        if self.debug {
            println!("\tData = {:x?}", val);
            println!("\tAccumulator = {:x?}", self.reg_a);
            println!("\tStatus(old) = {:x?}", self.reg_stat);
        }

        if val & self.reg_a == 0 {
            self.reg_stat |= 0x02;
        } else {
            self.reg_stat &= 0xFD;
        }

        if val & 0x40 != 0 {
            self.reg_stat |= 0x40;
        } else {
            self.reg_stat &= 0xBF;
        }

        if val & 0x80 != 0 {
            self.reg_stat |= 0x80;
        } else {
            self.reg_stat &= 0x7F;
        }
        if self.debug {
            println!("\tStatus(new) = {:x?}", self.reg_stat);
        }

    }
    //Branch if Minus
    fn bmi(&mut self) {
        if self.debug {
            println!("BMI:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
        if self.reg_stat & 0x80 != 0 {
            if self.debug {
                println!("\tProgram Counter(old) = {:x?}", self.reg_pc);
                println!("\tOffset = {:x?}", self.mem_read(self.reg_pc));
            }
            let mut offset = self.mem_read(self.reg_pc) as u16;
            if offset &0x0080 != 0 {
                offset |= 0xFF00;
            }
            self.reg_pc = self.reg_pc.wrapping_add(offset).wrapping_add(0x0001_u16);
            if self.debug {
                println!("\tProgram Counter(new) = {:x?}", self.reg_pc);
            }
        }
    }
    //Branch if Not Equal
    fn bne(&mut self) {
        if self.debug {
            println!("BNE:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
        if self.reg_stat & 0x02 == 0 {
            if self.debug {
                println!("\tProgram Counter(old) = {:x?}", self.reg_pc);
                println!("\tOffset = {:x?}", self.mem_read(self.reg_pc));
            }
            let mut offset = self.mem_read(self.reg_pc) as u16;
            if offset &0x0080 != 0 {
                offset |= 0xFF00;
            }
            self.reg_pc = self.reg_pc.wrapping_add(offset).wrapping_add(0x0001_u16);
            if self.debug {
                println!("\tProgram Counter(new) = {:x?}", self.reg_pc);
            }
        }
    }
    //Branch if Positive
    fn bpl(&mut self) {
        if self.debug {
            println!("BPL:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
        if self.reg_stat & 0x80 == 0 {
            if self.debug {
                println!("\tProgram Counter(old) = {:x?}", self.reg_pc);
                println!("\tOffset = {:x?}", self.mem_read(self.reg_pc));
            }
            let mut offset = self.mem_read(self.reg_pc) as u16;
            if offset &0x0080 != 0 {
                offset |= 0xFF00;
            }
            self.reg_pc = self.reg_pc.wrapping_add(offset).wrapping_add(0x0001_u16);
            if self.debug {
                println!("\tProgram Counter(new) = {:x?}", self.reg_pc);
            }
        }
    }
    //Branch if Overflow Clear
    fn bvc(&mut self) {
        if self.debug {
            println!("BVC:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
        if self.reg_stat & 0x40 == 0 {
            if self.debug {
                println!("\tProgram Counter(old) = {:x?}", self.reg_pc);
                println!("\tOffset = {:x?}", self.mem_read(self.reg_pc));
            }
            let mut offset = self.mem_read(self.reg_pc) as u16;
            if offset &0x0080 != 0 {
                offset |= 0xFF00;
            }
            self.reg_pc = self.reg_pc.wrapping_add(offset).wrapping_add(0x0001_u16);
            if self.debug {
                println!("\tProgram Counter(new) = {:x?}", self.reg_pc);
            }
        }
    }
    //Branch if Overflow Set
    fn bvs(&mut self) {
        if self.debug {
            println!("BVS:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
        if self.reg_stat & 0x40 != 0 {
            if self.debug {
                println!("\tProgram Counter(old) = {:x?}", self.reg_pc);
                println!("\tOffset = {:x?}", self.mem_read(self.reg_pc));
            }
            let mut offset = self.mem_read(self.reg_pc) as u16;
            if offset &0x0080 != 0 {
                offset |= 0xFF00;
            }
            self.reg_pc = self.reg_pc.wrapping_add(offset).wrapping_add(0x0001_u16);
            if self.debug {
                println!("\tProgram Counter(new) = {:x?}", self.reg_pc);
            }
        }
    }

    //Force Interrupt
    fn brk(&mut self){
        if self.debug {
            println!("BRK: ");
            println!("\tProgram Counter(old) = {:x?}", self.reg_pc);
            println!("\tStatus(old) = {:x?}", self.reg_stat);
            println!("\tStack Pointer(old) = {:x?}", self.reg_sp);
        }
        self.stack_push16(self.reg_pc);
        self.stack_push(self.reg_stat| 0x30);
        self.reg_pc = self.mem_read16(0xFFFE);
        self.reg_stat |= 0x10;
        if self.debug {
            println!("\tProgram Counter(new) = {:x?}", self.reg_pc);
            println!("\tStatus(new) = {:x?}", self.reg_stat);
            println!("\tStack Pointer(new) = {:x?}", self.reg_sp);
        }
    }
    //Return from Interrupt
    fn rti(&mut self) {
        if self.debug {
            println!("RTI: ");
            println!("\tProgram Counter(old) = {:x?}", self.reg_pc);
            println!("\tStatus(old) = {:x?}", self.reg_stat);
            println!("\tStack Pointer(old) = {:x?}", self.reg_sp);
        }
        self.reg_stat = self.stack_pull();
        self.reg_stat |= 0x20;
        self.reg_stat &= 0xEF;

        let addr = self.stack_pull16();
        self.reg_pc = self.change_endian(addr);
        if self.debug {
            println!("\tProgram Counter(new) = {:x?}", self.reg_pc);
            println!("\tStatus(new) = {:x?}", self.reg_stat);
            println!("\tStack Pointer(new) = {:x?}", self.reg_sp);
        }
    }

    //Compare
    fn cmp(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("CMP:");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let mut val = self.mem_read(addr);
        if self.debug {
            println!("\tData = {:x?}", val);
            println!("\tAccumulator = {:x?}", self.reg_a);
        }
        val = !val;

        let mut place = 0x01;
        let mut cin = 1;
        let mut cout = 0;
        let mut res: u8 = 0;
        while place != 0x00 {
            if place != 0x01 {
                cin = cout;
            }
            let bits = 
                (place & val != 0) as u8 + 
                0b010 * (place & self.reg_a != 0) as u8 +
                0b100 * cin;
            match bits {
                0b000 => {
                    res &= !place;
                    cout = 0;
                }
                0b001 | 0b010 | 0b100 => {
                    res |= place;
                    cout = 0;
                }
                0b011 | 0b101 | 0b110 => {
                    res &= !place;
                    cout = 1;
                }
                0b111 => {
                    res |= place;
                    cout = 1;
                }
                _ => {
                    panic!("What the fuck?!?");
                }
            }
            place <<= 1;
        }
        if cout != 0 {
            self.reg_stat |= 0x01;
        } else {
            self.reg_stat &= 0xFE;
        }

        self.update_nz(res);
        if self.debug {
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Compare X Register
    fn cpx(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("CPX:");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let mut val = self.mem_read(addr);
        if self.debug {
            println!("\tData = {:x?}", val);
            println!("\tX Register = {:x?}", self.reg_x);
        }
        val = !val;

        let mut place = 0x01;
        let mut cin = 1;
        let mut cout = 0;
        let mut res: u8 = 0;
        while place != 0x00 {
            if place != 0x01 {
                cin = cout;
            }
            let bits = 
                (place & val != 0) as u8 + 
                0b010 * (place & self.reg_x != 0) as u8 +
                0b100 * cin;
            match bits {
                0b000 => {
                    res &= !place;
                    cout = 0;
                }
                0b001 | 0b010 | 0b100 => {
                    res |= place;
                    cout = 0;
                }
                0b011 | 0b101 | 0b110 => {
                    res &= !place;
                    cout = 1;
                }
                0b111 => {
                    res |= place;
                    cout = 1;
                }
                _ => {
                    panic!("What the fuck?!?");
                }
            }
            place <<= 1;
        }
        if cout != 0 {
            self.reg_stat |= 0x01;
        } else {
            self.reg_stat &= 0xFE;
        }

        self.update_nz(res);
        if self.debug {
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Compare Y Register
    fn cpy(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("CPY:");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let mut val = self.mem_read(addr);
        if self.debug {
            println!("\tData = {:x?}", val);
            println!("\tY Register = {:x?}", self.reg_y);
        }
        val = !val;

        let mut place = 0x01;
        let mut cin = 1;
        let mut cout = 0;
        let mut res: u8 = 0;
        while place != 0x00 {
            if place != 0x01 {
                cin = cout;
            }
            let bits = 
                (place & val != 0) as u8 + 
                0b010 * (place & self.reg_y != 0) as u8 +
                0b100 * cin;
            match bits {
                0b000 => {
                    res &= !place;
                    cout = 0;
                }
                0b001 | 0b010 | 0b100 => {
                    res |= place;
                    cout = 0;
                }
                0b011 | 0b101 | 0b110 => {
                    res &= !place;
                    cout = 1;
                }
                0b111 => {
                    res |= place;
                    cout = 1;
                }
                _ => {
                    panic!("What the fuck?!?");
                }
            }
            place <<= 1;
        }
        if cout != 0 {
            self.reg_stat |= 0x01;
        } else {
            self.reg_stat &= 0xFE;
        }

        self.update_nz(res);
        if self.debug {
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }

    //Clear Carry Flag
    fn clc(&mut self) {
        self.reg_stat &= 0xFE;

        if self.debug {
            println!("CLC:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Set Carry Flag
    fn sec(&mut self) {
        self.reg_stat |= 0x01;

        if self.debug {
            println!("SEC:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Clear Decimal Mode
    fn cld(&mut self) {
        self.reg_stat &= 0xF7;

        if self.debug {
            println!("CLD:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Set Decimal Mode
    fn sed(&mut self) {
        self.reg_stat |= 0x08;

        if self.debug {
            println!("SED:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Clear Interrupt Disable
    fn cli(&mut self) {
        self.reg_stat &= 0xFB;

        if self.debug {
            println!("CLI:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Set Interrupt Disable
    fn sei(&mut self) {
        self.reg_stat |= 0x04;

        if self.debug {
            println!("SEI:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Clear Overflow Flag
    fn clv(&mut self) {
        self.reg_stat &= 0xBF;

        if self.debug {
            println!("CLV:");
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }

    //Store Accumulator Value in Memory
    fn sta(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("STA:");
        }
        let addr = self.get_op_addr(mode);
        self.mem_write(addr, self.reg_a);
        if self.debug {
            println!("\tAddress = {:x?}", addr);
            println!("\tAccumulator = {:x?}", self.reg_a);
            println!("\tMem Content = {:x?}", self.mem_read(addr));
        }
    }
    //Store X Register Value in Memory
    fn stx(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("STX:");
        }
        let addr = self.get_op_addr(mode);
        self.mem_write(addr, self.reg_x);
        if self.debug {
            println!("\tAddress = {:x?}", addr);
            println!("\tX Register = {:x?}", self.reg_x);
            //println!("\tMem Content = {:x?}", self.mem_read(addr));
        }
    }
    //Store Y Register Value in Memory
    fn sty(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("STY:");
        }
        let addr = self.get_op_addr(mode);
        self.mem_write(addr, self.reg_y);
        if self.debug {
            println!("\tAddress = {:x?}", addr);
            println!("\tY Register = {:x?}", self.reg_y);
            println!("\tMem Content = {:x?}", self.mem_read(addr));
        }
    }

    //Transfer stack pointer to X
    fn tsx(&mut self) {
        self.reg_x = self.reg_sp;

        self.update_nz(self.reg_x);

        if self.debug {
            println!("TSX:");
            println!("\tX Register = {:x?}", self.reg_x);
            println!("\tStack Pointer = {:x?}", self.reg_sp);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Transfer x to stack pointer
    fn txs(&mut self) {
        self.reg_sp = self.reg_x;

        if self.debug {
            println!("TXS:");
            println!("\tX Register = {:x?}", self.reg_x);
            println!("\tStack Pointer = {:x?}", self.reg_sp);
        }
    }

    //Push accumulator on stack
    fn pha(&mut self) {
        self.stack_push(self.reg_a);
        if self.debug {
            println!("PHA:");
            println!("\tAccumulator = {:x?}", self.reg_a);
            println!("\tStack Pointer = {:x?}", self.reg_sp);
        }
    }
    //Push status register on stack
    fn php(&mut self) {

        self.stack_push(self.reg_stat| 0x30);
        if self.debug {
            println!("PHP:");
            println!("\tStatus = {:x?}", self.reg_stat);
            println!("\tStack Pointer = {:x?}", self.reg_sp);
        }
    }

    //Pull accumulator from stack
    fn pla(&mut self) {
        self.reg_a = self.stack_pull();

        self.update_nz(self.reg_a);
        if self.debug {
            println!("PLA:");
            println!("\tAccumulator = {:x?}", self.reg_a);
            println!("\tStack Pointer = {:x?}", self.reg_sp);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Pull status register from stack
    fn plp(&mut self) {
        self.reg_stat = self.stack_pull();
        
        self.reg_stat |= 0x20;
        self.reg_stat &= 0xEF;
        if self.debug {
            println!("PLP:");
            println!("\tStatus = {:x?}", self.reg_stat);
            println!("\tStack Pointer = {:x?}", self.reg_sp);
        }
    }

    //Increment Memory
    fn inc(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("INC: ");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let val = self.mem_read(addr);
        if self.debug {
            println!("\tData(old) = {:x?}", val);
        }
        if val == 0xFF {
            self.mem_write(addr, 0);
        } else {
            self.mem_write(addr, val + 1);
        }

        self.update_nz(val.wrapping_add(1));
        if self.debug {
            println!("\tData(new) = {:x?}", self.mem_read(addr));
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Decrement Memory
    fn dec(&mut self, mode: &AddressingMode) {
        if self.debug {
            println!("DEC: ");
        }
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("\tMem Address = {:x?}", addr);
        }
        let val = self.mem_read(addr);
        if self.debug {
            println!("\tData(old) = {:x?}", val);
        }
        if val == 0x00 {
            self.mem_write(addr, 0xFF);
        } else {
            self.mem_write(addr, val - 1);
        }

        self.update_nz(val.wrapping_sub(1));
        if self.debug {
            println!("\tData(new) = {:x?}", self.mem_read(addr));
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Increment X
    fn inx(&mut self) {
        if self.debug {
            println!("INX:");
            println!("\tX Register(old) = {:x?}", self.reg_x);
        }
        if self.reg_x == 0xFF {
            self.reg_x = 0;
        } else {
            self.reg_x += 1;
        }

        self.update_nz(self.reg_x);
        if self.debug {
            println!("\tX Register(new) = {:x?}", self.reg_x);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Decrement X
    fn dex(&mut self) {
        if self.debug {
            println!("DEX:");
            println!("\tX Register(old) = {:x?}", self.reg_x);
        }
        if self.reg_x == 0x00 {
            self.reg_x = 0xFF;
        } else {
            self.reg_x -= 1;
        }

        self.update_nz(self.reg_x);
        if self.debug {
            println!("\tX Register(new) = {:x?}", self.reg_x);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }

    //Increment Y
    fn iny(&mut self) {
        if self.debug {
            println!("INY:");
            println!("\tY Register(old) = {:x?}", self.reg_y);
        }
        if self.reg_y == 0xFF {
            self.reg_y = 0;
        } else {
            self.reg_y += 1;
        }

        self.update_nz(self.reg_y);
        if self.debug {
            println!("\tY Register(new) = {:x?}", self.reg_y);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }
    //Decrement Y
    fn dey(&mut self) {
        if self.debug {
            println!("DEY:");
            println!("\tY Register(old) = {:x?}", self.reg_y);
        }
        if self.reg_y == 0x00 {
            self.reg_y = 0xFF;
        } else {
            self.reg_y -= 1;
        }

        self.update_nz(self.reg_y);
        if self.debug {
            println!("\tY Register(new) = {:x?}", self.reg_y);
            println!("\tStatus = {:x?}", self.reg_stat);
        }
    }

    //Jump
    fn jmp(&mut self, mode: &AddressingMode) {
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("JMP:");
            println!("\tProgram Counter(old) = {:x?}", self.reg_pc);
        }

        self.reg_pc = addr;
        if self.debug {
            println!("\tAddress = {:x?}", addr);
            println!("\tProgram Counter(new) = {:x?}", self.reg_pc);
        }
    }
    //Jump to Subroutine
    fn jsr(&mut self, mode: &AddressingMode) {
        let addr = self.get_op_addr(mode);
        if self.debug {
            println!("JSR:");
            println!("\tAddress = {:x?}", addr);
            println!("\tProgram Counter(old) = {:x?}", self.reg_pc);
            println!("\tStack Pointer(old) = {:x?}", self.reg_sp);
        }

        self.stack_push16(self.reg_pc + 1);
        self.reg_pc = addr;
        if self.debug {
            println!("\tProgram Counter(new) = {:x?}", self.reg_pc);
            println!("\tStack Pointer(new) = {:x?}", self.reg_sp);
        }
    }
    //Return from Subroutine
    fn rts(&mut self) {
        if self.debug {
            println!("RTS:");
            println!("\tProgram Counter(old) = {:x?}", self.reg_pc);
            println!("\tStack Pointer(old) = {:x?}", self.reg_sp);
        }
        let addr = self.stack_pull16();
        self.reg_pc = self.change_endian(addr) + 1;
        if self.debug {
            println!("\tProgram Counter(new) = {:x?}", self.reg_pc);
            println!("\tStack Pointer(new) = {:x?}", self.reg_sp);
        }
    }

    //No Operation
    fn nop(&self) {
        if self.debug {
            println!("NOP:")
        }
    }

    fn interrupt_nmi(&mut self) {
        self.stack_push16(self.reg_pc);
        self.stack_push(self.reg_stat | 0x20 & 0xEF);
        self.reg_pc = self.mem_read16(0xFFFA);
        self.reg_stat |= 0x04;
    }

    //NZ flag handler
    fn update_nz(&mut self, res: u8) {
        if res == 0 {
            self.reg_stat |= 0x02; //set zero flag
        } else {
            self.reg_stat &= 0xFD; //unset zero flag
        }

        if res & 0x80 != 0 {
            self.reg_stat |= 0x80; //set negative flag
        } else {
            self.reg_stat &= 0x7F; //unset negative flag
        }
    }

    //bytecode interpreter
    pub fn interpret(&mut self) {
        if self.reg_pc < 0xFFFF {
            if self.mem_bus.poll_nmi_status() && !self.nmi_flag {
                self.interrupt_nmi();
                self.nmi_flag = true;
                self.cycles += 2;
            }
            else {
                if !self.mem_bus.poll_nmi_status() {
                    self.nmi_flag = false;
                }
                let code = self.mem_read(self.reg_pc);
                if self.brk_stop && code == 0x00 {
                    return;
                }
                if self.debug {
                    println!("${:x?}", self.reg_pc);
                }
                self.reg_pc+=1;

                let pc_state = self.reg_pc;
                let opcode = opcodes::OPCODES_MAP.get(&code).unwrap_or_else(|| panic!("OpCode {:x} is not recognized", code));
                self.cycles += opcode.cycles;

                //match opcodes to instructions
                match opcode.mnemonic {
                    "ADC" => self.adc(&opcode.mode),
                    "AND" => self.and(&opcode.mode),
                    "ASL" => self.asl(&opcode.mode),
                    "BCC" => self.bcc(),
                    "BCS" => self.bcs(),
                    "BEQ" => self.beq(),
                    "BIT" => self.bit(&opcode.mode),
                    "BMI" => self.bmi(),
                    "BNE" => self.bne(),
                    "BPL" => self.bpl(),
                    "BRK" => self.brk(),
                    "BVC" => self.bvc(),
                    "BVS" => self.bvs(),
                    "CLC" => self.clc(),
                    "CLD" => self.cld(),
                    "CLI" => self.cli(),
                    "CLV" => self.clv(),
                    "CMP" => self.cmp(&opcode.mode),
                    "CPX" => self.cpx(&opcode.mode),
                    "CPY" => self.cpy(&opcode.mode),
                    "DCP" => { self.dec(&opcode.mode); self.cmp(&opcode.mode);} ,
                    "DEC" => self.dec(&opcode.mode),
                    "DEX" => self.dex(),
                    "DEY" => self.dey(),
                    "EOR" => self.eor(&opcode.mode),
                    "INC" => self.inc(&opcode.mode),
                    "ISB" => { self.inc(&opcode.mode); self.sbc(&opcode.mode); },
                    "INX" => self.inx(),
                    "INY" => self.iny(),
                    "JMP" => self.jmp(&opcode.mode),
                    "JSR" => self.jsr(&opcode.mode),
                    "LDA" => self.lda(&opcode.mode),
                    "LDX" => self.ldx(&opcode.mode),
                    "LDY" => self.ldy(&opcode.mode),
                    "LAX" => {self.lda(&opcode.mode); self.ldx(&opcode.mode); },
                    "LSR" => self.lsr(&opcode.mode),
                    "NOP" => self.nop(),
                    "ORA" => self.ora(&opcode.mode),
                    "PHA" => self.pha(),
                    "PHP" => self.php(),
                    "PLA" => self.pla(),
                    "PLP" => self.plp(),
                    "RLA" => { self.rol(&opcode.mode); self.and(&opcode.mode); },
                    "ROL" => self.rol(&opcode.mode),
                    "ROR" => self.ror(&opcode.mode),
                    "RRA" => { self.ror(&opcode.mode); self.adc(&opcode.mode); },
                    "RTI" => self.rti(),
                    "RTS" => self.rts(),
                    "SAX" => self.sax(&opcode.mode),
                    "SBC" => self.sbc(&opcode.mode),
                    "SEC" => self.sec(),
                    "SED" => self.sed(),
                    "SEI" => self.sei(),
                    "SLO" => { self.asl(&opcode.mode); self.ora(&opcode.mode); },
                    "SRE" => { self.lsr(&opcode.mode); self.eor(&opcode.mode); },
                    "STA" => self.sta(&opcode.mode),
                    "STX" => self.stx(&opcode.mode),
                    "STY" => self.sty(&opcode.mode),
                    "TAX" => self.tax(),
                    "TAY" => self.tay(),
                    "TSX" => self.tsx(),
                    "TXA" => self.txa(),
                    "TXS" => self.txs(),
                    "TYA" => self.tya(),
                    _ => todo!(),
                }
                
                //Extra Cycles
                match opcode.mnemonic {
                    "ADC" | "AND" | "CMP" |
                    "EOR" | "LDA" | "LDX" |
                    "LDY" | "ORA" | "SBC" |
                    "NOP" | "LAX" => {
                        match opcode.mode {
                            AddressingMode::Absolute_X | AddressingMode::Absolute_Y => {
                                if self.get_op_addr(&opcode.mode) & 0xFF00 != self.get_op_addr(&AddressingMode::Absolute) & 0xFF00 {
                                    self.cycles += 1;
                                }
                            },
                            AddressingMode::Indirect_Y => {
                                if self.get_op_addr(&opcode.mode) & 0xFF00 != self.get_op_addr(&opcode.mode).wrapping_sub(self.reg_y as u16) & 0xFF00 {
                                    self.cycles += 1;
                                }
                            },
                            _ => {}
                        }
                    },
                    "BCC" | "BCS" | "BEQ" | "BVC" |
                    "BMI" | "BNE" | "BPL" | "BVS" => {
                        if pc_state != self.reg_pc {
                            self.cycles += 1;
                            if (pc_state + opcode.len as u16 - 1) & 0xFF00 != self.reg_pc & 0xFF00 {
                                self.cycles += 1;
                            }
                        }
                    },
                    _ => {}
                }
                if pc_state == self.reg_pc {
                    self.reg_pc += opcode.len as u16 - 1;
                }
            }
            if self.cycles > 0 {
                self.mem_bus.tick(3 * self.cycles);
                self.tot_cycles += self.cycles as u32;
                self.cycles = 0;
            }
        }
    }
}
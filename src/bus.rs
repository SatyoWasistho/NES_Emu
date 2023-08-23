use crate::cpu::Mem;
use crate::rom::Rom;
use crate::ppu::PPU;
use crate::input::Controller;

#[derive(Clone)]
pub struct Bus {
    cpu_vram: [u8; 0x800],
    prg_rom: Vec<u8>,
    pub ppu: PPU,
    pub port1: Controller,
    pub port2: Controller,
}

impl Bus {
    pub fn new(rom: Rom) -> Self {
        Bus {
            cpu_vram: [0; 2048],
            ppu: PPU::new(rom.chr_rom, rom.screen_mirroring),
            prg_rom: rom.prg_rom,
            port1: Controller::new(),
            port2: Controller::new(),
        }
    }
    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;
       if self.prg_rom.len() == 0x4000 && addr >= 0x4000 {
           //mirror if needed
           addr %= 0x4000;
       }
       self.prg_rom[addr as usize]
    }
    pub fn poll_nmi_status(&self) -> bool {
        self.ppu.nmi_interrupt
    }
    pub fn tick(&mut self, cycles: u8) {
        self.ppu.tick(cycles);
    }
 }

const RAM: u16 = 0x0000;
const RAM_MIRRORS_END: u16 = 0x1FFF;
const PPU_REGS: u16 = 0x2000;
const PPU_REGS_MIRRORS_END: u16 = 0x3FFF;
const ROM: u16 = 0x8000;
const ROM_END: u16 = 0xFFFF;

impl Mem for Bus {
    // read byte from memory
    fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0x07FF;
                self.cpu_vram[mirror_down_addr as usize]
            }
            0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x2006 | 0x4014 => {
                //panic!("Attempt to read from write-only PPU address {:x}", addr);
                0
            }
            0x2002 => self.ppu.stat_read(),
            0x2004 => self.ppu.oam_read(),
            0x2007 => self.ppu.read(),

            PPU_REGS..=PPU_REGS_MIRRORS_END => {
                let mirror_down_addr = addr & 0x2007;
                self.mem_read(mirror_down_addr)
            },
            0x4016 => self.port1.read(),
            0x4017 => self.port2.read(),
            ROM ..=ROM_END => self.read_prg_rom(addr),
            _ => {
                //println!("Ignoring mem access at ${:04x?}", addr);
                0
            }
        }
    }
    // write byte in memory
    fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            RAM ..= RAM_MIRRORS_END => {
                let mirror_down_addr = addr & 0x07FF;
                self.cpu_vram[mirror_down_addr as usize] = data;
            },
            0x2000 => {
                self.ppu.ctrl_write(data);
            },
            0x2001 => {
                self.ppu.mask_write(data);
            },
            0x2002 => {
                panic!("attempt to write to PPU status register");
            },
            0x2003 => {
                self.ppu.oam_addr_write(data);
            },
            0x2004 => {
                self.ppu.oam_write(data);
            }
            0x2005 => {
                self.ppu.scroll_write(data);
            },
            0x2006 => {
                self.ppu.addr_write(data);
            },
            0x2007 => {
                self.ppu.write(data);
            }
            0x2008 ..= PPU_REGS_MIRRORS_END => {
                let mirror_down_addr = addr & 0x2007;
                self.mem_write(mirror_down_addr, data);
            },
            0x4014 => {
                let mut oam_dma = [0; 256];
                let base_addr = ((data as usize) << 8) & 0x07FF;
                for i in 0..256 {
                    oam_dma[i] = self.cpu_vram[base_addr + i];
                }
                self.ppu.write_oam_dma(&oam_dma);
            },
            0x4016 => {
                if data & 0x01 != 0 {
                    self.port1.set_strobe();
                } else {
                    self.port1.reset_strobe();
                }
            },
            0x4017 => {
                if data & 0x01 != 0 {
                    self.port2.set_strobe();
                } else {
                    self.port2.reset_strobe();
                }
            }
            ROM ..= ROM_END => {
                panic!("Attempt to write to Cartridge ROM space")
            },
            _ => {
                //println!("Ignoring mem write-access at ${:X?}", addr);
            }
        }
    }
    // reads as little endian, returns big endian
    fn mem_read16(&mut self, addr: u16) -> u16 {
        let lo = self.mem_read(addr) as u16;
        let hi = self.mem_read(addr + 1) as u16;
        (hi << 8) | lo
    }
    // reads value as big endian, writes as little endian
    fn mem_write16(&mut self, addr: u16, val: u16) {
        let lo = (val >> 8) as u8;
        let hi = (val & 0x00FF) as u8;
        self.mem_write(addr, lo);
        self.mem_write(addr + 1, hi);
    }
}
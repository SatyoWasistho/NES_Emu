/****************** Linking External Modules ******************/
use crate::rom::Mirroring;
/**************************************************************/

#[derive(Clone)]
pub struct PPU {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; 0x20],
    pub vram: [u8; 0x0800],
    pub oam_data: [u8; 0x0100],

    pub mirroring: Mirroring,
    addr_hi: u8,
    addr_lo: u8,
    pub ctrl: u8,
    mask: u8,
    pub stat: u8,
    oam_addr: u8,
    pub fetch_scroll_x: u8,
    pub fetch_scroll_y: u8,
    pub scroll_x: u8,
    pub scroll_y: u8,
    data_buf: u8,

    pub v: u16,
    pub t: u16,
    pub x: u8,

    addr_latch: bool,

    pub cycles: u16,
    pub scanlines: u16,

    pub nmi_interrupt: bool,
}

impl PPU {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        PPU {
            chr_rom,
            palette_table: [0; 0x20],
            vram: [0; 0x0800],
            oam_data: [0; 0x0100],
            
            mirroring,
            addr_hi: 0,
            addr_lo: 0,
            /*
            Control Register - 
                7  bit  0
                ---- ----
                VPHB SINN
                |||| ||||
                |||| ||++- Base nametable address
                |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
                |||| |+--- VRAM address increment per CPU read/write of PPUDATA
                |||| |     (0: add 1, going across; 1: add 32, going down)
                |||| +---- Sprite pattern table address for 8x8 sprites
                ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
                |||+------ Background pattern table address (0: $0000; 1: $1000)
                ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels – see PPU OAM#Byte 1)
                |+-------- PPU master/slave select
                |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
                +--------- Generate an NMI at the start of the
                        vertical blanking interval (0: off; 1: on)
            */
            ctrl: 0x00,
            /*
            Mask Register -
                7  bit  0
                ---- ----
                BGRs bMmG
                |||| ||||
                |||| |||+- Greyscale (0: normal color, 1: produce a greyscale display)
                |||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
                |||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
                |||| +---- 1: Show background
                |||+------ 1: Show sprites
                ||+------- Emphasize red (green on PAL/Dendy)
                |+-------- Emphasize green (red on PAL/Dendy)
                +--------- Emphasize blue
            */
            mask: 0,
            /*
            Status Register - 
                7  bit  0
                ---- ----
                VSO. ....
                |||| ||||
                |||+-++++- PPU open bus. Returns stale PPU bus contents.
                ||+------- Sprite overflow. The intent was for this flag to be set
                ||         whenever more than eight sprites appear on a scanline, but a
                ||         hardware bug causes the actual behavior to be more complicated
                ||         and generate false positives as well as false negatives; see
                ||         PPU sprite evaluation. This flag is set during sprite
                ||         evaluation and cleared at dot 1 (the second dot) of the
                ||         pre-render line.
                |+-------- Sprite 0 Hit.  Set when a nonzero pixel of sprite 0 overlaps
                |          a nonzero background pixel; cleared at dot 1 of the pre-render
                |          line.  Used for raster timing.
                +--------- Vertical blank has started (0: not in vblank; 1: in vblank).
                        Set at dot 1 of line 241 (the line *after* the post-render
                        line); cleared after reading $2002 and at dot 1 of the
                        pre-render line.
            */
            stat: 0,
            oam_addr: 0,
            fetch_scroll_x: 0,
            fetch_scroll_y: 0,
            scroll_x: 0,
            scroll_y: 0,
            data_buf: 0,

            v: 0,
            t: 0,
            x: 0,

            addr_latch: false,

            cycles: 21,
            scanlines: 0,

            nmi_interrupt: false,

        }
    }

    pub fn tick(&mut self, cycles: u8) -> bool {
        for i in 0..cycles {
            self.cycles += 1;
            if self.cycles == 256 {
                if (self.v & 0x7000) != 0x7000 {
                    self.v += 0x1000;
                } else {
                    self.v &= 0x8FFF;
                    let mut y = (self.v & 0x03E0) >> 5;
                    if y == 29 {
                        y = 0;
                        self.v ^= 0x0800;
                    } else if y == 31 {
                        y = 0;
                    } else {
                        y += 1;
                    }
                    self.v = (self.v & 0xFC1F) | (y << 5);
                }
            }
            if self.cycles == 257 {
                self.scroll_x = self.fetch_scroll_x;
                self.v = (self.v & 0x7BE0) | (self.t & 0x041F);
            }
            if self.scanlines == 261 && self.cycles >= 280 && self.cycles <= 304 {
                self.scroll_y = self.fetch_scroll_y;
                self.v = (self.v & 0x041F) | (self.t & 0x7BE0);
            }
            if self.cycles == 328 || self.cycles == 336 || (
                self.cycles > 0 && self.cycles <= 256 && self.cycles % 8 == 0
            ) {
                if (self.v & 0x001F) == 31 {
                    self.v &= 0xFFE0;
                    self.v ^= 0x0400;
                } else {
                    self.v += 1;
                }
            }
            if self.cycles >= 341 {

                if self.is_sprite_0_hit(self.cycles as usize) {
                    self.stat |= 0x40;
                }

                self.cycles = 0;
                self.scanlines += 1;
            }
        }
        if self.scanlines == 241 {
            self.stat |= 0x80;
            self.stat &= 0xBF;
            if self.ctrl & 0x80 != 0 {
                self.nmi_interrupt = true;
            }
        }

        if self.scanlines > 261 {
            self.scanlines = 0;
            self.nmi_interrupt = false;
            self.stat &= 0x3F;
            return true;
        }
        false
    }

    fn is_sprite_0_hit(&self, cycle: usize) -> bool {
        let y = self.oam_data[0] as usize;
        let x = self.oam_data[3] as usize;
        (y == (self.scanlines - 4) as usize) && (x <= cycle) && (self.mask & 0x10 != 0)
    }

    pub fn addr_write(&mut self, val: u8) {
        if self.addr_latch {
            self.addr_lo = val;
            self.t = (self.t & 0x00FF) | (((val as u16) & 0x003F) << 8);
        } else {
            self.addr_hi = val;
            self.t = (self.t & 0xFF00) | ((val as u16) & 0x00FF);
            self.v = self.t;
        }
        self.addr_latch = !self.addr_latch;
    }
    pub fn ctrl_write(&mut self, val: u8) {
        let prev_nmi_stat = self.ctrl & 0x80 != 0;
        self.ctrl = val;
        if !prev_nmi_stat && self.ctrl & 0x80 != 0 && self.stat & 0x80 != 0 {
            self.nmi_interrupt = true;
        }
        self.t = (self.t & 0xF3FF) | (((val as u16) & 0x0003) << 10);
    }
    pub fn mask_write(&mut self, val: u8) {
        self.mask = val;
    }
    pub fn stat_read(&mut self) -> u8 {
        let res = self.stat;
        self.stat &= 0x7F;
        self.addr_latch = false;
        res
    }
    fn inc_vram_addr(&mut self) {
        let (lo, cout) = self.addr_lo.overflowing_add(if self.ctrl & 0x04 != 0 { 32 } else { 1 });
        self.addr_lo = lo;
        if cout {
            self.addr_hi += 1;
        }
        self.v += if self.ctrl & 0x04 != 0 { 32 } else { 1 };

    }
    pub fn oam_addr_write(&mut self, val: u8) {
        self.oam_addr = val;
    }
    pub fn oam_write(&mut self, val: u8) {
        self.oam_data[self.oam_addr as usize] = val;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }
    pub fn oam_read(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
    }
    pub fn write_oam_dma(&mut self, data: &[u8; 256]) {
        for i in 0..256 {
            self.oam_data[self.oam_addr as usize + i] = data[i];
        }
        self.tick(255);
        self.tick(255);
        
        self.tick(3 + (self.cycles % 2) as u8);
        /*
        
        let (left, right) = self.oam_data.split_at_mut(self.oam_addr as usize + 1);
        left.copy_from_slice(&data[(255-self.oam_addr) as usize..=255]);
        right.copy_from_slice(&data[0..=(255-self.oam_addr) as usize]);
        */
    }
    pub fn scroll_write(&mut self, val: u8) {
        if self.addr_latch {
            self.fetch_scroll_y = val;
            self.t = (self.t & 0xFFE0) | ((val as u16) >> 3);
            self.x = val &0x03;
        } else {
            self.fetch_scroll_x = val;
            self.t = (self.t & 0x0C1F) | ((val as u16) << 12) | (((val as u16) & 0xF8) << 2);
        }
        self.addr_latch = !self.addr_latch;
    }
    pub fn read(&mut self) -> u8 {
        let addr = ((self.addr_hi as u16) << 8) | self.addr_lo as u16;
        self.inc_vram_addr();

        match addr {
            0x0000..=0x1FFF => {
                let res = self.data_buf;
                self.data_buf = self.chr_rom[addr as usize];
                res
            },
            0x2000..=0x2FFF => {
                let res = self.data_buf;
                self.data_buf = self.vram[self.mirror_vram_addr(addr) as usize];
                res
            },
            0x3000..=0x3EFF => panic!("addr space 0x3000..0x3eff is not expected to be used, requested = {:X?} ", addr),
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror & 0x001F) as usize]
            },
            0x3F00..=0x3FFF => {
                self.palette_table[(addr & 0x001F) as usize]
            },
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
    }
    pub fn write(&mut self, data: u8) {
        let addr = ((self.addr_hi as u16) << 8) | self.addr_lo as u16;

        match addr {
            0x0000..=0x1FFF => {
                println!("attempt to write to chr rom space ${:04X?}", addr);
            },
            0x2000..=0x2FFF => {
                self.vram[self.mirror_vram_addr(addr) as usize] = data;
            },
            0x3000..=0x3EFF => panic!("addr space 0x3000..0x3eff is not expected to be used, requested = {} ", addr),
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror & 0x001F) as usize] = data;
            },
            0x3F00..=0x3FFF => {
                self.palette_table[(addr & 0x001F) as usize] = data;
            },
            _ => {panic!("unexpected access to mirrored space ${:04X?}", addr)},
        }
        self.inc_vram_addr();
    }

    pub fn mirror_vram_addr(&self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0x2FFF;
        let vram_idx = mirrored_vram - 0x2000;
        let name_table = vram_idx / 0x400;

        match(&self.mirroring, name_table) {
            (Mirroring::VERTICAL, 2) | (Mirroring::VERTICAL, 3) |
            (Mirroring::HORIZONTAL, 3) => vram_idx - 0x800,
            (Mirroring::HORIZONTAL, 2) | (Mirroring::HORIZONTAL, 1) => vram_idx - 0x400,
            _ => vram_idx,
        }
    }
}

#[rustfmt::skip]

pub static SYSTEM_PALLETE: [(u8,u8,u8); 64] = [
   (0x80, 0x80, 0x80), (0x00, 0x3D, 0xA6), (0x00, 0x12, 0xB0), (0x44, 0x00, 0x96), (0xA1, 0x00, 0x5E),
   (0xC7, 0x00, 0x28), (0xBA, 0x06, 0x00), (0x8C, 0x17, 0x00), (0x5C, 0x2F, 0x00), (0x10, 0x45, 0x00),
   (0x05, 0x4A, 0x00), (0x00, 0x47, 0x2E), (0x00, 0x41, 0x66), (0x00, 0x00, 0x00), (0x05, 0x05, 0x05),
   (0x05, 0x05, 0x05), (0xC7, 0xC7, 0xC7), (0x00, 0x77, 0xFF), (0x21, 0x55, 0xFF), (0x82, 0x37, 0xFA),
   (0xEB, 0x2F, 0xB5), (0xFF, 0x29, 0x50), (0xFF, 0x22, 0x00), (0xD6, 0x32, 0x00), (0xC4, 0x62, 0x00),
   (0x35, 0x80, 0x00), (0x05, 0x8F, 0x00), (0x00, 0x8A, 0x55), (0x00, 0x99, 0xCC), (0x21, 0x21, 0x21),
   (0x09, 0x09, 0x09), (0x09, 0x09, 0x09), (0xFF, 0xFF, 0xFF), (0x0F, 0xD7, 0xFF), (0x69, 0xA2, 0xFF),
   (0xD4, 0x80, 0xFF), (0xFF, 0x45, 0xF3), (0xFF, 0x61, 0x8B), (0xFF, 0x88, 0x33), (0xFF, 0x9C, 0x12),
   (0xFA, 0xBC, 0x20), (0x9F, 0xE3, 0x0E), (0x2B, 0xF0, 0x35), (0x0C, 0xF0, 0xA4), (0x05, 0xFB, 0xFF),
   (0x5E, 0x5E, 0x5E), (0x0D, 0x0D, 0x0D), (0x0D, 0x0D, 0x0D), (0xFF, 0xFF, 0xFF), (0xA6, 0xFC, 0xFF),
   (0xB3, 0xEC, 0xFF), (0xDA, 0xAB, 0xEB), (0xFF, 0xA8, 0xF9), (0xFF, 0xAB, 0xB3), (0xFF, 0xD2, 0xB0),
   (0xFF, 0xEF, 0xA6), (0xFF, 0xF7, 0x9C), (0xD7, 0xE8, 0x95), (0xA6, 0xED, 0xAF), (0xA2, 0xF2, 0xDA),
   (0x99, 0xFF, 0xFC), (0xDD, 0xDD, 0xDD), (0x11, 0x11, 0x11), (0x11, 0x11, 0x11)
];
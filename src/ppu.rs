/****************** Linking External Modules ******************/
pub mod two_write_reg;
use crate::rom::Mirroring;
use two_write_reg::TwoWriteReg;
/**************************************************************/

#[derive(Clone)]
pub struct PPU {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; 0x20],
    pub vram: [u8; 0x0800],
    pub oam_data: [u8; 0x0100],

    pub mirroring: Mirroring,
    addr: TwoWriteReg,
    pub ctrl: u8,
    mask: u8,
    pub stat: u8,
    oam_addr: u8,
    pub scroll: TwoWriteReg,
    data_buf: u8,

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
            addr: TwoWriteReg::new(true),
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
            scroll: TwoWriteReg::new(false),
            data_buf: 0,

            cycles: 21,
            scanlines: 0,

            nmi_interrupt: false,
        }
    }

    pub fn tick(&mut self, cycles: u8) -> bool {
        self.cycles += cycles as u16;
        if self.cycles >= 341 {

            if self.is_sprite_0_hit(self.cycles as usize) {
                self.stat |= 0x40;
            }

            self.cycles -= 341;
            self.scanlines += 1;

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
        }
        false
    }

    fn is_sprite_0_hit(&self, cycle: usize) -> bool {
        let y = self.oam_data[0] as usize;
        let x = self.oam_data[3] as usize;
        (y == self.scanlines as usize) && (x <= cycle) && (self.mask & 0x10 != 0)
    }

    pub fn addr_write(&mut self, val: u8) {
        self.addr.update(val);
    }
    pub fn ctrl_write(&mut self, val: u8) {
        let prev_nmi_stat = self.ctrl & 0x80 != 0;
        self.ctrl = val;
        if !prev_nmi_stat && self.ctrl & 0x80 != 0 && self.stat & 0x80 != 0 {
            self.nmi_interrupt = true;
        }
    }
    pub fn mask_write(&mut self, val: u8) {
        self.mask = val;
    }
    pub fn stat_read(&mut self) -> u8 {
        let res = self.stat;
        self.stat &= 0x7F;
        self.addr.set_latch();
        self.scroll.reset_latch();
        res
    }
    fn inc_vram_addr(&mut self) {
        self.addr.inc(if self.ctrl & 0x04 != 0 { 32 } else { 1 });
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
        /*
        
        let (left, right) = self.oam_data.split_at_mut(self.oam_addr as usize + 1);
        left.copy_from_slice(&data[(255-self.oam_addr) as usize..=255]);
        right.copy_from_slice(&data[0..=(255-self.oam_addr) as usize]);
        */
    }
    pub fn scroll_write(&mut self, val: u8) {
        self.scroll.update(val);
    }
    pub fn read(&mut self) -> u8 {
        let addr = self.addr.get();
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
        let addr = self.addr.get();

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
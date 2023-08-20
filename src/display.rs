use crate::ppu::PPU;
use crate::rom::Mirroring;

const WIDTH: usize = 256;
const HEIGHT: usize = 240;

pub struct Rect {
    pub x1: usize,
    pub y1: usize,
    pub x2: usize,
    pub y2: usize,
}

pub struct Frame {
    pub data: [u8; WIDTH * HEIGHT * 4],
 }

 impl Frame {
 
    pub fn new(color: (u8, u8, u8)) -> Self {
        let mut data = [0x00; WIDTH * HEIGHT * 4];
        for i in 0..(WIDTH * HEIGHT) {
            data[i * 4 + 0] = color.0;
            data[i * 4 + 1] = color.1;
            data[i * 4 + 2] = color.2;
            data[i * 4 + 3] = 0xFF;
        }
        Frame {
            data: data,
        }
    }
 
    #[inline(always)]
    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        static mut base: usize = 0;
        unsafe {
            base = y * 4 * WIDTH + x * 4;
            if base < self.data.len() {
                self.data[base] = rgb.0;
                self.data[base + 1] = rgb.1;
                self.data[base + 2] = rgb.2;
            }
        }
    }
 }
 #[inline(always)]
 pub fn bg_palette(ppu: &PPU, name_table_offset: usize, tile_column: usize, tile_row: usize) -> [u8;4] {
    let attr_table_idx: usize = tile_row / 4 * 8 + tile_column / 4;
    let attr_byte: u8 = ppu.vram[0x03C0 + attr_table_idx + name_table_offset];

    let palette_idx: u8 = match ((tile_column %4) >> 1) + ((tile_row % 4) & 2) {
        0 => attr_byte & 0b11,
        1 => (attr_byte >> 2) & 0b11,
        2 => (attr_byte >> 4) & 0b11,
        3 => (attr_byte >> 6) & 0b11,
        _ => panic!("should not happen"),
    };
    let palette_start: usize = (1 + palette_idx * 4) as usize;
    [
        ppu.palette_table[0], 
        ppu.palette_table[palette_start+1], 
        ppu.palette_table[palette_start], 
        ppu.palette_table[palette_start+2]
    ]
 }
 #[inline(always)]
 pub fn sprite_palette(ppu: &PPU, palette_idx: u8) -> [u8; 4] {
    let start: usize = 0x11 + (palette_idx * 4) as usize;
    [
        0,
        ppu.palette_table[start + 1],
        ppu.palette_table[start],
        ppu.palette_table[start + 2],
    ]
}
#[inline(always)]
 pub fn show_tile(frame: &mut Frame, chr_rom: &Vec<u8>, bank: usize, tile_n: usize, viewport: Rect, x_offset: usize, y_offset: usize, x_scroll: isize, y_scroll: isize, palette: [u8; 4]) {
    if bank > 1 {
        return;
    }
 
    let tile: &[u8] = &chr_rom[(bank * 0x1000 + tile_n * 16)..=(bank * 0x1000 + tile_n * 16 + 15)];

    static mut upper: u8 = 0;
    static mut lower: u8 = 0;
    
    static mut value: u8 = 0;

    static mut rgb: (u8, u8, u8) = SYSTEM_PALLETE[0];

    for y in 0..=7 {
        unsafe {
            upper = tile[y];
            lower = tile[y + 8];
        }
 
        for x in (0..=7).rev() {
            unsafe {
                value = (1 & upper) << 1 | (1 & lower);
                upper >>= 1;
                lower >>= 1;
                rgb = SYSTEM_PALLETE[palette[value as usize] as usize];
                if (
                    x_offset + x >= viewport.x1 && 
                    x_offset + x <  viewport.x2 && 
                    y_offset + y >= viewport.y1 && 
                    y_offset + y <  viewport.y2 &&
                    value > 0
                 ) {
                    frame.set_pixel(((x_offset + x) as isize - x_scroll) as usize, ((y_offset + y) as isize - y_scroll) as usize, rgb);
                }
            }
        }
    }
 }
 #[inline(always)]
 pub fn show_sprite_tile(frame: &mut Frame, chr_rom: &Vec<u8>, bank: usize, tile_n: usize, x_offset: usize, y_offset: usize, flip_vertical: bool, flip_horizontal: bool, palette: [u8; 4]) {
    if bank > 1 {
        return;
    }
 
    let tile = &chr_rom[(bank * 0x1000 + tile_n * 16)..=(bank * 0x1000 + tile_n * 16 + 15)];

    
    for y in 0..=7 {
        let mut upper = tile[y];
        let mut lower = tile[y + 8];
 
        for x in (0..=7).rev() {
            let value = (1 & upper) << 1 | (1 & lower);
            upper >>= 1;
            lower >>= 1;
            let rgb = SYSTEM_PALLETE[palette[value as usize] as usize];
            if value > 0 {
                match (flip_horizontal, flip_vertical) {
                    (false, false) => frame.set_pixel(x_offset + x, y_offset + y, rgb),
                    (true, false) => frame.set_pixel(x_offset + 7 - x, y_offset + y, rgb),
                    (false, true) => frame.set_pixel(x_offset + x, y_offset + 7 - y, rgb),
                    (true, true) => frame.set_pixel(x_offset + 7 - x, y_offset + 7 - y, rgb),
                }
            }
        }
    }
 }

 pub fn render(ppu: &PPU, frame: &mut Frame, scanline_start: usize, scanline_stop: usize) {
    
    let scroll_x = ppu.scroll.val.1 as usize;
    let scroll_y = ppu.scroll.val.0 as usize;
    let bg_bank = ((ppu.ctrl & 0x10) >> 4) as usize;

    let (main_nametable, second_nametable) = match (&ppu.mirroring, ppu.ctrl & 0x03) {
        (Mirroring::FOUR_SCREEN,_) => {
            (0x0000, 0x0000)
        },
        (_, 0x00) | (Mirroring::VERTICAL, 0x02) |
        (Mirroring::HORIZONTAL, 0x01) => {
            (0x0000, 0x0400)
        },
        (Mirroring::VERTICAL, 0x01) | (_, 0x03) |
        (Mirroring::HORIZONTAL, 0x02) => {
            (0x0400, 0x0000)
        },(_,_) => {
            (0x0000, 0x0000)
        },
    };
    for i in (0..ppu.oam_data.len()).step_by(4).rev() {
        if ppu.oam_data[i + 2] & 0x20 != 0 {
            let tile_idx = ppu.oam_data[i + 1] as u16;
            let tile_x = ppu.oam_data[i + 3] as usize;
            let tile_y = ppu.oam_data[i] as usize;

            let flip_vertical = ppu.oam_data[i + 2] >> 7 & 1 == 1;
            let flip_horizontal = ppu.oam_data[i + 2] >> 6 & 1 == 1;

            let palette_idx = ppu.oam_data[i + 2] & 0x03;
            let sprite_palette = sprite_palette(ppu, palette_idx);

            if ppu.ctrl & 0x20 == 0 {
                let sprite_bank = ((ppu.ctrl & 0x08) >> 3) as usize;

                show_sprite_tile(frame, &ppu.chr_rom, sprite_bank, tile_idx as usize, tile_x, tile_y, flip_vertical, flip_horizontal, sprite_palette);
            } else {
                if flip_vertical {
                    show_sprite_tile(frame, &ppu.chr_rom, tile_idx as usize & 0x01, tile_idx as usize | 0x01, tile_x, tile_y, flip_vertical, flip_horizontal, sprite_palette);
                    show_sprite_tile(frame, &ppu.chr_rom, tile_idx as usize & 0x01, tile_idx as usize & 0xFE, tile_x, tile_y + 8, flip_vertical, flip_horizontal, sprite_palette);
                } else {
                    show_sprite_tile(frame, &ppu.chr_rom, tile_idx as usize & 0x01, tile_idx as usize & 0xFE, tile_x, tile_y, flip_vertical, flip_horizontal, sprite_palette);
                    show_sprite_tile(frame, &ppu.chr_rom, tile_idx as usize & 0x01, tile_idx as usize | 0x01, tile_x, tile_y + 8, flip_vertical, flip_horizontal, sprite_palette);
                }
            }
        }
    }

    for i in 0..0x03C0 {
        let tile_n = ppu.vram[i + main_nametable] as usize;
        let x = i % 32_usize;
        let y = i / 32_usize;
        let palette = bg_palette(ppu, main_nametable, x, y);

        if y * 8 >= scanline_start && y * 8 < scanline_stop {
        
            show_tile(
                frame, 
                &ppu.chr_rom, 
                bg_bank, 
                tile_n, 
                Rect {
                    x1: scroll_x,
                    y1: scroll_y,
                    x2: 256,
                    y2: 240,
                },
                x * 8,
                y * 8,
                scroll_x as isize, 
                scroll_y as isize, 
                palette
            );
        }
    }
    if scroll_x > 0 {
        for i in 0..0x03C0 {
            let tile_n = ppu.vram[i + second_nametable] as usize;
            let x = i % 32_usize;
            let y = i / 32_usize;
            let palette = bg_palette(ppu, second_nametable, x, y);
            
            if y * 8 >= scanline_start && y * 8 < scanline_stop {
                show_tile(
                    frame, 
                    &ppu.chr_rom, 
                    bg_bank, 
                    tile_n, 
                    Rect {
                        x1: 0,
                        y1: 0,
                        x2: 256 + scroll_x,
                        y2: 240 + scroll_y,
                    },
                    x * 8 + 256,
                    y * 8 + 0,
                    scroll_x as isize, 
                    0, 
                    palette
                );
            }
        }
    } else if scroll_y > 0 {
        for i in 0..0x03C0 {
            let tile_n = ppu.vram[i + second_nametable] as usize;
            let x = i % 32_usize;
            let y = i / 32_usize;
            let palette = bg_palette(ppu, second_nametable, x, y);
            
            if y * 8 + 240 - scroll_y >= scanline_start && y * 8 + 240 - scroll_y < scanline_stop {
                show_tile(
                    frame, 
                    &ppu.chr_rom, 
                    bg_bank, 
                    tile_n, 
                    Rect {
                        x1: 0,
                        y1: 0,
                        x2: 256 + scroll_x,
                        y2: 240 + scroll_y,
                    },
                    x * 8,
                    y * 8 + 240,
                    0, 
                    scroll_y as isize, 
                    palette
                );
            }
        }
    }
    for i in (0..ppu.oam_data.len()).step_by(4).rev() {
        if ppu.oam_data[i + 2] & 0x20 == 0 {
            let tile_idx = ppu.oam_data[i + 1] as u16;
            let tile_x = ppu.oam_data[i + 3] as usize;
            let tile_y = ppu.oam_data[i] as usize;

            let flip_vertical = ppu.oam_data[i + 2] >> 7 & 1 == 1;
            let flip_horizontal = ppu.oam_data[i + 2] >> 6 & 1 == 1;

            let palette_idx = ppu.oam_data[i + 2] & 0x03;
            let sprite_palette = sprite_palette(ppu, palette_idx);

            if ppu.ctrl & 0x20 == 0 {
                let sprite_bank = ((ppu.ctrl & 0x08) >> 3) as usize;

                show_sprite_tile(frame, &ppu.chr_rom, sprite_bank, tile_idx as usize, tile_x, tile_y, flip_vertical, flip_horizontal, sprite_palette);
            } else {
                if flip_vertical {
                    show_sprite_tile(frame, &ppu.chr_rom, tile_idx as usize & 0x01, tile_idx as usize | 0x01, tile_x, tile_y, flip_vertical, flip_horizontal, sprite_palette);
                    show_sprite_tile(frame, &ppu.chr_rom, tile_idx as usize & 0x01, tile_idx as usize & 0xFE, tile_x, tile_y + 8, flip_vertical, flip_horizontal, sprite_palette);
                } else {
                    show_sprite_tile(frame, &ppu.chr_rom, tile_idx as usize & 0x01, tile_idx as usize & 0xFE, tile_x, tile_y, flip_vertical, flip_horizontal, sprite_palette);
                    show_sprite_tile(frame, &ppu.chr_rom, tile_idx as usize & 0x01, tile_idx as usize | 0x01, tile_x, tile_y + 8, flip_vertical, flip_horizontal, sprite_palette);
                }
            }
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
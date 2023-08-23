#[derive(Debug, PartialEq, Clone)]
pub enum Mirroring {
    VERTICAL,
    HORIZONTAL,
    FOUR_SCREEN,
}

const NES_TAG: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];
const PRG_ROM_PAGE_SIZE: u16 = 0x4000;
const CHR_ROM_PAGE_SIZE: u16 = 0x2000;

pub struct Rom {
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mapper: u8,
    pub screen_mirroring: Mirroring,
}

impl Rom {
    //convert raw bytecode to formatted ROM
    pub fn new(raw: &Vec<u8>) -> Self {
        // check header format
        if &raw[0..4] != NES_TAG {
            panic!("File is not in iNES file format");
        }
        
        let mapper = (raw[7] & 0xF0) | (raw[6] >> 4);

        let ines_ver = (raw[7] >> 2) & 0x03;
        if ines_ver != 0 {
            panic!("NES2.0 format not supported");
        }
        let screen_mirroring: Mirroring;
        if raw[6] & 0x08 != 0 {
            println!("Mirror Type: Four Screen");
            screen_mirroring = Mirroring::FOUR_SCREEN;
        }
        else if raw[6] &0x01 != 0 {
            println!("Mirror Type: Vertical");
            screen_mirroring = Mirroring::VERTICAL;
        }
        else {
            println!("Mirror Type: Horizontal");
            screen_mirroring = Mirroring::HORIZONTAL;
        }

        let prg_rom_size = raw[4] as u16 * PRG_ROM_PAGE_SIZE;
        let chr_rom_size = raw[5] as u16 * CHR_ROM_PAGE_SIZE;

        println!("Program Rom Size: {} bytes", prg_rom_size);
        println!("Character Rom Size: {} bytes", chr_rom_size);

        let prg_rom_start: u16 = 0x0010 + 0x0080 * (raw[6] & 0x04) as u16;
        let chr_rom_start: u16 = prg_rom_start + prg_rom_size;

        Rom {
            prg_rom: raw[prg_rom_start as usize..(prg_rom_start + prg_rom_size) as usize].to_vec(),
            chr_rom: raw[chr_rom_start as usize..(chr_rom_start + chr_rom_size) as usize].to_vec(),
            mapper,
            screen_mirroring,
        }
    }
}
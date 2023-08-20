
#[derive(Clone)]
pub struct TwoWriteReg {
    pub val: (u8, u8),
    hi_ptr: bool,
}

impl TwoWriteReg {
    pub fn new(latch: bool) -> Self {
        TwoWriteReg {
            val: (0, 0),
            hi_ptr: latch,
        }
    }
    fn set(&mut self, data: u16) {
        self.val.0 = (data >> 8) as u8;
        self.val.1 = (data & 0x00FF) as u8;
    }
    pub fn update(&mut self, data: u8) {
        if self.hi_ptr {
            self.val.0 = data;
        } else {
            self.val.1 = data;
        }
        self.hi_ptr = !self.hi_ptr;
    }

    pub fn inc(&mut self, data: u8) {
        self.set(self.get().wrapping_add(data as u16));
    }
    pub fn set_latch(&mut self) {
        self.hi_ptr = true;
    }
    pub fn reset_latch(&mut self) {
        self.hi_ptr = false;
    }
    pub fn get(&self) -> u16 {
        ((self.val.0 as u16) << 8) | (self.val.1 as u16)
    }
}
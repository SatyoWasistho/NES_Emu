use crate::cpu::CPU;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref test_bmp_display: Vec<u8> = vec![
        0xA9, 0x00,         //$8000 LDA #0x00     ; set pointer at $10 to $0200
        0x85, 0x10,         //$8002 STA 0x10
        0xA9, 0x02,         //$8004 LDA #0x02
        0x85, 0x11,         //$8006 STA 0x11

        0xA2, 0x06,         //$8008 LDX #0x06     ; max value for 0x11, the high byte of the pointer

        0xA0, 0x00,         //$800A LDY #0x00     ; index - this value is added to the pointer

        0xA9, 0x07,         //$800C LDA #0x07     ; colour code to be used to fill the display

                            //loop:
        0x91, 0x10,         //$800E STA (0x10), X ; store colour to the value of the pointer + y
        0xC8,               //$8010 INY           ; increment index - prepare to fill next pixel
        0xD0, 0xFE,         //$8011 BNE loop      ; branch until page done - stops when Y==0

        0xE6, 0x11,         //$8013 INC 0x11      ; increment high byte of pointer
        0xE4, 0x11,         //$8015 CPX 0x11      ; compare with max value
        0xD0, 0xF8,         //$8017 BNE loop      ; continue if not done
        0x00                //$8019 BRK           ; done - return to debugger
    ];

    pub static ref snake_code: Vec<u8> = vec![
        0x20, 0x06, 0x06, 
        0x20, 0x38, 0x06, 
        0x20, 0x0d, 0x06, 
        0x20, 0x2a, 0x06, 
        0x60, 
        0xa9, 0x02, 
        0x85, 0x02, 
        0xa9, 0x04, 
        0x85, 0x03, 
        0xa9, 0x11, 
        0x85, 0x10, 
        0xa9, 0x10, 
        0x85, 0x12, 
        0xa9, 0x0f, 
        0x85, 0x14, 
        0xa9, 0x04, 
        0x85, 0x11, 
        0x85, 0x13, 
        0x85, 0x15, 
        0x60, 
        0xa5, 0xfe, 
        0x85, 0x00, 
        0xa5, 0xfe,
        0x29, 0x03, 
        0x18, 
        0x69, 0x02, 
        0x85, 0x01, 
        0x60, 
        0x20, 0x4d, 0x06, 
        0x20, 0x8d, 0x06, 
        0x20, 0xc3, 0x06, 
        0x20, 0x19, 0x07, 
        0x20, 0x20, 0x07, 
        0x20, 0x2d, 0x07, 
        0x4c, 0x38, 0x06, 
        0xa5, 0xff, 
        0xc9, 0x77, 
        0xf0, 0x0d, 
        0xc9, 0x64, 
        0xf0, 0x14, 
        0xc9, 0x73, 
        0xf0, 0x1b, 
        0xc9, 0x61, 
        0xf0, 0x22, 
        0x60,
        0xa9, 0x04, 
        0x24, 0x02, 
        0xd0, 0x26, 
        0xa9, 0x01, 
        0x85, 0x02, 
        0x60, 
        0xa9, 0x08, 
        0x24, 0x02, 
        0xd0, 0x1b, 
        0xa9, 0x02, 
        0x85, 0x02, 
        0x60, 
        0xa9, 0x01, 
        0x24, 0x02, 
        0xd0, 0x10, 
        0xa9, 0x04, 
        0x85, 0x02,
        0x60, 
        0xa9, 0x02, 
        0x24, 0x02, 
        0xd0, 0x05, 
        0xa9, 0x08, 
        0x85, 0x02, 
        0x60, 
        0x60, 
        0x20, 0x94, 0x06,
        0x20, 0xa8, 0x06, 
        0x60, 
        0xa5, 0x00, 
        0xc5, 0x10, 
        0xd0, 0x0d, 
        0xa5, 0x01, 
        0xc5, 0x11, 
        0xd0, 0x07,
        0xe6, 0x03, 
        0xe6, 0x03, 
        0x20, 0x2a, 0x06, 
        0x60, 
        0xa2, 0x02, 
        0xb5, 0x10, 
        0xc5, 0x10, 
        0xd0, 0x06,
        0xb5, 0x11, 
        0xc5, 0x11, 
        0xf0, 0x09, 
        0xe8, 
        0xe8, 
        0xe4, 0x03, 
        0xf0, 0x06, 
        0x4c, 0xaa, 0x06, 
        0x4c, 0x35, 0x07, 
        0x60, 
        0xa6, 0x03, 
        0xca, 
        0x8a, 
        0xb5, 0x10, 
        0x95, 0x12, 
        0xca, 
        0x10, 0xf9, 
        0xa5, 0x02,
        0x4a, 
        0xb0, 0x09, 
        0x4a, 
        0xb0, 0x19, 
        0x4a, 
        0xb0, 0x1f, 
        0x4a, 
        0xb0, 0x2f, 
        0xa5, 0x10, 
        0x38, 
        0xe9, 0x20, 
        0x85, 0x10, 
        0x90, 0x01, 
        0x60, 
        0xc6, 0x11, 
        0xa9, 0x01, 
        0xc5, 0x11, 
        0xf0, 0x28, 
        0x60, 
        0xe6, 0x10, 
        0xa9, 0x1f, 
        0x24, 0x10, 
        0xf0, 0x1f, 
        0x60, 
        0xa5, 0x10, 
        0x18, 
        0x69, 0x20,
        0x85, 0x10, 
        0xb0, 0x01, 
        0x60, 
        0xe6, 0x11, 
        0xa9, 0x06, 
        0xc5, 0x11, 
        0xf0, 0x0c, 
        0x60, 
        0xc6, 0x10, 
        0xa5, 0x10, 
        0x29, 0x1f, 
        0xc9, 0x1f, 
        0xf0, 0x01, 
        0x60, 
        0x4c, 0x35, 0x07, 
        0xa0, 0x00, 
        0xa5, 0xfe, 
        0x91, 0x00, 
        0x60,
        0xa6, 0x03, 
        0xa9, 0x00, 
        0x81, 0x10, 
        0xa2, 0x00, 
        0xa9, 0x01, 
        0x81, 0x10, 
        0x60, 
        0xa2, 0x00, 
        0xea,
        0xea, 
        0xca, 
        0xd0, 0xfb, 
        0x60
];
}
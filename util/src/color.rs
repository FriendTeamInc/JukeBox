pub fn hsv2rgb(hue: f32, sat: f32, val: f32) -> (u8, u8, u8) {
    let c = val * sat;
    let v = (hue / 60.0) % 2.0 - 1.0;
    let v = if v < 0.0 { -v } else { v };
    let x = c * (1.0 - v);
    let m = val - c;
    let (r, g, b) = if hue < 60.0 {
        (c, x, 0.0)
    } else if hue < 120.0 {
        (x, c, 0.0)
    } else if hue < 180.0 {
        (0.0, c, x)
    } else if hue < 240.0 {
        (0.0, x, c)
    } else if hue < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let r = ((r + m) * 255.0) as u8;
    let g = ((g + m) * 255.0) as u8;
    let b = ((b + m) * 255.0) as u8;

    (r, g, b)
}

pub const fn rgb888_to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    let r = ((r as u16) & 0b11111000) << 8;
    let g = ((g as u16) & 0b11111100) << 3;
    let b = (b as u16) >> 3;
    r | g | b
}

pub const fn rgb565_to_rgb888(c: u16) -> (u8, u8, u8) {
    let r = ((c & 0b11111_000000_00000) >> 11) as u8;
    let g = ((c & 0b00000_111111_00000) >> 5) as u8;
    let b = ((c & 0b00000_000000_11111) >> 0) as u8;
    (r, g, b)
}

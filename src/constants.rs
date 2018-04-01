pub const F2TO32: f64 = 4294967296.0;

pub const DISPLAY_LOC: i64 = 1024 * 1024 * 256; // spec says "location 256 mbyte"
pub const DISPLAY_DAT: i64 = 0x13FFEF80; // display data location
// 8 bytes width
// 8 bytes height
// 8 bytes color depth
// 8 bytes direct write flags (1 = dont refresh every cycle, 2 = direct write (when in 32bit depth))
// 8 bytes frame count, for syncing
// 8 bytes refresh rate
// 8 bytes screen off (1 = off, 0 = on)

pub const IO_MOUSE: i64 = 0x13FFF7A0; // mouse data beginning
// 8 bytes left click in kg (???), 2^32 is 1 kg
// 8 bytes right click in kg (???), 2^32 is 1 kg
// 8 bytes middle click in kg (???), 2^32 is 1 kg
// 8 bytes relative x movement, 2^32 is 1px
// 8 bytes relative y movement, 2^32 is 1px
// 8 bytes absolute x from top left to bottom right (max is 2^32)
// 8 bytes absolute y from top left to bottom right (max is 2^32)
// 8 bytes power of touch, 2^32 is 1kg
// 8 bytes scroll wheel, -1 is up and 1 is down
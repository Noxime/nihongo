#![allow(dead_code)]
pub const F2TO32: f64 = 4294967296.0;
pub const CPU_STR: &str = "NihongoVM";
pub const CODE_MIL_INS: usize = 1;
pub const CORE_CHECK_RATE: usize = 1_000;
pub const CORE_SLEEP_TIME: u32 = 1;

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

pub const IO_KEYBOARD: i64 = 0x13FFF7F0;
// 8 byte unicode

pub const CPU_0_FLAGS: i64 = 0x13EE0000 + 0;
pub const CPU_0_PC: i64    = 0x13EE0000 + 8;
// 8 bytes cpu state, 0 no cpu, 1 running 2 stop requested 4 stopped
// 8 bytes program counter
pub const CPU_1_FLAGS: i64 = 0x13EE0000 + 16;
pub const CPU_1_PC: i64    = 0x13EE0000 + 24;
pub const CPU_2_FLAGS: i64 = 0x13EE0000 + 32;
pub const CPU_2_PC: i64    = 0x13EE0000 + 40;
pub const CPU_3_FLAGS: i64 = 0x13EE0000 + 48;
pub const CPU_3_PC: i64    = 0x13EE0000 + 56;

// CPU States
pub const CPU_NOT_PRESENT: i64    = 0;
pub const CPU_RUNNING: i64        = 1;
pub const CPU_STOP_REQUESTED: i64 = 2;
pub const CPU_STOPPED: i64        = 4;
pub const CPU_SHUTDOWN: i64       = 8;
pub const CPU_RESET: i64          = 16;
pub const CPU_SUSPEND_RAM: i64    = 32;

pub const CPU_SPINUP_CYCLES: i64 = 0x13EDFFF8;
pub const CPU_VENDOR_INFO: i64   = 0x13FE0028;
// 40 byte ascii string containing CPU info

pub const TIMER: i64 = 0x13FFFFF0;
// NOTE: Unsigned

pub const DISK: i64 = 0x13EDF360;
pub const DISK_DATA_OFFSET: i64 = 0;
pub const DISK_ADDR_OFFSET: i64 = 8;
pub const DISK_CMND_OFFSET: i64 = 16;
pub const DISK_STRIDE: i64 = 24;
// max 100 disk devices
// 8 bytes data, only lowest byte functional
// 8 bytes disk address
// 8 bytes command (0 not present 1 initial value 2 read disk 4 write disk 3 finished reading 5 finished writing)
pub const DISK_NO: i64 = 0;
pub const DISK_INIT: i64 = 1;
pub const DISK_READ: i64 = 2;
pub const DISK_WRITE: i64 = 4;
pub const DISK_READ_DONE: i64 = 3;
pub const DISK_WRITE_DONE: i64 = 5;
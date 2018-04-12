use sdl2::mouse::MouseState;

use constants::*;
use *;

use std::sync::Mutex;

use std::fs::{File, OpenOptions};
use std::io::Write;

#[derive(Clone, Copy)]
pub enum MousePress {
    Up(i64),
    Down(i64),
}

lazy_static! {
    static ref MOUSE_QUEUE: Mutex<Vec<MousePress>> = Mutex::new(vec![]);
    static ref KEY_QUEUE: Mutex<Vec<i64>> = Mutex::new(vec![]);
    static ref DISKS: Mutex<Vec<(File, Vec<u8>)>> = Mutex::new(vec![]);
}

pub fn queue_mouse_press(which: MousePress) {
    MOUSE_QUEUE.lock().unwrap().push(which)
}

pub fn queue_key(c: i64) {
    KEY_QUEUE.lock().unwrap().push(c);
}

// take a SDL keycode and translate it to Dawn keycodes
pub fn queue_keycode(which: Keycode) {
    let v = match which {
        Keycode::LGui | Keycode::RGui => 1,
        Keycode::Home => 2,
        Keycode::End => 3,
        //
        Keycode::Backspace => 8,
        Keycode::PageUp => 9,
        Keycode::PageDown => 10,
        //
        Keycode::Return => 13,
        Keycode::Up => 14,
        Keycode::Left => 15,
        Keycode::Down => 16,
        Keycode::Right => 17,
        //
        Keycode::Escape => 27,
        //
        Keycode::PrintScreen => 192,
        //
        _ => return
    };
    queue_key(v);
}

pub fn work_key_queue(mem: &mut Vec<u8>) {
    let key = {
        let mut queue = KEY_QUEUE.lock().unwrap();
        if queue.len() == 0 { return; }
        let x = queue[0];
        queue.remove(0);
        x
    };
    write(mem, key as i64, IO_KEYBOARD);
}

pub fn work_mouse_queue(mem: &mut Vec<u8>) {
    let press = {
        let mut queue = MOUSE_QUEUE.lock().unwrap();
        if queue.len() == 0 { return; }
        let x = queue[0];
        queue.remove(0);
        x
    };
    match press {
        MousePress::Up(v) => write(mem, 0, IO_MOUSE + v * 8),
        MousePress::Down(v) => write(mem, 1, IO_MOUSE + v * 8),
    }
    //write(mem, match press { MousePress::Up(_) => 1, _ => 0 }, IO_MOUSE + press.0 * 8);
}

pub fn update_mouse_pos(mem: &mut Vec<u8>, x: i32, y: i32, w: i32, h: i32) {
    write(mem, (x as f64 / w as f64 * F2TO32) as i64, IO_MOUSE + 48);
    write(mem, (y as f64 / h as f64 * F2TO32) as i64, IO_MOUSE + 56);
}

pub fn write_mouse(mem: &mut Vec<u8>, mouse: MouseState, width: usize, height: usize) {
    write(mem, if mouse.left() { 1 } else { 0 }, IO_MOUSE + 0);
    write(mem, if mouse.right() { 1 } else { 0 }, IO_MOUSE + 8);
    write(mem, if mouse.middle() { 1 } else { 0 }, IO_MOUSE + 16);

    write(mem, 0, IO_MOUSE + 24);
    write(mem, 0, IO_MOUSE + 32);

    //println!("{}\n{}", mouse.x(), (mouse.x() as f64 / width as f64 * F2TO32) as i64);
    // println!("[{}, {}]", mouse.x() as f64 / width as f64, mouse.y() as f64 / height as f64);
    write(mem, (mouse.x() as f64 / width as f64 * F2TO32) as i64, IO_MOUSE + 48);
    write(mem, (mouse.y() as f64 / height as f64 * F2TO32) as i64, IO_MOUSE + 56);
}

pub fn init_disks(mem: &mut Vec<u8>, files: Vec<String>) -> Result<(), ()> {
    let mut disks = DISKS.lock().unwrap();
    for (i, filename) in files.iter().enumerate() {
        println!("Opening disk image: `{}`", filename);
        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(filename).unwrap();
        let mut bin = vec![];
        f.read_to_end(&mut bin).expect("File read failed");
        println!("Read {} bytes of disk {}", bin.len(), filename);
        disks.push((f, bin));
        write(mem, DISK_INIT, DISK + DISK_CMND_OFFSET + i as i64 * DISK_STRIDE);
    }
    println!("Opened {} disks: {:#?}", disks.len(), disks);
    Ok(())
}

pub fn work_disk(mem: &mut Vec<u8>) {
    let mut disks = DISKS.lock().unwrap();
    for (i, (_, ref mut buf)) in disks.iter().enumerate() {
        //buf[0] = 0xD0;
        let state = read(mem, DISK + DISK_CMND_OFFSET + i as i64 * DISK_STRIDE);
        match state {
            DISK_READ => {
                let addr = read(mem, DISK + DISK_ADDR_OFFSET + i as i64 * DISK_STRIDE) & 0xFFF;
                if let Some(value) = buf.get(addr as usize) {
                    mem[(DISK + DISK_DATA_OFFSET + i as i64 * DISK_STRIDE) as usize] = *value;
                    println!("Read {:#X} from {:#X} (disk {})", *value, addr, i);
                } else {
                    mem[(DISK + DISK_DATA_OFFSET + i as i64 * DISK_STRIDE) as usize] = 0xFF; // Read error, not in range
                    println!("Out of bounds read! ({:#X}, disk size is {:#X}) (disk {})", addr, buf.len(), i);
                }
                write(mem, DISK_READ_DONE, DISK + DISK_CMND_OFFSET + i as i64 * DISK_STRIDE);
            },
            DISK_WRITE => {
                let addr = read(mem, DISK + DISK_ADDR_OFFSET + i as i64 * DISK_STRIDE) & 0xFFF;
                if let Some(_) = buf.get(addr as usize) {
                    buf[addr as usize] = mem[(DISK + DISK_DATA_OFFSET + i as i64 * DISK_STRIDE) as usize];
                    println!("Wrote {:#X} to {:#X} (disk {})", mem[(DISK + DISK_DATA_OFFSET + i as i64 * DISK_STRIDE) as usize], addr, i);
                    println!("MEM DUMP");
                    println!("{:?}", &mem[DISK as usize .. DISK as usize + 24]);
                } else {
                    println!("Out of bounds write! ({:#X}, disk size is {:#X}) (disk {})", addr, buf.len(), i);
                }
                write(mem, DISK_WRITE_DONE, DISK + DISK_CMND_OFFSET + i as i64 * DISK_STRIDE);
            },
            _ => ()
        }
    }
}

pub fn save_disks() {
    println!("Saving disks");
    let mut disks = DISKS.lock().unwrap();
    for (ref mut file, buf) in disks.iter_mut() {
        file.write_all(&buf[..]).expect("Disk write failed");
        file.flush().expect("Flush failed");
    }
    println!("Disk dump done");
}
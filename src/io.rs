use sdl2::mouse::MouseState;

use constants::*;
use *;

use std::sync::Mutex;

#[derive(Clone, Copy)]
pub enum MousePress {
    Up(i64),
    Down(i64),
}

lazy_static! {
    static ref MOUSE_QUEUE: Mutex<Vec<MousePress>> = Mutex::new(vec![]);
}

pub fn queue_mouse_press(which: MousePress) {
    MOUSE_QUEUE.lock().unwrap().push(which)
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
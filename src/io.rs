use sdl2::mouse::MouseState;

use constants::*;
use *;

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
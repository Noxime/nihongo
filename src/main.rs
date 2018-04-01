#![feature(core_intrinsics)]

extern crate byteorder;
extern crate sdl2;

use byteorder::{BigEndian, ByteOrder};
use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use std::env;
use std::fs::File;
use std::io::Read;

use std::time::Instant;
use std::intrinsics::bswap;

mod constants;
mod display;
mod io;

use constants::*;

fn load_bin(filename: String) -> Vec<u8> {
    let mut file = match File::open(&filename) {
        Ok(v) => v,
        Err(why) => { panic!("Failed to open file: {}", why); }
    };
    let mut bin: Vec<u8> = vec![];
    match file.read_to_end(&mut bin) {
        Err(why) => { panic!("Reading file bytes failed: {}", why); }
        _ => ()
    };
    bin
}

fn read(mem: &Vec<u8>, address: i64) -> i64 {
    use std::mem::transmute;
    // let big = unsafe {
    //     transmute::<[u8; 8], i64>(
    //         mem[address as usize .. address as usize + 8]
    //     )
    // };
    // let e = &mem[address as usize];
    
    let base = (&mem[..]).as_ptr();
    let addr = unsafe { base.offset(address as isize) };
    unsafe { bswap(unsafe { *(addr as *const i64) }) }

    // println!("Base: {:?}", base);
    // println!("Addr: {:?}", addr);
    // println!("Valu: {:?}", val);

    // let e = unsafe { (mem as *const _).offset(address as isize * 8) };
    // let big = unsafe { *(e as *const i64) };

    // println!("{:p}", mem);
    // println!("{:?}", e);
    // let big = unsafe {
        // transmute::<[u8], i64>(
            // mem[address as usize]
        // )
    // };
    // unsafe { bswap(big) }
}

fn write(mem: &mut Vec<u8>, s: i64, address: i64) {
    use std::mem::transmute;
    let s = unsafe { bswap(s) };
    let big = unsafe { &mut transmute::<i64, [u8; 8]>(s) };
    &mem[address as usize .. address as usize + 8].clone_from_slice(big);
}

fn main() {
    println!("Starting Nihongo, DAWN system emulator");

    let filename = env::args().skip(1).next().unwrap_or("disk0.bin".to_string());
    println!("Loading file: {}", &filename);
    let bin = &mut load_bin(filename);

    println!("Binary file loaded ({} bytes)", bin.len());

    // defaults
    let width = 1024;
    let height = 512;

    let context = sdl2::init().unwrap();
    let video = context.video().unwrap();
    let window = video.window("Nihongo", width, height)
        .position_centered()
        .build().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let tex_creator = canvas.texture_creator();
    let mut tex = tex_creator.create_texture_streaming(
        Some(PixelFormatEnum::RGB24), // pixel format
        width, // dimens
        height
    ).unwrap();
    canvas.clear();
    canvas.present();
    let mut pump = context.event_pump().unwrap();

    // vm variables
    let mut pc = 0i64;
    let mut iter = 0usize;
    let mut last_frame = 0i64;
    let start = Instant::now();
    println!("VM state initialized, launching");

    // program loop
    'main: loop {
        // program counter cannot be negative, if so we have to halt
        if pc < 0 {
            break;
        }

        let a_addr = read(bin, pc + 0);
        let b_addr = read(bin, pc + 8);
        let c_addr = read(bin, pc + 16);
        
        let a = read(bin, a_addr);
        let b = read(bin, b_addr);

        let s = b - a;

        write(bin, s, b_addr);

        if s <= 0 {
            pc = c_addr;
        } else {
            pc += 24;
        }

        // end of actual emulator
        
        iter += 1;
        if iter % 4_000_000 == 0 {
            let time = {
                let e = start.elapsed();
                e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
            };


            let (w, h, d) = display::draw_screen(
                bin, 
                &mut tex, 
                &mut canvas, 
                &mut last_frame
            );
            
            io::write_mouse(bin, pump.mouse_state(), w, h);

            let _  = canvas.window_mut()
                .set_title(&format!("Nihongo {}x{}x{} @ {:.2} mhz", 
                w, h, d, 
                iter as f64 / time / 1_000_000.0
            ));

            for event in pump.poll_iter() {
                match event {
                    Event::Quit {..} |
                    Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'main
                    },
                    _ => {}
                }
            }

            let time2 = {
                let e = start.elapsed();
                e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
            };
        }
    }
    println!("Halted, PC: {:#X}", pc);
}
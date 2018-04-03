#![feature(core_intrinsics)]
#![feature(const_fn)]

#[cfg(feature = "sdl")]
extern crate sdl2;

#[cfg(feature = "sdl")]
use sdl2::pixels::PixelFormatEnum;
#[cfg(feature = "sdl")]
use sdl2::event::Event;
#[cfg(feature = "sdl")]
use sdl2::keyboard::Keycode;

use std::env;
use std::fs::File;
use std::io::Read;
use std::time::Instant;

use std::intrinsics::{bswap, likely, unlikely};

mod constants;
#[cfg(feature = "sdl")]
mod display;
#[cfg(feature = "sdl")]
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

/*
#[inline(always)]
fn read_ptr(base: *const u8, address: i64) -> i64 {
    let addr = unsafe { base.offset(address as isize) as *const i64 };
    unsafe { bswap(*(addr as *const i64) ) }
}

#[inline(always)]
fn write_ptr(base: *mut u8, s: i64, address: i64) {
    use std::ptr::write;
    let addr = unsafe { base.offset(address as isize) };
    unsafe { write(addr as *mut i64, bswap(s)) };
}
*/

#[inline(always)]
fn read(mem: &Vec<u8>, address: i64) -> i64 {
    
    let base = (&mem[..]).as_ptr();
    let addr = unsafe { base.offset(address as isize) as *const i64 };
    unsafe { bswap(*(addr as *const i64) ) }
    /*
    use std::mem::transmute;
    let x: [u8; 8] = [
        mem[address as usize + 0],
        mem[address as usize + 1],
        mem[address as usize + 2],
        mem[address as usize + 3],
        mem[address as usize + 4],
        mem[address as usize + 5],
        mem[address as usize + 6],
        mem[address as usize + 7],
    ];
    unsafe { bswap(transmute(x)) }
    */
}

#[inline(always)]
fn write(mem: &mut Vec<u8>, s: i64, address: i64) {
    use std::ptr::write;
    let base = (&mut mem[..]).as_mut_ptr();
    let addr = unsafe { base.offset(address as isize) };
    unsafe { write(addr as *mut i64, bswap(s)) };
}

fn main() {
    println!("Starting Nihongo, DAWN system emulator");

    let filename = env::args().skip(1).next().unwrap_or("disk0.bin".to_string());
    println!("Loading file: {}", &filename);

    let bin = &mut load_bin(filename);

    println!("Binary file loaded ({} bytes)", bin.len());

    
    // default video
    let width = 1024;
    let height = 512;

    
    #[cfg(feature = "sdl")] let context = sdl2::init().unwrap();
    #[cfg(feature = "sdl")] let video = context.video().unwrap();
    #[cfg(feature = "sdl")] let window = video.window("Nihongo", width, height)
                                .position_centered()
                                .build().unwrap();
    #[cfg(feature = "sdl")] let mut canvas = window.into_canvas().build().unwrap();
    #[cfg(feature = "sdl")] let tex_creator = canvas.texture_creator();
    #[cfg(feature = "sdl")] let mut tex = tex_creator.create_texture_streaming(
                                Some(PixelFormatEnum::RGB24), // pixel format
                                width, // dimens    
                                height).unwrap();
    #[cfg(feature = "sdl")] canvas.clear();
    #[cfg(feature = "sdl")] canvas.present();
    #[cfg(feature = "sdl")] let mut pump = context.event_pump().unwrap();

    let mut iter = 0usize;
    let mut ins = 0usize;
    let mut test = 0usize;
    let mut last_frame = 0i64;

    write(bin, CPU_RUNNING, CPU_0_FLAGS); // Cpu 0 starts off running
    write(bin, CPU_STOPPED, CPU_1_FLAGS); // State is stopped
    write(bin, CPU_NOT_PRESENT, CPU_2_FLAGS); // State is stopped
    write(bin, CPU_NOT_PRESENT, CPU_3_FLAGS); // State is stopped
    write(bin, 0, CPU_0_PC); // start 0
    write(bin, 0, CPU_1_PC); // start 1
    write(bin, 0, CPU_2_PC); // start 2
    write(bin, 0, CPU_3_PC); // start 3

    println!("VM state initialized");
    
    //PROFILER.lock().unwrap().start("./nihongo.profile").expect("Profile failed");
    let start = Instant::now();

    // program loop
    'main: loop {

        //let mut ran_0 = false;
        /*
        let a_addr = read(bin, pc0 + 0);
        let b_addr = read(bin, pc0 + 8);
        let c_addr = read(bin, pc0 + 16);
        let a = read(bin, a_addr);
        let b = read(bin, b_addr);
        let s = b - a;
        write(bin, s, b_addr);
        if s <= 0 {
            pc0 = c_addr;
        } else {
            pc0 += 24;
        }
        ins += 1;
        */

        
        match read(bin, CPU_0_FLAGS) { // CPU_0
            //CPU_NOT_PRESENT => { write(bin, CPU_RUNNING, CPU_0_FLAGS); }, // Ideally this shouldn't happen, but Dawn is buggy?
            CPU_STOP_REQUESTED | // Should we worry about core 0?
            CPU_STOPPED => { write(bin, CPU_RUNNING, CPU_0_FLAGS); },
            CPU_SHUTDOWN => { println!("Graceful shutdown"); break 'main; },
            CPU_RESET => { println!("Performing hard reset, 0x10 written to CPU_0 flags"); unimplemented!() },
            //CPU_RUNNING | 
            flags => {
                let mut pc = read(bin, CPU_0_PC);

                let pc_restore = pc;
                let a_addr = read(bin, pc +  0);
                let b_addr = read(bin, pc +  8);
                pc         = read(bin, pc + 16);

                let a = read(bin, a_addr);
                let b = read(bin, b_addr);

                let s = b - a;
                write(bin, s, b_addr);
                if b_addr == CPU_1_PC {
                    println!("CPU_0 wrote to CPU_1 flags??");
                }

                // wait.. we didn't jump! go back to where we came from and go 24
                if unsafe { unlikely(s > 0) } {
                    pc = pc_restore + 24;
                }
                write(bin, pc, CPU_0_PC);

                ins += 1;
            }
        }
        
        match read(bin, CPU_1_FLAGS) { // CPU_1
            CPU_NOT_PRESENT | // This is disabled, don't run
            CPU_STOPPED => {}, // CPU is asleep, NOP
            CPU_STOP_REQUESTED => { write(bin, CPU_STOPPED, CPU_1_FLAGS); },
            CPU_SHUTDOWN => { println!("CPU_1 is not allowed to shutdown the system; Ignored"); },
            CPU_RESET => { println!("CPU_1 is not allowed to reset the system; Ignored"); },
            //CPU_RUNNING | 
            flags => {
                let mut pc = read(bin, CPU_1_PC);

                let pc_restore = pc;
                let a_addr = read(bin, pc +  0);
                let b_addr = read(bin, pc +  8);
                pc         = read(bin, pc + 16);

                let a = read(bin, a_addr);
                let b = read(bin, b_addr);

                let s = b - a;
                write(bin, s, b_addr);

                // wait.. we didn't jump! go back to where we came from and go 24
                if unsafe { unlikely(s > 0) } {
                    pc = pc_restore + 24;
                }

                ins += 1;
            }
        }
        
        // write(bin, pc0, CPU_0_PC);
        // write(bin, pc1, CPU_1_PC);
        

        /*

        match read(bin, CPU_1_FLAGS) { // CPU_1
            CPU_RUNNING => {
                let mut pc = pc1;
                let lpc = read(bin, CPU_1_PC);

                if !ran_0 {
                    println!("CPU_1 ran without CPU_0?");
                }

                if pc != lpc {
                    println!("CPU_1 PC was off sync! PC: {:#X}, Mem: {:#X}", pc, lpc);
                    pc = lpc;
                }

                let pc_restore = pc2;
                let a_addr = read(bin, pc +  0);
                let b_addr = read(bin, pc +  8);
                pc         = read(bin, pc + 16);

                let a = read(bin, a_addr);
                let b = read(bin, b_addr);

                let s = b - a;
                write(bin, s, b_addr);        

                // wait.. we didn't jump! go back to where we came from and go 24
                if unsafe { unlikely(s > 0) } {
                    pc = pc_restore + 24;
                }

                write(bin, pc, CPU_1_PC);
                pc1 = pc;

                ins += 1;
            },
            CPU_NOT_PRESENT |
            CPU_STOPPED
            => {},
            CPU_STOP_REQUESTED => write(bin, CPU_STOPPED, CPU_1_FLAGS),
            v => { println!("CPU_1 state: {}", v); } // Auxiliary cores don't care about shutdown or reset
        }
        */

        // end of actual emulator
        
        iter += 1;

        if unsafe { unlikely(iter == 8_000_000_000) } {
            break 'main;
        }

        #[cfg(not(feature = "sdl"))] {
            if unsafe { unlikely(iter % 32_000_000 == 0) } {
                let time = {
                    let e = start.elapsed();
                    e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
                };
                println!("{:.2} MIPS, {:.2} CLCS",
                ins as f64 / time / 1_000_000.0,
                iter as f64 / time / 1_000_000.0
                );
            }
        }

        #[cfg(feature = "sdl")]
        {
        if unsafe { unlikely(iter % 4_000_000 == 0) } {
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
                .set_title(&format!("Nihongo {}x{}x{} @ {:.2} MIPS, Ins/Clc: {:.3}", 
                w, h, d, 
                ins as f64 / time / 1_000_000.0,
                ins as f64 / iter as f64,
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
        

    }

    let finish = {
        let e = start.elapsed();
        e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
    };

    //PROFILER.lock().unwrap().stop().expect("Can't stop profiler");

    println!("Halted CPU_0, PC: {:#X}", read(bin, CPU_0_PC));
    println!("Halted CPU_1, PC: {:#X}", read(bin, CPU_1_PC));
    println!("Halted CPU_2, PC: {:#X}", read(bin, CPU_2_PC));
    println!("Halted CPU_3, PC: {:#X}", read(bin, CPU_3_PC));
    println!("Runtime: {:.2}s, instructions: {} million, cycles {} million", finish, ins / 1_000_000, iter / 1_000_000);
    println!("Average clcs: {:.2}", iter as f64 / finish / 1_000_000.0);
    println!("Average MIPS: {:.2}", ins as f64 / finish / 1_000_000.0);
}

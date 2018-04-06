#![feature(core_intrinsics)]
#![feature(const_fn)]

extern crate sdl2;
#[cfg(feature = "detect_cpu")]
extern crate raw_cpuid;
extern crate argparse;
#[macro_use]
extern crate lazy_static;


use sdl2::pixels::PixelFormatEnum;

use sdl2::event::Event;

use sdl2::keyboard::Keycode;

use argparse::{ArgumentParser, StoreTrue, Store};

use std::env;
use std::fs::File;
use std::io::Read;
use std::time::Instant;

use std::thread;
use std::sync::{Mutex, Arc, mpsc};

use std::intrinsics::{bswap, likely, unlikely};
use std::mem::transmute;

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

#[inline(always)]
fn read_ptr(base: *const u8, address: i64) -> i64 {
    let addr = unsafe { base.offset(address as isize) as *const i64 };
    unsafe { bswap(*(addr as *const i64) ) }
}
#[inline(always)]
fn write_ptr(base: *mut u8, s: i64, address: i64) {
    use std::ptr::write;
    //let base = (&mut mem[..]).as_mut_ptr();
    let addr = unsafe { base.offset(address as isize) };
    unsafe { write(addr as *mut i64, bswap(s)) };
}

#[inline(always)]
fn read(mem: &Vec<u8>, address: i64) -> i64 {
    let base = (&mem[..]).as_ptr();
    let addr = unsafe { base.offset(address as isize) as *const i64 };
    unsafe { bswap(*(addr as *const i64) ) }
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

    // argument parsing
    let mut filename = "disk0.bin".to_string();
    let mut option_cores = 4;
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Nihongo-VM is a DawnOS compatible SUBLEQ emulator.");
        ap.refer(&mut filename)
            .add_option(&["-f", "--file"], Store, "Binary file to load");
        ap.refer(&mut option_cores)
            .add_option(&["-c", "--cores"], Store, "How many cores to run (max 4)");
        ap.parse_args_or_exit();
    }

    println!("Loading file: {}", &filename);

    //let raw_bin = &mut load_bin(filename);
    //let bin = Arc::new(raw_bin);
    let bin = &mut load_bin(filename);

    println!("Binary file loaded ({} bytes)", bin.len());

    
    // default video
    let width = 1024;
    let height = 512;

    let context = sdl2::init().unwrap();
    //let (win, tex) = display::init(&context);
    let video = context.video().unwrap();
    // default DawnOS window size
    let window = video.window("Nihongo", 1024, 512)
                                .position_centered()
                                .build().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let tex_creator = canvas.texture_creator();
    let mut tex = tex_creator.create_texture_streaming(
                                Some(PixelFormatEnum::RGB24), // pixel format
                                1024, // dimens    
                                512).unwrap();
    canvas.clear();
    canvas.present();
    let mut pump = context.event_pump().unwrap();

    let mut test = 0usize;
    let mut last_frame = 0i64;

    // which cores should be active
    let enable_1 = option_cores > 1;
    let enable_2 = option_cores > 2;
    let enable_3 = option_cores > 3;

    write(bin, CPU_RUNNING, CPU_0_FLAGS); // Cpu 0 starts off running
    write(bin, if enable_1 { CPU_STOPPED } else { CPU_NOT_PRESENT }, CPU_1_FLAGS); // State is stopped
    write(bin, if enable_2 { CPU_STOPPED } else { CPU_NOT_PRESENT }, CPU_2_FLAGS); // State is stopped
    write(bin, if enable_3 { CPU_STOPPED } else { CPU_NOT_PRESENT }, CPU_3_FLAGS); // State is stopped
    write(bin, 0, CPU_0_PC); // start 0
    write(bin, 0, CPU_1_PC); // start 1
    write(bin, 0, CPU_2_PC); // start 2
    write(bin, 0, CPU_3_PC); // start 3

    // write cpu metadata
    //write(bin, 1, CPU_SPINUP_CYCLES); // since we are single threaded, our core spinup is instant // EDIT: NVM

    {
        let mut cpu = CPU_STR.to_string();
        #[cfg(feature = "detect_cpu")]
        {
            let id = raw_cpuid::CpuId::new();
            let v = format!("{}", id.get_vendor_info()
                .map(|v| format!("{}", v))
                .unwrap_or("[UNKNOWN]".to_string()));
            let m = format!("{}", id.get_feature_info()
                .map(|v| format!("{}-{}", v.family_id(), v.model_id()))
                .unwrap_or("[UNKNOWN]".to_string()));

            cpu = format!("N-VM {} {}", v, m);
            println!("Detected CPU: {}", cpu);
        }
        if cpu.bytes().count() > 40 {
            cpu.truncate(40);
            println!("ERROR: CPU identifier string was longer than 40 bytes. Trunacated to \"{}\"", cpu);
        }
        for (i, b) in cpu.bytes().enumerate() {
            bin[CPU_VENDOR_INFO as usize + i] = b;
        }
    }
    
    // This allows us to circumvent Rust's FEARLESS CONCERRUNCY and replace it with our _spooky parallelism_
    let mem_ptr: u64 = unsafe { transmute((&bin[..]).as_ptr()) };
    
    println!("VM state initialized");
    //PROFILER.lock().unwrap().start("./nihongo.profile").expect("Profile failed");
    let start = Instant::now();

    let t1 = thread::spawn(move || {
        let mem: *mut u8 = unsafe { transmute(mem_ptr) };
        let mut pc = read_ptr(mem, CPU_0_PC);
        println!("CPU_0 thread launched");
        loop {
            for _ in 0 .. 512 {

            
            let pc_restore = pc;
            let a_addr = read_ptr(mem, pc +  0);
            let b_addr = read_ptr(mem, pc +  8);
            pc         = read_ptr(mem, pc + 16);
            let a = read_ptr(mem, a_addr);
            let b = read_ptr(mem, b_addr);
            let s = b - a;
            write_ptr(mem, s, b_addr);
            if unsafe { unlikely(s > 0) } {
                pc = pc_restore + 24;
            }
            }
        }
    });
    if enable_1 {
        thread::spawn(move || {
            let mem: *mut u8 = unsafe { transmute(mem_ptr) };
            let mut ins = 0;
            let mut pc = read_ptr(mem, CPU_1_PC);
            let mut state = read_ptr(mem, CPU_1_FLAGS);
            println!("CPU_1 thread launched");
            loop {
                // check for state changes, aka if we should launch our CPU or
                // something
                ins += 1;
                if unsafe { unlikely(ins % CORE_CHECK_RATE == 0) } {
                    let old_state = state;
                    state = read_ptr(mem, CPU_1_FLAGS);

                    // state, change do our thang
                    if unsafe { unlikely(state != old_state) } {
                        match state {
                            CPU_RUNNING => {
                                pc = read_ptr(mem, CPU_1_PC);
                            },
                            CPU_STOP_REQUESTED => {
                                state = CPU_STOPPED;
                                write_ptr(mem, state, CPU_1_FLAGS);
                                write_ptr(mem, pc, CPU_1_PC);
                            },
                            _ => {}
                        }
                    }
                }

                if unsafe { likely(state == CPU_STOPPED) } {
                    continue;
                } else if unsafe { unlikely(state == CPU_RUNNING) } {
                    let pc_restore = pc;
                    let a_addr = read_ptr(mem, pc +  0);
                    let b_addr = read_ptr(mem, pc +  8);
                    pc         = read_ptr(mem, pc + 16);
                    let a = read_ptr(mem, a_addr);
                    let b = read_ptr(mem, b_addr);
                    let s = b - a;
                    write_ptr(mem, s, b_addr);
                    if unsafe { unlikely(s > 0) } {
                        pc = pc_restore + 24;
                    }
                }
            }
        });
    }
    if enable_2 {
        thread::spawn(move || {
            let mem: *mut u8 = unsafe { transmute(mem_ptr) };
            let mut ins = 0;
            let mut pc = read_ptr(mem, CPU_2_PC);
            let mut state = read_ptr(mem, CPU_2_FLAGS);
            println!("CPU_2 thread launched");
            loop {
                // check for state changes, aka if we should launch our CPU or
                // something
                ins += 1;
                if unsafe { unlikely(ins % CORE_CHECK_RATE == 0) } {
                    let old_state = state;
                    state = read_ptr(mem, CPU_2_FLAGS);

                    // state, change do our thang
                    if unsafe { unlikely(state != old_state) } {
                        match state {
                            CPU_RUNNING => {
                                pc = read_ptr(mem, CPU_2_PC);
                            },
                            CPU_STOP_REQUESTED => {
                                state = CPU_STOPPED;
                                write_ptr(mem, state, CPU_2_FLAGS);
                                write_ptr(mem, pc, CPU_2_PC);
                            }
                            _ => {}
                        }
                    }
                }

                if unsafe { likely(state == CPU_STOPPED) } {
                    continue;
                } else if unsafe { unlikely(state == CPU_RUNNING) } {
                    let pc_restore = pc;
                    let a_addr = read_ptr(mem, pc +  0);
                    let b_addr = read_ptr(mem, pc +  8);
                    pc         = read_ptr(mem, pc + 16);
                    let a = read_ptr(mem, a_addr);
                    let b = read_ptr(mem, b_addr);
                    let s = b - a;
                    write_ptr(mem, s, b_addr);
                    if unsafe { unlikely(s > 0) } {
                        pc = pc_restore + 24;
                    }
                }
            }
        });
    }
    if enable_3 {
        thread::spawn(move || {
            let mem: *mut u8 = unsafe { transmute(mem_ptr) };
            let mut ins = 0;
            let mut pc = read_ptr(mem, CPU_3_PC);
            let mut state = read_ptr(mem, CPU_3_FLAGS);
            println!("CPU_3 thread launched");
            loop {
                // check for state changes, aka if we should launch our CPU or
                // something
                ins += 1;
                if unsafe { unlikely(ins % CORE_CHECK_RATE == 0) } {
                    let old_state = state;
                    state = read_ptr(mem, CPU_3_FLAGS);

                    // state, change do our thang
                    if unsafe { unlikely(state != old_state) } {
                        match state {
                            CPU_RUNNING => {
                                pc = read_ptr(mem, CPU_3_PC);
                            },
                            CPU_STOP_REQUESTED => {
                                state = CPU_STOPPED;
                                write_ptr(mem, state, CPU_3_FLAGS);
                                write_ptr(mem, pc, CPU_3_PC);
                            }
                            _ => {}
                        }
                    }
                }

                if unsafe { likely(state == CPU_STOPPED) } {
                    continue;
                } else if unsafe { unlikely(state == CPU_RUNNING) } {
                    let pc_restore = pc;
                    let a_addr = read_ptr(mem, pc +  0);
                    let b_addr = read_ptr(mem, pc +  8);
                    pc         = read_ptr(mem, pc + 16);
                    let a = read_ptr(mem, a_addr);
                    let b = read_ptr(mem, b_addr);
                    let s = b - a;
                    write_ptr(mem, s, b_addr);
                    if unsafe { unlikely(s > 0) } {
                        pc = pc_restore + 24;
                    }
                }
            }
        });
    }
    
    let mut ins_count = 0usize;
    let mut last_time = 0.0;
    let mut peak_mips = 0.0;
    // program loop
    'update: loop {
        let time = {
            let e = start.elapsed();
            e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
        };

        // Timer
        write(bin, unsafe { transmute((time * F2TO32) as u64) }, TIMER);

        for event in pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'update;
                },
                Event::MouseMotion { x, y, .. } => {
                    let (w, h) = display::get_dimens(bin);
                    io::update_mouse_pos(bin, x, y, w, h);
                },
                Event::MouseButtonDown { which, .. } => {
                    io::queue_mouse_press(io::MousePress::Down(which as i64));
                },
                Event::MouseButtonUp { which, .. } => {
                    io::queue_mouse_press(io::MousePress::Up(which as i64));
                },
                Event::Window { .. } => {},
                v => { println!("Event: {:#?}", v)}
            }
        }

        display::refresh(bin, &mut tex, &mut canvas);
        io::work_mouse_queue(bin);

        // we want to update our timer every 2ms
        thread::sleep_ms(2);
    }
    /*
    'main: loop {
        
        /*
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

        
        {
            let time = {
                let e = start.elapsed();
                e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
            };
            let delta = time - last_time;
            last_time = time;

            let (w, h, d) = display::draw_screen(
                bin, 
                &mut tex, 
                &mut canvas, 
                &mut last_frame
            );
            /*
            let last_ins_count = ins_count;
            ins_count += rx.try_iter().fold(0, |s, v| if v == CODE_MIL_INS { s + 1 } else { s });
            let mips = (ins_count - last_ins_count) as f64 / delta * INS_REPORT_RATE as f64 / 1_000_000.0;
            if mips > peak_mips {
                peak_mips = mips;
            }*/

            io::write_mouse(bin, pump.mouse_state(), w, h);

            let _  = canvas.window_mut()
                .set_title(
                    //&format!("Nihongo {}x{}x{} @ {:.2} MIPS (peak: {:.2}) Cores: {}{}{}{}", 
                    &format!("Nihongo {}x{}x{} Cores: {}{}{}{}", 
                        w, h, d, 
                        /*mips,
                        peak_mips,
                        */
                        //match read(bin, CPU_0_FLAGS) { CPU_NOT_PRESENT => 'N', CPU_RUNNING => 'R', CPU_STOPPED => 'S', _ => 'O'},
                        'R', // CPU_0 runs always
                        match read(bin, CPU_1_FLAGS) { CPU_NOT_PRESENT => 'N', CPU_RUNNING => 'R', CPU_STOPPED => 'S', _ => 'O'},
                        match read(bin, CPU_2_FLAGS) { CPU_NOT_PRESENT => 'N', CPU_RUNNING => 'R', CPU_STOPPED => 'S', _ => 'O'},
                        match read(bin, CPU_3_FLAGS) { CPU_NOT_PRESENT => 'N', CPU_RUNNING => 'R', CPU_STOPPED => 'S', _ => 'O'},
            ));

            for event in pump.poll_iter() {
                match event {
                    Event::Quit {..} |
                    Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'main
                    },
                    v => { println!("Event: {:#?}", v)}
                }
            }

            // update timer
            write(bin, unsafe { transmute((time * F2TO32) as u64) }, TIMER);
        }
        
        thread::sleep_ms(250);
        */
    }
    */

    let finish = {
        let e = start.elapsed();
        e.as_secs() as f64 + e.subsec_nanos() as f64 / 1_000_000_000.0
    };

    println!();
    //PROFILER.lock().unwrap().stop().expect("Can't stop profiler");
    //println!("Nihongo-VM exited with code: {}", code);
    println!("Halted CPU_0, PC: {:#X}", read(bin, CPU_0_PC));
    println!("Halted CPU_1, PC: {:#X}", read(bin, CPU_1_PC));
    println!("Halted CPU_2, PC: {:#X}", read(bin, CPU_2_PC));
    println!("Halted CPU_3, PC: {:#X}", read(bin, CPU_3_PC));
    println!("Runtime: {:.2}s, instructions: {} million", finish, 1 / 1_000_000);
    println!("Average MIPS: {:.2}", 1 as f64 / finish / 1_000_000.0);
}

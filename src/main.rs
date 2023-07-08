mod chip8;

extern crate sdl2;

use chip8::display::DisplayBuffer;
use chip8::display::RES_HEIGHT;
use chip8::display::RES_WIDTH;
use chip8::Chip8;
use clap::Parser;
use rodio::OutputStream;
use rodio::Sink;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use std::fs::File;
use std::io::Read;
use std::time::Duration;

const TIMER_SPEED: u32 = 60;

fn map_keycode(key: Keycode) -> Option<u8> {
    match key {
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0xC),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),
        _ => None,
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    rom: String,

    #[arg(long, default_value_t = 1000)]
    speed: u16,

    #[arg(long, default_value_t = 16)]
    scale: u8,
}

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    let source = rodio::source::SineWave::new(700.0);
    sink.pause();
    sink.append(source);

    let mut pause_emulation = false;

    let args = Args::parse();

    let window = video_subsystem
        .window(
            "Chip8",
            RES_WIDTH as u32 * args.scale as u32,
            RES_HEIGHT as u32 * args.scale as u32,
        )
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.present();

    let canvas_color = Color::RGB(0, 0, 0);
    let pixel_color = Color::RGB(255, 255, 255);

    let cpu_timer_speed_ratio: u32 = args.speed as u32 / TIMER_SPEED;

    let mut chip8 = Chip8::new();

    let mut file = File::open(args.rom).expect("Unable to open ROM file!");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    chip8.load(&buffer);

    let mut cycle_n: u64 = 0;

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        canvas.set_draw_color(canvas_color);
        canvas.clear();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    ..
                } => {
                    pause_emulation = !pause_emulation;
                }
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if let Some(x) = map_keycode(key) {
                        chip8.key_pressed(x, true);
                    }
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    if let Some(x) = map_keycode(key) {
                        chip8.key_pressed(x, false);
                    }
                }
                _ => {}
            }
        }

        draw_canvas(
            &mut canvas,
            chip8.display.as_buffer(),
            pixel_color,
            args.scale as u32,
        );

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / args.speed as u32));

        if pause_emulation {
            sink.pause();
            continue;
        }

        chip8.fetch_execute();
        if chip8.beep() {
            sink.play();
        } else {
            sink.pause();
        }

        if cycle_n % cpu_timer_speed_ratio as u64 == 0 {
            chip8.dec_timers();
        }

        cycle_n += 1;
    }
}

fn draw_canvas(canvas: &mut WindowCanvas, buffer: DisplayBuffer, color: Color, scale: u32) {
    for (index, item) in buffer.iter().enumerate() {
        if item == &true {
            let x: i32 = i32::try_from(index % RES_WIDTH).unwrap() * scale as i32;
            let y: i32 = i32::try_from(index / RES_WIDTH).unwrap() * scale as i32;
            let rectangle = Rect::new(x, y, scale, scale);
            canvas.set_draw_color(color);
            canvas.fill_rect(rectangle).unwrap();
        }
    }
}

/************************************** Linking External Modules **************************************/
mod cpu;
mod ppu;
mod display;
mod input;
mod bus;
mod rom;
mod shader;
mod shader_program;
mod vertex_array;
mod buffer;
mod renderer;
mod texture;
use crate::renderer::Renderer;
use glium::glutin::event::KeyboardInput;
use glium::glutin::event::{Event, WindowEvent, DeviceEvent, ElementState};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::{WindowBuilder, Fullscreen};
use glium::glutin::{Api, ContextBuilder, GlRequest};
use glium::glutin::dpi::{Size, PhysicalSize};
use rfd::FileDialog;
use futures::executor::block_on;
use std::time::{Duration, Instant};
use std::fs::File;
use std::path::PathBuf;
use std::io::Read;
use std::sync::mpsc;
use std::thread;
use cpu::*;
use bus::Bus;
use rom::Rom;
/******************************************************************************************************/

/***************** Display Dimensions *****************/
const WINDOW_WIDTH: u32 = 1024;
const WINDOW_HEIGHT: u32 = 960;
const SCREEN_WIDTH: u32 = 256;
const SCREEN_HEIGHT: u32 = 240;
/******************************************************/

/***** Game Loop Functions *****/

/*
Startup Function:

Runs once during the first frame of execution. Initializes all
emulated hardware components
*/
fn startup() -> CPU {

    //Prompt User to select rom (via file dialog box)
    let mut filename = FileDialog::new()
    .add_filter("NES ROM", &["nes"])
    .pick_file();

    let file = File::open(filename.get_or_insert(PathBuf::new()));
    
    //load ROM
    let mut raw = Vec::new();
    let _ = file.expect("REASON").read_to_end(&mut raw);
    let rom = Rom::new(&raw);

    //generate Memory Bus
    let bus = Bus::new(rom);

    //generate CPU
    let mut cpu_6502 = CPU::new(bus);
    cpu_6502.reset();

    /*
    output CPU as return value. CPU is already connected 
    to all hardware peripherals via memory bus.
    */
    cpu_6502
}

/*
Update Function:

Runs once per frame of app execution. Drives the CPU and PPU
functionality for the duration of a single screen update.
*/

#[inline(always)]
fn update(cpu_6502: &mut CPU, frame: &mut display::Frame){
    *frame = display::Frame::new(display::SYSTEM_PALLETE[cpu_6502.mem_bus.ppu.palette_table[0] as usize]);
    let mut scanline = 0;
    while cpu_6502.mem_bus.ppu.scanlines <= 240 {
        scanline += 8;
        display::render(&cpu_6502.mem_bus.ppu, frame, scanline - 8, scanline);
        while (cpu_6502.mem_bus.ppu.scanlines as usize) < scanline{
            cpu_6502.interpret();
        }
    }

    while cpu_6502.mem_bus.ppu.scanlines > 240 {
        cpu_6502.interpret();
    }
    
}

/*
Compute Thread:

Handles the actual hardware emulation.First initializes the 
emulated CPU, display frame, and controller input, then runs 
the I/O and computation continuously through an application 
loop, passing data to and from the window loop as necessary.

Application Loop Logic -
    Recieve/Handle Input -> Compute Frame -> Send Frame
*/
fn compute_thread(tx: mpsc::SyncSender<[u8; (SCREEN_HEIGHT * SCREEN_WIDTH * 4) as usize]>, rx: mpsc::Receiver<Option<KeyboardInput>>) {
    //Initialize Display Frame, CPU (+ Peripherals), and Input Container
    let mut frame = display::Frame::new((0, 0, 0));
    let mut cpu_6502 = startup();
    tx.send(frame.data).unwrap();
    let mut input_option = rx.recv().unwrap();


    //Application Loop
    while true {
        //Recieve/Parse Input From Window Thread
        input_option = rx.recv().unwrap();
        match input_option{
            //if button is pressed, update emulated controller 
            //state as needed
            Some(input) => {
                cpu_6502.mem_bus.port1.keyboard_input(input);
            },
            //otherwise, do nothing
            _ => ()
        }
        //compute for one frame update
        update(&mut cpu_6502, &mut frame);
        //send frame to window thread
        tx.send(frame.data).unwrap();
    }
}

/*
Window Thread:

Handles the OpenGL Context, Windowing, and Event Handling. First
Initializes the OpenGL Context, Window, and Event Handler, then
runs event handling and screen rendering through an application
loop, passing data to and from the compute thread as necessary.

Application Loop Logic - 
    Calculate Framerate -> Poll User Input -> Send Input to
    Compute Thread -> Recieve Display Frame from Compute Thread
    -> Render Display Frame
*/
fn window_thread(tx: mpsc::SyncSender<Option<KeyboardInput>>, rx: mpsc::Receiver<[u8; (SCREEN_HEIGHT * SCREEN_WIDTH * 4) as usize]>) {
    //Initialize OpenGL Context, Window, and Event Handler
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(
            Size::from(
                PhysicalSize::new(
                    WINDOW_WIDTH, 
                    WINDOW_HEIGHT
                )
            )
        )
        .with_title("NES Emu");

    let gl_context = ContextBuilder::new()
        .with_gl(GlRequest::Specific(Api::OpenGl, (3, 3)))
        .build_windowed(window, &event_loop)
        .expect("Cannot create windowed context");

    let gl_context = unsafe {
        gl_context
            .make_current()
            .expect("Failed to make context current")
    };

    gl::load_with(|ptr| gl_context.get_proc_address(ptr) as *const _);

    //Initialize renderer and framerate calculation variables
    let renderer = Renderer::new().expect("Cannot create renderer");
    let mut now = Instant::now();
    let mut frames = 0;
    let mut frame_time = Instant::now();
    let mut input_option: Option<KeyboardInput> = None;
    
    //Application Loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        //Framerate Limiter
        while frame_time.elapsed() < Duration::new(0,16666666) {}
        frame_time = Instant::now();
    
        //Calculate FPS
        if now.elapsed() < Duration::new(1, 0) {
            frames += 1;
        }
        else {
            println!("Framerate: {} fps", frames);
            frames = 0;
            now = Instant::now();
        }

        //Input Handling
        input_option = None;
        match event {
            Event::LoopDestroyed => (),
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => gl_context.resize(physical_size),
                    WindowEvent::KeyboardInput { device_id, input, .. } => {
                        input_option = Some(input);
                    },
                    _ => (),
                }
            },
            _ => (),
        }

        tx.send(input_option).unwrap();     //Send Input
        renderer.draw(&rx.recv().unwrap()); //Recieve Frame
        gl_context.swap_buffers();          //Update Screen with Current Frame
    });
}

/***** Main Function *****/
fn main() {
    //Initialize Message Passing Channels
    let (frame_send, frame_recv) = mpsc::sync_channel(1);
    let (input_send, input_recv) = mpsc::sync_channel(1);
    //Run Compute on Separate Thread
    thread::spawn( move || {
        compute_thread(frame_send, input_recv);
    });
    //Run Graphics Pipeline on Main Thread (Cannot Run on Sub-thread)
    window_thread(input_send, frame_recv);
}
/*************************/
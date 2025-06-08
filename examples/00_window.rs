use std::time::Duration;

use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::sys::timer::SDL_DelayPrecise;
use sdl3::EventPump;

const WINDOW_TITLE: &str = "Basic Window";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const FRAME_DELAY: Duration = Duration::from_millis(15);

type BoxError = Box<dyn std::error::Error>;

fn main() -> Result<(), BoxError> {
    let sdl = sdl3::init()?;
    let video_subsystem = sdl.video()?;

    let _window = video_subsystem
        .window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
        .resizable()
        .position_centered()
        .build()?;

    let mut app = App { quit: false };

    let mut event_pump = sdl.event_pump()?;
    loop {
        app.handle_events(&mut event_pump);
        if app.quit {
            break;
        }

        unsafe { SDL_DelayPrecise(FRAME_DELAY.as_nanos() as u64) };
    }

    Ok(())
}

struct App {
    quit: bool,
}

impl App {
    fn handle_events(&mut self, event_pump: &mut EventPump) {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    self.quit = true;
                    return;
                }

                _ => {}
            }
        }
    }
}

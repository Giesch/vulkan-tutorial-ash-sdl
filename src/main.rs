use std::time::Duration;

use sdl3::sys::timer::SDL_DelayPrecise;

use ash_sdl_vulkan_tutorial::*;

const WINDOW_TITLE: &str = "Vulkan Tutorial";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const FRAME_DELAY: Duration = Duration::from_millis(15);

fn main() -> Result<(), BoxError> {
    let sdl = sdl3::init()?;
    let video_subsystem = sdl.video()?;

    let window = video_subsystem
        .window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
        .resizable()
        .position_centered()
        .build()?;

    let renderer = Renderer::init(&window)?;
    let mut app = App {
        quit: false,
        renderer,
    };

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

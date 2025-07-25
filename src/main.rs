use std::time::Duration;

use sdl3::sys::timer::SDL_DelayPrecise;

use ash_sdl_vulkan_tutorial::*;

const WINDOW_TITLE: &str = "Vulkan Tutorial";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const FRAME_DELAY: Duration = Duration::from_millis(15);

fn main() -> Result<(), BoxError> {
    pretty_env_logger::init();

    let compiled_shaders = shaders::compile_slang_shaders()?;

    let sdl = sdl3::init()?;
    let video_subsystem = sdl.video()?;

    let window = video_subsystem
        .window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .resizable()
        .vulkan()
        .build()?;

    let renderer = Renderer::init(window, compiled_shaders)?;

    let mut app = App {
        quit: false,
        minimized: false,
        renderer,
    };

    let mut event_pump = sdl.event_pump()?;
    loop {
        let Ok(()) = app.handle_events(&mut event_pump) else {
            break;
        };
        if app.quit {
            break;
        }

        if !app.minimized {
            app.renderer.draw_frame()?;
        }

        unsafe { SDL_DelayPrecise(FRAME_DELAY.as_nanos() as u64) };
    }

    app.renderer.drain_gpu()?;

    Ok(())
}

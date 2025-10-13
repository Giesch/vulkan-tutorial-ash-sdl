pub mod shaders;

mod app;
mod game;
mod renderer;
mod util;

#[cfg(debug_assertions)]
mod shader_watcher;

pub use game::*;

use sdl3::sys::timer::SDL_DelayPrecise;

use app::App;
use renderer::{Renderer, RendererConfig};

pub fn run_game(game: impl Game + 'static) -> anyhow::Result<()> {
    pretty_env_logger::init();

    let sdl = sdl3::init()?;
    let video_subsystem = sdl.video()?;
    let window = video_subsystem
        .window(game.title(), game.window_width(), game.window_height())
        .position_centered()
        .resizable()
        .vulkan()
        .build()?;

    let frame_delay = game.frame_delay().as_nanos() as u64;

    let renderer_config = RendererConfig::from_game(&game)?;
    let renderer = Renderer::init(window, renderer_config)?;
    let mut app = App::new(renderer, Box::new(game));

    let mut event_pump = sdl.event_pump()?;
    loop {
        let Ok(()) = app.handle_events(&mut event_pump) else {
            break;
        };
        if app.quit {
            break;
        }

        if !app.minimized {
            app.game.draw_frame(&mut app.renderer)?;
        }

        unsafe { SDL_DelayPrecise(frame_delay) };
    }

    app.renderer.drain_gpu()?;

    Ok(())
}

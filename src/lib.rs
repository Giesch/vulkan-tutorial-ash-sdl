pub mod shaders;

mod app;
mod game;
mod renderer;
mod util;

#[cfg(debug_assertions)]
mod shader_watcher;

pub use game::*;

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

    let mut app = App::init(window, game)?;
    let event_pump = sdl.event_pump()?;
    app.run_loop(event_pump)
}

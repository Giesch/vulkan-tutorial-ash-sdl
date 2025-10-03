pub mod shaders;

mod app;
mod game;
mod renderer;
mod util;

#[cfg(debug_assertions)]
mod shader_watcher;

pub use app::App;
pub use game::VikingRoom;
pub use renderer::Renderer;

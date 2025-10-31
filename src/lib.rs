mod app;
mod game;
mod generated;
mod renderer;
mod shaders;
mod util;

#[cfg(debug_assertions)]
mod shader_watcher;

pub use game::*;
pub use shaders::build_tasks;

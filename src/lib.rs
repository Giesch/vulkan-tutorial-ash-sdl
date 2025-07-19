mod app;
mod renderer;
#[cfg(debug_assertions)]
mod shader_watcher;
mod shaders;

pub use app::App;
pub use renderer::Renderer;

pub type BoxError = Box<dyn std::error::Error>;

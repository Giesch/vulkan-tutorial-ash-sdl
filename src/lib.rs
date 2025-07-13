mod app;
mod renderer;
mod shaders;

pub use app::App;
pub use renderer::Renderer;

pub type BoxError = Box<dyn std::error::Error>;

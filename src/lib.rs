mod app;
mod renderer;

pub use app::App;
pub use renderer::Renderer;

pub type BoxError = Box<dyn std::error::Error>;

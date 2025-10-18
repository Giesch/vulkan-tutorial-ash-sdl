use std::time::Duration;

use crate::app::App;
use crate::renderer::Renderer;

/// This is the only trait from this module to implement directly.
pub trait Game {
    fn window_description() -> WindowDescription;

    fn setup(renderer: Renderer) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn draw_frame(&mut self) -> anyhow::Result<()>;

    fn on_resize(&mut self) -> anyhow::Result<()>;

    fn frame_delay(&self) -> Duration;

    fn deinit(self: Box<Self>) -> anyhow::Result<()>;

    fn run() -> anyhow::Result<()>
    where
        Self: Sized + 'static,
    {
        pretty_env_logger::init();

        let sdl = sdl3::init()?;
        let video_subsystem = sdl.video()?;
        let window_desc = Self::window_description();
        let window = video_subsystem
            .window(window_desc.title, window_desc.width, window_desc.height)
            .position_centered()
            .resizable()
            .vulkan()
            .build()?;

        let renderer = Renderer::init(window)?;
        let game = Self::setup(renderer)?;
        let app = App::init(game)?;

        let event_pump = sdl.event_pump()?;
        app.run_loop(event_pump)
    }
}

/// parameters passed through to SDL to create a window
pub struct WindowDescription {
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
}

/// methods called during initial window setup
pub trait GameSetup {
    fn window_description() -> WindowDescription;

    fn setup(renderer: Renderer) -> anyhow::Result<Self>
    where
        Self: Sized;
}

/// methods used after initialization
/// this trait needs to be object-safe
pub trait RuntimeGame {
    fn draw_frame(&mut self) -> anyhow::Result<()>;

    fn on_resize(&mut self) -> anyhow::Result<()>;

    fn frame_delay(&self) -> Duration;

    fn deinit(self: Box<Self>) -> anyhow::Result<()>;
}

impl<G> GameSetup for G
where
    G: Game,
{
    fn window_description() -> WindowDescription {
        G::window_description()
    }

    fn setup(renderer: Renderer) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        G::setup(renderer)
    }
}

impl<G> RuntimeGame for G
where
    G: Game,
{
    fn draw_frame(&mut self) -> anyhow::Result<()> {
        self.draw_frame()
    }

    fn on_resize(&mut self) -> anyhow::Result<()> {
        self.on_resize()
    }

    fn frame_delay(&self) -> Duration {
        self.frame_delay()
    }

    fn deinit(self: Box<Self>) -> anyhow::Result<()> {
        self.deinit()
    }
}

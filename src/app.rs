use std::ffi::c_void;

use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::Keycode;
use sdl3::EventPump;

use crate::renderer::Renderer;

pub struct App {
    pub quit: bool,
    pub minimized: bool,
    pub renderer: Renderer,
    pub game: Box<dyn Game>,
}

impl App {
    pub fn new(renderer: Renderer, game: Box<dyn Game>) -> Self {
        Self {
            game,
            renderer,
            quit: false,
            minimized: false,
        }
    }

    // https://wiki.libsdl.org/SDL3/SDL_EventType
    pub fn handle_events(&mut self, event_pump: &mut EventPump) -> Result<(), anyhow::Error> {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    self.quit = true;
                    return Ok(());
                }

                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::Resized(_new_width, _new_height) => {
                        // we take the new dimensions off the renderer's window ref
                        self.renderer.recreate_swapchain()?;
                    }
                    WindowEvent::Minimized => {
                        self.minimized = true;
                    }
                    WindowEvent::Maximized => {
                        self.minimized = false;
                    }
                    WindowEvent::Restored => {
                        self.minimized = false;
                    }

                    WindowEvent::Exposed => {
                        // Window has been exposed and should be redrawn,
                        // and can be redrawn directly from event watchers for this event
                    }
                    WindowEvent::PixelSizeChanged(_, _) => {
                        // vulkan: update display scale
                    }
                    WindowEvent::FocusLost => {
                        // pause in-game?
                    }
                    WindowEvent::DisplayChanged(_) => {
                        // vulkan: update whatever is necessary for new surface
                        // ie, display scale
                    }
                    WindowEvent::Shown => {}
                    WindowEvent::Hidden => {
                        // what do these two mean? minimized to task bar?
                    }
                    WindowEvent::CloseRequested => {
                        // handle same as quit?
                    }

                    WindowEvent::Moved(_, _) => {}
                    WindowEvent::MouseEnter => {}
                    WindowEvent::MouseLeave => {}
                    WindowEvent::FocusGained => {}
                    WindowEvent::HitTest(_, _) => {}
                    WindowEvent::ICCProfChanged => {}

                    WindowEvent::None => {}
                },

                _ => {}
            }
        }

        Ok(())
    }
}

// TODO where should this go? top app level?
pub trait Game {
    // TODO
    // we want a more zoomed-out struct than this
    // like, Game should say,
    // 'I want these GPU resources: a uniform buffer of size x, ...', etc
    // should the game keep the gpu resources in its own struct,
    //   instead of in the renderer?
    // then it passes them to the renderer or something?
    //   or renderer resources go in an ECS table and the game gets handles?
    //   rather than an ecs table, it could be a wrapped vec index kept here
    //
    // could do whatever the crappy version is first, just to get the module out
    fn uniform_buffer_size(&self) -> usize;

    fn update_uniform_buffer(
        &self,
        aspect_ratio: f32,
        mapped_uniform_buffer: *mut c_void,
    ) -> anyhow::Result<()>;
}

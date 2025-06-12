use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::Keycode;
use sdl3::EventPump;

use crate::Renderer;

pub struct App {
    pub quit: bool,
    pub renderer: Renderer,
}

impl App {
    pub fn handle_events(&mut self, event_pump: &mut EventPump) {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    self.quit = true;
                    return;
                }

                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::Exposed => {
                        // Window has been exposed and should be redrawn,
                        // and can be redrawn directly from event watchers for this event
                    }
                    WindowEvent::Resized(_, _) => {
                        // vulkan: invalidate swapchain
                    }
                    WindowEvent::PixelSizeChanged(_, _) => {
                        // vulkan: update display scale
                    }
                    WindowEvent::Minimized => {
                        // pause & sleep?
                    }
                    WindowEvent::Maximized => {
                        // unsleep if previously minimized?
                    }
                    WindowEvent::Restored => {
                        // unsleep if previously minimized?
                    }
                    WindowEvent::FocusLost => {
                        // pause?
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

                    _ => {}
                },

                _ => {}
            }
        }
    }
}

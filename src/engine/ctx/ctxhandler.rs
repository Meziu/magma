// SDL2 imports
use sdl2::event::{Event, WindowEvent};
use sdl2::EventPump;
use sdl2::Sdl;

// imports from the module
use super::audio::AudioHandler;
use super::video::VideoHandler;
use super::FPSHandler;

/// Main handler to manage calls to the SDL2 API
pub struct CtxHandler {
    ctx: Sdl,
    event_pump: EventPump,
    pub video: VideoHandler,
    pub fps_manager: FPSHandler,
    pub audio: AudioHandler,

    must_break: bool,
}

impl CtxHandler {
    /// Generate a new handler with a new context, window, graphics handler, event pump, audio mixer
    pub fn new() -> CtxHandler {
        let ctx = sdl2::init().expect("Couldn't init SDL2 context");

        let event_pump = ctx.event_pump().expect("Couldn't obtain Event Pump from SDL2 context");

        let video = VideoHandler::new(&ctx);
        let audio = AudioHandler::new();

        let fps_manager = FPSHandler::new(60);

        CtxHandler {
            ctx,
            event_pump,
            video,
            fps_manager,
            audio,

            must_break: false,
        }
    }

    /// Check all SDL2 and SDL_Window events
    pub fn check_events(&mut self) {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => self.must_break = true,
                Event::Window { win_event, .. } => {
                    if let WindowEvent::Resized(_, _) = win_event {
                        self.video.set_window_resized(true);
                    }
                }
                _ => {}
            }
        }
    }

    /// Fetch the flag to stop the program
    pub fn get_break_signal(&self) -> bool {
        self.must_break
    }

    /// Public function to set the Ctx's framerate
    pub fn set_framerate(&mut self, new_framerate: u16) {
        self.fps_manager.set_fps(new_framerate);
    }

    /// Get the current framerate
    pub fn get_framerate(&self) -> u16 {
        self.fps_manager.get_fps()
    }

    /// Wait for the next frame based on the current framerate
    pub fn wait(&mut self) {
        self.fps_manager.wait();
    }
}

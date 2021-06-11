// SDL2 imports
use sdl2::event::{Event, WindowEvent};
use sdl2::gfx::framerate::FPSManager;
use sdl2::EventPump;
use sdl2::Sdl;

// imports from the module
use super::audio::AudioHandler;
use super::video::VideoHandler;

/// Main handler to manage calls to the SDL2 API
pub struct CtxHandler {
    ctx: Sdl,
    event_pump: EventPump,
    pub video: VideoHandler,
    pub fps_manager: FPSManager,
    pub audio: AudioHandler,

    must_break: bool,
}

impl CtxHandler {
    /// Generate a new handler with a new context, window, graphics handler, event pump, audio mixer
    pub fn new(framerate: u32) -> CtxHandler {
        let ctx = sdl2::init().expect("Couldn't init SDL2 context");

        let event_pump = ctx.event_pump().expect("Couldn't obtain Event Pump from SDL2 context");

        let video = VideoHandler::new(&ctx, "Rust Testing Grounds");
        let audio = AudioHandler::new();

        let mut fps_manager = FPSManager::new();
        CtxHandler::_set_framerate(&mut fps_manager, framerate);

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
    pub fn set_framerate(&mut self, new_framerate: u32) {
        CtxHandler::_set_framerate(&mut self.fps_manager, new_framerate);
    }

    /// Private function to set framerate using FPSManager (unsafe in a public environment)
    fn _set_framerate(fps_manager: &mut FPSManager, new_framerate: u32) {
        if let Err(e) = fps_manager.set_framerate(new_framerate) {
            eprintln!("Couldn't set framerate to {}: {}", new_framerate, e);
        }
    }

    /// Get the current framerate
    pub fn get_framerate(&self) -> i32 {
        self.fps_manager.get_framerate()
    }

    /// Wait for the next frame based on the current framerate
    pub fn wait(&mut self) {
        self.fps_manager.delay();
    }
}

// SDL2 imports
use sdl2::event::{Event, WindowEvent};
use sdl2::gfx::framerate::FPSManager;
use sdl2::EventPump;
use sdl2::Sdl;

// std imports
use std::error::Error;
use std::fmt::{Display, Formatter, Debug};

// imports from the module
use super::audio::AudioHandler;
use super::video::{VideoHandler, VideoHandlerInitError};

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
    /// Generate a new handler with a new context, window, event pump
    pub fn new() -> Result<CtxHandler, CtxHandlerInitError> {
        let ctx = sdl2::init()?;

        let event_pump = ctx.event_pump()?;

        let video = VideoHandler::new(&ctx, "Rust Testing Grounds")?;
        let audio = AudioHandler::new()?;

        let mut fps_manager = FPSManager::new();
        fps_manager.set_framerate(60u32)?;

        Ok(CtxHandler {
            ctx,
            event_pump,
            video,
            fps_manager,
            audio,

            must_break: false,
        })
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

    pub fn get_break_signal(&self) -> bool {
        self.must_break
    }
}

#[derive(Debug)]
pub enum CtxHandlerInitError {
    ByString(String),
    FromVideoInit(VideoHandlerInitError),
}

impl Display for CtxHandlerInitError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let out = match self {
            Self::ByString(e) => format!("From String: {}", e),
            Self::FromVideoInit(e) => format!("On Video Context Init: {}", e),
        };

        write!(f, "Context Handler Init Error: {}", out)
    }
}

impl From<String> for CtxHandlerInitError {
    fn from(e: String) -> Self {
        Self::ByString(e)
    }
}

impl From<VideoHandlerInitError> for CtxHandlerInitError {
    fn from(e: VideoHandlerInitError) -> Self {
        Self::FromVideoInit(e)
    }
}

impl Error for CtxHandlerInitError {}

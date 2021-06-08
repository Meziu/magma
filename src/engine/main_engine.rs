// standard imports
use std::error::Error;
use std::path::Path;
use std::fmt::{Display, Formatter, Debug};

// import the ctx mdule
use super::ctx::{CtxHandler, CtxHandlerInitError, GraphicsLoopError};

/// Main struct to handle the whole program in all it's components
pub struct Engine {
    ctx_handler: CtxHandler,
}

impl Engine {
    /// Engine init process
    pub fn new() -> Result<Self, EngineInitError> {
        let ctx_handler = CtxHandler::new()?;

        Ok(Self { ctx_handler })
    }

    /// Main function to run the program, returns an error if any panics are necessary
    pub fn run(&mut self) -> Result<(), EngineRuntimeError> {
        let chunk = self
            .ctx_handler
            .audio
            .sfx_from_file(Path::new("assets/example.ogg"))?;
        self.ctx_handler.audio.sfx_play(&chunk)?;

        'mainloop: loop {
            self.ctx_handler.check_events();
            if self.ctx_handler.get_break_signal() {
                break 'mainloop;
            }

            self.ctx_handler.video.update()?;

            self.ctx_handler.fps_manager.delay();
        }

        Ok(())
    }
}


#[derive(Debug)]
pub enum EngineInitError {
    OnCtxInit(CtxHandlerInitError),
}

impl Display for EngineInitError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let out = match self {
            Self::OnCtxInit(e) => format!("On System Context Init: {}", e),
        };

        write!(f, "Engine Init Panic: {}", out)
    }
}

impl Error for EngineInitError {}

impl From<CtxHandlerInitError> for EngineInitError {
    fn from(e: CtxHandlerInitError) -> Self {
        Self::OnCtxInit(e)
    }
}


#[derive(Debug)]
pub enum EngineRuntimeError {
    ByString(String),
    OnGraphicsLoop(GraphicsLoopError),
}

impl Display for EngineRuntimeError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let out = match self {
            Self::ByString(e) => format!("From String: {}", e),
            Self::OnGraphicsLoop(e) => format!("During Graphics Render and Loop: {}", e),
        };

        write!(f, "Engine Runtime Panic: {}", out)
    }
}

impl Error for EngineRuntimeError {}

impl From<String> for EngineRuntimeError {
    fn from(e: String) -> Self {
        Self::ByString(e)
    }
}

impl From<GraphicsLoopError> for EngineRuntimeError {
    fn from(e: GraphicsLoopError) -> Self {
        Self::OnGraphicsLoop(e)
    }
}

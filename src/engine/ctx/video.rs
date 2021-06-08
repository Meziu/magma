// SDL2 imports
use sdl2::video::Window;
use sdl2::{Sdl, VideoSubsystem};
use sdl2::video::WindowBuildError;

// std imports
use std::fmt::{Display, Formatter, Debug};
use std::error::Error;

// vulkan implementation imports
use super::vulkan::{GraphicsHandler, GraphicsLoopError, GraphicsHandlerCreationError};


/// Component of the CtxHandler to handle all calls to graphic APIs
pub struct VideoHandler {
    video_subsystem: VideoSubsystem,
    window: Window,
    gl_handler: GraphicsHandler,

    window_resized: bool,
}

impl VideoHandler {
    pub fn new(ctx: &Sdl, window_name: &str) -> Result<VideoHandler, VideoHandlerInitError> {
        let video_subsystem = ctx.video()?;

        let window = video_subsystem
            .window(window_name, 800, 600)
            .position_centered()
            .vulkan()
            .resizable()
            .build()?;

        let gl_handler = GraphicsHandler::new(&window)?;

        Ok(VideoHandler {
            video_subsystem,
            window,
            gl_handler,
            window_resized: false,
        })
    }

    fn get_window_resized(&self) -> bool {
        self.window_resized
    }
    pub fn set_window_resized(&mut self, new_value: bool) {
        self.window_resized = new_value;
    }

    /// Frame-by-frame update of the graphics and everything related
    pub fn update(&mut self) -> Result<(), GraphicsLoopError> {
        let resized = self.get_window_resized();

        self.gl_handler.vulkan_loop(resized, &self.window)?;

        self.set_window_resized(false);

        Ok(())
    }
}


#[derive(Debug)]
pub enum VideoHandlerInitError {
    ByString(String),
    FromGraphicsInit(GraphicsHandlerCreationError),
    FromWindowBuild(WindowBuildError),
}

impl Display for VideoHandlerInitError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let out = match self {
            Self::ByString(e) => format!("From String: {}", e),
            Self::FromGraphicsInit(e) => format!("On Graphics Init: {}", e),
            Self::FromWindowBuild(e) => format!("On Window Build: {}", e),
        };

        write!(f, "Video Handler Init Error: {}", out)
    }
}

impl From<String> for VideoHandlerInitError {
    fn from(e: String) -> Self {
        Self::ByString(e)
    }
}

impl From<GraphicsHandlerCreationError> for VideoHandlerInitError {
    fn from(e: GraphicsHandlerCreationError) -> Self {
        Self::FromGraphicsInit(e)
    }
}

impl From<WindowBuildError> for VideoHandlerInitError {
    fn from(e: WindowBuildError) -> Self {
        Self::FromWindowBuild(e)
    }
}

impl Error for VideoHandlerInitError {}

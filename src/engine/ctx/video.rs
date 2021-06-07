// SDL2 imports
use sdl2::video::Window;
use sdl2::{Sdl, VideoSubsystem};

// std imports
use std::error::Error;

// vulkan implementation imports
use super::vulkan::{GraphicsHandler, GraphicsLoopError};


/// Component of the CtxHandler to handle all calls to graphic APIs
pub struct VideoHandler {
    video_subsystem: VideoSubsystem,
    window: Window,
    gl_handler: GraphicsHandler,

    window_resized: bool,
}

impl VideoHandler {
    pub fn new(ctx: &Sdl, window_name: &str) -> Result<VideoHandler, Box<dyn Error>> {
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

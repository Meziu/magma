// SDL2 imports
use sdl2::video::Window;
use sdl2::{Sdl, VideoSubsystem};

// vulkan implementation imports
use super::vulkan::GraphicsHandler;

/// Component of the CtxHandler to handle all calls to graphic APIs
pub struct VideoHandler {
    video_subsystem: VideoSubsystem,
    window: Window,
    gl_handler: GraphicsHandler,

    window_resized: bool,
}

impl VideoHandler {
    pub fn new(ctx: &Sdl) -> VideoHandler {
        let video_subsystem = ctx.video().expect("Couldn't obtain SDL2 Video Subsystem");

        let window = video_subsystem
            .window("Rust Testing Grounds", 800, 600)
            .position_centered()
            .vulkan()
            .resizable()
            .build()
            .expect("Couldn't build SDL2 Window from Video Subsystem");

        let gl_handler = GraphicsHandler::new(&window);

        VideoHandler {
            video_subsystem,
            window,
            gl_handler,
            window_resized: false,
        }
    }

    fn get_window_resized(&self) -> bool {
        self.window_resized
    }
    pub fn set_window_resized(&mut self, new_value: bool) {
        self.window_resized = new_value;
    }

    /// Frame-by-frame update of the graphics and everything related
    pub fn update(&mut self) {
        let resized = self.get_window_resized();

        self.gl_handler.vulkan_loop(resized, &self.window);

        self.set_window_resized(false);
    }
}

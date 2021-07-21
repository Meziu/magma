// standard imports

// SDL2 imports
use sdl2::video::Window;
use sdl2::{Sdl, VideoSubsystem};

// vulkan implementation imports
use super::vulkan::GraphicsHandler;

// other imports
use super::draw_objects::{SpriteObject, PrimitiveObject};
use cgmath::{Vector2, Vector4};

/// Component of the CtxHandler to handle all calls to graphic APIs
pub struct VideoHandler {
    video_subsystem: VideoSubsystem,
    window: Window,
    pub gl_handler: GraphicsHandler,

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

    pub fn new_sprite(&mut self, texture_path: &str, z_index: u8) -> SpriteObject {
        self.gl_handler.new_sprite(texture_path, z_index)
    }

    pub fn new_rectangle(&mut self, scale: Vector2<f32>, color: Vector4<f32>, global_position: Vector2<f32>, z_index: u8) -> PrimitiveObject {
        self.gl_handler.new_rectangle(scale, color, global_position, z_index)
    }

    /// Frame-by-frame update of the graphics and everything related
    pub fn update(&mut self) {
        let resized = self.get_window_resized();

        self.gl_handler.vulkan_loop(resized, &self.window);

        self.set_window_resized(false);
    }
}

// standard imports
use std::error::Error;
use std::path::Path;

// sdl2 imports
use sdl2::event::Event;

// import the sdlhandler.rs file
mod sdlhandler;
use sdlhandler::SdlHandler;

/// Main struct to handle the whole program in all it's components
pub struct Engine {
    pub sdl_manager: SdlHandler,
}

impl Engine {
    pub fn new() -> Result<Engine, Box<dyn Error>> {
        let sdl_manager = SdlHandler::new()?;

        Ok(Engine { sdl_manager })
    }

    /// Main function to run the program, returns an error if any panics are necessary
    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let mut i: f32 = 0.0;

        let chunk = self
            .sdl_manager
            .audio
            .sfx_from_file(Path::new("example.ogg"))?;
        self.sdl_manager.audio.sfx_play(&chunk)?;

        let mut vao: gl::types::GLuint = 0;
        let mut shader_program: u32 = 0;
        self.sdl_manager
            .video
            .hello_triangle_init(&mut vao, &mut shader_program);

        'mainloop: loop {
            i = (i + 1.0 / 255.0) % 1.0;
            self.sdl_manager
                .video
                .gl_set_clear_color(i, 64.0 / 255.0, 1.0 - i, 1 as f32);
            self.sdl_manager.video.gl_clear();

            for event in self.sdl_manager.event_pump.poll_iter() {
                if let Event::Quit { .. } = event {
                    break 'mainloop;
                }
            }

            self.sdl_manager
                .video
                .hello_triangle_draw(shader_program, vao);

            self.sdl_manager.video.gl_window_swap();
            self.sdl_manager.fps_manager.delay();
        }

        Ok(())
    }
}

// standard imports
use std::error::Error;
use std::path::Path;

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
        let chunk = self
            .sdl_manager
            .audio
            .sfx_from_file(Path::new("assets/example.ogg"))?;
        self.sdl_manager.audio.sfx_play(&chunk)?;

        'mainloop: loop {
            self.sdl_manager.check_events();
            if self.sdl_manager.get_break_signal() {
                break 'mainloop;
            }

            self.sdl_manager.video.update()?;

            self.sdl_manager.fps_manager.delay();
        }

        Ok(())
    }
}

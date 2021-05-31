// standard imports
use std::error::Error;
use std::path::Path;

// sdl2 imports
use sdl2::event::Event;
use sdl2::event::WindowEvent;

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
        //let mut i: f32 = 0.0;

        let chunk = self
            .sdl_manager
            .audio
            .sfx_from_file(Path::new("assets/example.ogg"))?;
        self.sdl_manager.audio.sfx_play(&chunk)?;

        'mainloop: loop {
            let mut resized_window = false;
            for event in self.sdl_manager.event_pump.poll_iter() {
                match event {
                    Event::Quit{ .. } => break 'mainloop,
                    Event::Window{win_event, ..} => { 
                        if let WindowEvent::Resized(_, _) = win_event {
                            resized_window = true;
                        }
                    },
                    _ => {},
                }
            }

            self.sdl_manager.video.update(resized_window)?;
            self.sdl_manager.fps_manager.delay();
        }

        Ok(())
    }
}

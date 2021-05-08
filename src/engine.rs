// standard imports
use std::error::Error;
use std::path::Path;

// sdl2 imports
use sdl2::event::Event;
use sdl2::pixels::Color;

// import the sdlhandler.rs file
mod sdlhandler;
use sdlhandler::SdlHandler;

/// Main struct to handle the whole program in all it's components
pub struct Engine
{
    pub sdl_manager: SdlHandler,
}

impl Engine
{
    pub fn new() -> Result<Engine, Box<dyn Error>>
    {
        let sdl_manager = SdlHandler::new()?;

        Ok(
            Engine
            {
                sdl_manager
            }
        )
    }

    /// Main function to run the program, returns an error if any panics are necessary
    pub fn run(&mut self) -> Result<(), Box<dyn Error>>
    {
        let mut i = 0;

        self.sdl_manager.audio.music_from_file(Path::new("sample.mp3"))?;
        self.sdl_manager.audio.music_play(-1)?;

        'mainloop : loop 
        {
            i = (i + 1) % 255;
            self.sdl_manager.video.canvas_set_draw_color(Color::RGB(i, 64, 255 - i));
            self.sdl_manager.video.canvas_clear();

            for event in self.sdl_manager.event_pump.poll_iter()
            {
                if let Event::Quit {..} = event 
                {
                    break 'mainloop
                }
            }
    
            self.sdl_manager.video.canvas_present();
            self.sdl_manager.fps_manager.delay();
        }

        //self.sdl_manager.

        Ok(())
    }
}

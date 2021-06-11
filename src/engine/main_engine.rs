// standard imports
use std::path::Path;

// import the ctx mdule
use super::ctx::CtxHandler;

/// Main struct to handle the whole program in all it's components
pub struct Engine {
    ctx_handler: CtxHandler,
}

impl Engine {
    /// Engine init process
    pub fn new() -> Self {
        let ctx_handler = CtxHandler::new(60);

        Self { ctx_handler }
    }

    /// Main function to run the program
    pub fn run(&mut self) {
        if let Ok(_) = self
            .ctx_handler
            .audio
            .music_from_file(Path::new("assets/example.ogg"))
        {
            println!("Music was loaded fine!");
            match self.ctx_handler.audio.music_play(-1) {
                Ok(_) => println!("Music played fine!"),
                Err(_) => println!("Music couldn't play..."),
            }
        }

        'mainloop: loop {
            self.ctx_handler.check_events();
            if self.ctx_handler.get_break_signal() {
                break 'mainloop;
            }

            self.ctx_handler.video.update();

            self.ctx_handler.wait();
        }
    }
}

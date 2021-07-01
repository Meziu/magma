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
        let ctx_handler = CtxHandler::new();

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
        } else {
            println!("Music couldn't be loaded...");
        }

        // before, z index wasn't sorted and depth depended on the order in the vector
        // now the order isn't important but the z index must be specified
        self.ctx_handler.video.new_sprite("assets/rust.png", 1);
        self.ctx_handler.video.new_sprite("assets/python.png", 0);
        
        'mainloop: loop {
            self.ctx_handler.check_events();
            if self.ctx_handler.get_break_signal() {
                break 'mainloop;
            }
            println!("{:#?}, {:#?}", self.ctx_handler.video.gl_handler.read_window_size(),  self.ctx_handler.video.gl_handler.read_camera_position());
            
            self.ctx_handler.video.gl_handler.write_camera_position(cgmath::Vector2::new(0.0, 100.0));
            

            self.ctx_handler.video.update();

            self.ctx_handler.wait();
        }
    }
}

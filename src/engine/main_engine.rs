// standard imports
use std::path::Path;

// import the ctx mdule
use super::ctx::CtxHandler;

// other imports

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
        if self
            .ctx_handler
            .audio
            .music_from_file(Path::new("assets/example.ogg")).is_ok()
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
        let _ferris = self.ctx_handler.video.new_sprite("assets/rust.png", 1);
        let python = self.ctx_handler.video.new_sprite("assets/python.png", 1);

        let _rect = self.ctx_handler.video.new_rectangle((100.0, 100.0).into(), (0.0, 0.0, 1.0, 1.0).into(), (200.0, 200.0).into(), 2);

        let mut i = 0.0;
        'mainloop: loop {
            self.ctx_handler.check_events();
            if self.ctx_handler.get_break_signal() {
                break 'mainloop;
            }

            i += 2.0;
            {
                self.ctx_handler.video.gl_handler.camera_scale.y = 1.0 - (i / 1000.0);

                let mut sprite = python.get_mut();
                sprite.global_position.x = i;
                sprite.color = cgmath::Vector4::new(1.0, 1.0, 1.0, 1.0 - (i / 255.0));
            }

            self.ctx_handler.video.update();

            self.ctx_handler.wait();

            println!("{}", self.ctx_handler.get_current_framerate());
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

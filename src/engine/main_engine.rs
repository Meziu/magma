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
    pub fn new() -> Self{
        let ctx_handler = CtxHandler::new();

        Self { ctx_handler }
    }

    /// Main function to run the program, returns an error if any panics are necessary
    pub fn run(&mut self) {
        let chunk = self
            .ctx_handler
            .audio
            .sfx_from_file(Path::new("assets/example.ogg"));
        self.ctx_handler.audio.sfx_play(&chunk);

        'mainloop: loop {
            self.ctx_handler.check_events();
            if self.ctx_handler.get_break_signal() {
                break 'mainloop;
            }

            self.ctx_handler.video.update();

            self.ctx_handler.fps_manager.delay();
        }
    }
}

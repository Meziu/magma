//! Quick implementation of a framerate handler to avoid needing SDL2_gfx

// standard imports
use std::time;
use std::thread;


/// Basic struct to handle FPS waiting
pub struct FPSHandler {
    last_loop: time::Instant,
    fps: u16,
}

impl FPSHandler {
    pub fn new(fps: u16) -> Self {
        Self {
            last_loop: time::Instant::now(),
            fps: fps,
        }
    }

    pub fn get_fps(&self) -> u16 {
        self.fps
    }

    pub fn set_fps(&mut self, new_fps: u16) {
        self.fps = new_fps;
    }

    pub fn wait(&mut self) -> f32 {
        let time_elapsed = self.last_loop.elapsed();

        let total_nanos = time_elapsed.as_secs() * 1_000_000_000 + time_elapsed.subsec_nanos() as u64;
        let delta = (1. / self.fps as f32) * 1_000_000_000. - (total_nanos as f32);

        if delta > 0. {
            thread::sleep(time::Duration::new(0, delta as u32))
        };

        self.last_loop = time::Instant::now();

        delta
    }
}
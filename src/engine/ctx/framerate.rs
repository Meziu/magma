//! Quick implementation of a framerate handler to avoid needing SDL2_gfx

// standard imports
use std::time::{Duration, Instant};
use std::thread;


/// Basic struct to handle FPS waiting
pub struct FPSHandler {
    last_loop: Instant,
    delta: f32,
    limit: f32,
}

impl FPSHandler {
    pub fn new(limit: u16) -> Self {
        let limit = 1. / limit as f32;

        Self {
            last_loop: Instant::now(),
            delta: 0.0,
            limit,
        }
    }

    pub fn get_limit(&self) -> f32 {
        self.limit
    }
    pub fn set_limit(&mut self, new_limit: f32) {
        self.limit = new_limit;
    }

    pub fn get_fps(&self) -> u16 {
        (1. / self.get_delta()).round() as u16
    }

    pub fn get_delta(&self) -> f32 {
        self.delta
    }

    pub fn wait(&mut self) {
        let time_elapsed = self.last_loop.elapsed().as_secs_f32();

        let wait_time = self.limit - time_elapsed;

        // If we are early on the framerate limit, wait for it
        if wait_time > 0. {
            thread::sleep(Duration::from_secs_f32(wait_time));
        };

        self.delta = self.last_loop.elapsed().as_secs_f32();

        self.last_loop = Instant::now();
    }
}
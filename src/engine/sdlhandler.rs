// render imports
use sdl2::pixels::Color;
use sdl2::{Sdl, VideoSubsystem};
use sdl2::render::WindowCanvas;
use sdl2::EventPump;
use sdl2::gfx::framerate::FPSManager;
use sdl2::mixer;

// std imports
use std::error::Error;
use std::path::Path;


/// Main handler to manage calls to the SDL2 API
pub struct SdlHandler
{
    context             : Sdl ,
    pub event_pump      : EventPump,
    pub video           : SdlVideoHandler,
    pub fps_manager     : FPSManager,
    pub audio           : SdlAudioHandler,
}

impl SdlHandler
{
    /// Generate a new handler with a new context, window, event pump
    pub fn new() -> Result<SdlHandler, Box<dyn Error>>
    {
        let context = sdl2::init()?;

        let event_pump = context.event_pump()?;

        let video = SdlVideoHandler::new(&context, "Rust Testing Grounds")?;
        let audio = SdlAudioHandler::new()?;

        let mut fps_manager = FPSManager::new();
        fps_manager.set_framerate(60u32)?;

        Ok(
            SdlHandler
            {
                context,
                event_pump,
                video,
                fps_manager,
                audio,
            }
        )
    }
}

/// Component of the SdlHandler to handle all graphics
pub struct SdlVideoHandler
{
    video_subsystem : VideoSubsystem,
    canvas          : WindowCanvas,
}

impl SdlVideoHandler
{
    fn new(context: &Sdl, window_name: &str) -> Result<SdlVideoHandler, Box<dyn Error>>
    {
        let video_subsystem = context.video()?;
     
        let window = video_subsystem.window(window_name, 800, 600)
            .position_centered()
            .build()?;
     
        let mut canvas = window.into_canvas().build()?;
    
        canvas.set_draw_color(Color::RGB(0, 255, 255));
        canvas.clear();
        canvas.present();

        Ok(
            SdlVideoHandler
            {
                video_subsystem,
                canvas,
            }
        )
    }

    /// Fill the canvas with the clear colour
    pub fn canvas_clear(&mut self)
    {
        self.canvas.clear();
    }

    /// Set the canvas' current drawing colour
    pub fn canvas_set_draw_color(&mut self, c: Color)
    {
        self.canvas.set_draw_color(c);
    }

    /// Render the canvas
    pub fn canvas_present(&mut self)
    {
        self.canvas.present();
    }
}


pub struct SdlAudioHandler
{
    mix_context : mixer::Sdl2MixerContext,
    music       : Option<Box<mixer::Music<'static>>>,
}

impl SdlAudioHandler
{
    fn new() -> Result<SdlAudioHandler, Box<dyn Error>>
    {
        let init_flags = mixer::InitFlag::all();

        let mix_context = mixer::init(init_flags)?;

        mixer::allocate_channels(3);

        mixer::open_audio(44100, mixer::AUDIO_U16, 2, 512)?;
        
        Ok(
            SdlAudioHandler
            {
                mix_context,
                music: None,
            }
        )
    }


    // functions wrapper to handle music behaviour
    pub fn music_from_file<P: AsRef<Path>> (&mut self, path: P) -> Result<(), Box<dyn Error>>
    {
        let new_music = mixer::Music::from_file(path)?;
        
        self.music = Some(Box::new(new_music));

        Ok(())
    }

    pub fn music_play(&self, loops: i32) -> Result<(), Box<dyn Error>>
    {
        if let Some(m) = &self.music
        {
            m.play(loops)?;
        }

        Ok(())
    }

    pub fn music_pause(&self)
    {
        mixer::Music::pause();
    }

    pub fn music_resume(&self)
    {
        mixer::Music::resume();
    }

    pub fn music_rewind(&self)
    {
        mixer::Music::rewind();
    }

    pub fn music_stop(&self)
    {
        mixer::Music::halt();
    }

    pub fn music_get_volume(&self) -> i32
    {
        mixer::Music::get_volume()
    }

    pub fn music_set_volume(&self, volume: i32)
    {
        mixer::Music::set_volume(volume);
    }
    //-------------------------------------------------------------------
}

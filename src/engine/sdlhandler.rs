// render imports
use sdl2::gfx::framerate::FPSManager;
use sdl2::mixer::{self, Channel, Chunk, Music};
use sdl2::video::{Window};
use sdl2::EventPump;
use sdl2::{Sdl, VideoSubsystem};

// std imports
use std::error::Error;
use std::path::Path;

// import the vulkan.rs file
pub mod vulkan;
use vulkan::*;

/// Main handler to manage calls to the SDL2 API
pub struct SdlHandler {
    sdl: Sdl,
    pub event_pump: EventPump,
    pub video: SdlVideoHandler,
    pub fps_manager: FPSManager,
    pub audio: SdlAudioHandler,
}

impl SdlHandler {
    /// Generate a new handler with a new context, window, event pump
    pub fn new() -> Result<SdlHandler, Box<dyn Error>> {
        let sdl = sdl2::init()?;

        let event_pump = sdl.event_pump()?;

        let video = SdlVideoHandler::new(&sdl, "Rust Testing Grounds")?;
        let audio = SdlAudioHandler::new()?;

        let mut fps_manager = FPSManager::new();
        fps_manager.set_framerate(60u32)?;

        Ok(SdlHandler {
            sdl,
            event_pump,
            video,
            fps_manager,
            audio,
        })
    }
}

/// Component of the SdlHandler to handle all calls to graphic API's
pub struct SdlVideoHandler {
    video_subsystem: VideoSubsystem,
    window: Window,
    gl_handler: GraphicsHandler,
}

impl SdlVideoHandler {
    fn new(sdl: &Sdl, window_name: &str) -> Result<SdlVideoHandler, Box<dyn Error>> {
        let video_subsystem = sdl.video()?;

        let window = video_subsystem
            .window(window_name, 800, 600)
            .position_centered()
            .vulkan()
            .resizable()
            .build()?;

        let gl_handler = GraphicsHandler::new(&window)?;

        Ok(SdlVideoHandler {
            video_subsystem,
            window,
            gl_handler,
        })
    }

    pub fn update(&mut self, resized: bool)
    {
        self.gl_handler.vulkan_loop(resized, &self.window);
    }
}

/// Component of the SdlHandler to handle all calls to SDL_Mixer's API
pub struct SdlAudioHandler {
    mix_context: mixer::Sdl2MixerContext,
    music: Option<Box<Music<'static>>>,
    general_channel: Channel,
}

impl SdlAudioHandler {
    fn new() -> Result<SdlAudioHandler, Box<dyn Error>> {
        let mut init_flags = mixer::InitFlag::empty();
        init_flags.set(mixer::InitFlag::OGG, true);

        let mix_context = mixer::init(init_flags)?;

        mixer::allocate_channels(5);

        mixer::open_audio(44100, mixer::AUDIO_U16, 2, 1024)?;

        let general_channel = Channel::all();

        Ok(SdlAudioHandler {
            mix_context,
            music: None,
            general_channel,
        })
    }

    //----------------
    // SOUND EFFECTS
    //----------------
    pub fn sfx_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Box<Chunk>, Box<dyn Error>> {
        let mut new_chunk = Chunk::from_file(path)?;
        new_chunk.set_volume(30);
        let new_chunk = Box::new(new_chunk);

        Ok(new_chunk)
    }

    pub fn sfx_play(&self, chunk: &Box<Chunk>) -> Result<(), Box<dyn Error>> {
        let _channel = self.general_channel.play(chunk.as_ref(), 0)?;

        Ok(())
    }
    //-----------------------------------------------------------------

    //--------
    // MUSIC
    //--------
    pub fn music_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn Error>> {
        let new_music = Music::from_file(path)?;
        self.music = Some(Box::new(new_music));

        Ok(())
    }

    pub fn music_play(&self, loops: i32) -> Result<(), Box<dyn Error>> {
        if let Some(m) = &self.music {
            m.play(loops)?;
        }

        Ok(())
    }

    pub fn music_pause(&self) {
        Music::pause();
    }

    pub fn music_resume(&self) {
        Music::resume();
    }

    pub fn music_rewind(&self) {
        Music::rewind();
    }

    pub fn music_stop(&self) {
        Music::halt();
    }

    pub fn music_get_volume(&self) -> i32 {
        Music::get_volume()
    }

    pub fn music_set_volume(&self, volume: i32) {
        Music::set_volume(volume);
    }
    //--------------------------------------------------------------
}

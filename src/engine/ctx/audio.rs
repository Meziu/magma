// std imports
use std::path::Path;

// SDL2 imports
use sdl2::mixer::{self, Channel, Chunk, Music};


/// Component of the CtxHandler to handle all calls to SDL_Mixer's API
pub struct AudioHandler {
    mix_context: mixer::Sdl2MixerContext,
    music: Option<Box<Music<'static>>>,
    general_channel: Channel,
}

impl AudioHandler {
    pub fn new() -> AudioHandler{
        let mut init_flags = mixer::InitFlag::empty();
        init_flags.set(mixer::InitFlag::OGG, true);

        let mix_context = mixer::init(init_flags).expect("Couldn't init SDL2 Mixer context");

        mixer::allocate_channels(5);

        mixer::open_audio(44100, mixer::AUDIO_U16, 2, 1024).expect("Couldn't open audio on SDL2 Mixer Context");

        let general_channel = Channel::all();

        AudioHandler {
            mix_context,
            music: None,
            general_channel,
        }
    }

    //----------------
    // SOUND EFFECTS
    //----------------
    pub fn sfx_from_file<P: AsRef<Path>>(&mut self, path: P) -> Option<Box<Chunk>> {
        match Chunk::from_file(path) {
            Ok(mut chunk) => {
                chunk.set_volume(30);
                Some(Box::new(chunk))
            },
            Err(e) => {
                eprintln!("Couldn't load SFX from file: {}", e); 
                None
            },
        }
    }

    pub fn sfx_play(&self, chunk: &Option<Box<Chunk>>) -> Option<Channel> {
        if let Some(chunk_box) = chunk {
            match self.general_channel.play(chunk_box.as_ref(), 0) {
                Ok(c) => Some(c),
                Err(e) => {
                    eprintln!("Couldn't play SFX: {}", e);
                    None
                },
            }
        }
        else {
            eprintln!("Tried to play non-existing SFX");
            None
        }
    }

    //--------
    // MUSIC
    //--------
    pub fn music_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let new_music = Music::from_file(path)?;
        self.music = Some(Box::new(new_music));

        Ok(())
    }

    pub fn music_play(&self, loops: i32) -> Result<(), String> {
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
}
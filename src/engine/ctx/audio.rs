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
    pub fn sfx_from_file(&mut self, path: &Path) -> SoundEffect {
        let new_chunk = match Chunk::from_file(path) {
            Ok(chunk) => {
                Some(Box::new(chunk))
            },
            Err(e) => {
                eprintln!("Couldn't load SFX from file \'{}\': {}", path.display(), e); 
                None
            },
        };

        SoundEffect {data: new_chunk, volume: 30,}
    }

    pub fn sfx_play(&self, chunk: &SoundEffect) -> Option<Channel> {
        if let Some(chunk_box) = &chunk.data {
            match self.general_channel.play(chunk_box.as_ref(), 0) {
                Ok(c) => {
                    c.set_volume(30);
                    Some(c)
                },
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
    pub fn music_from_file(&mut self, path: &Path) -> Result<(), ()> {
        match Music::from_file(path) {
            Ok(music) => {
                self.music = Some(Box::new(music));
                self.music_set_volume(30);
                Ok(())
            },
            Err(e) => {
                eprintln!("Couldn't load music from file \'{}\': {}", path.display(), e);
                Err(())
            },
        }
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

pub struct SoundEffect {
    data: Option<Box<Chunk>>,
    volume: i32,
}

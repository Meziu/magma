// render imports
use sdl2::gfx::framerate::FPSManager;
use sdl2::mixer::{self, Channel, Chunk, Music};
use sdl2::video::{GLContext, Window};
use sdl2::EventPump;
use sdl2::{Sdl, VideoSubsystem};

// std imports
use std::error::Error;
use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::path::Path;

// OpenGL imports
use gl::types::*;
use gl::{self};


// import the opengl.rs file
pub mod opengl;
use opengl::*;

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
    gl_context: GLContext,
}

impl SdlVideoHandler {
    fn new(sdl: &Sdl, window_name: &str) -> Result<SdlVideoHandler, Box<dyn Error>> {
        let video_subsystem = sdl.video()?;

        {
            let gl_attr = video_subsystem.gl_attr();
            gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
            gl_attr.set_context_version(2, 1);
            // DEBUG CONTEXT
            gl_attr.set_context_flags().debug().set();
        }

        let window = video_subsystem
            .window(window_name, 800, 600)
            .position_centered()
            .opengl()
            .resizable()
            .build()?;
        
        let gl_context = window.gl_create_context()?;
        let _gl = gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const c_void);

        Ok(SdlVideoHandler {
            video_subsystem,
            window,
            gl_context,
        })
    }

    /// Set the context's clear colour
    #[inline(always)]
    pub fn gl_set_clear_color(&self, r: f32, g: f32, b: f32, a: f32) {
        unsafe { gl::ClearColor(r, g, b, a) };
    }

    /// Fill the context with the clear colour
    #[inline(always)]
    pub fn gl_clear(&self) {
        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT) };
    }

    #[inline(always)]
    pub fn gl_window_swap(&self) {
        self.window.gl_swap_window();
    }

    pub fn hello_triangle_init(&self, vao: &mut GLuint, shader_program: &mut u32) {
        // SHADERS INIT AND COMPILE

        let mut success: i32 = 0;
        let info_log = create_whitespace_cstring_with_len(512);
        const vertex_shader_source: &str = "#version 120
        void main()
        {
            gl_Position = vec4(gl_Vertex.x, gl_Vertex.y, gl_Vertex.z, 1.0);
        }\0";

        const vrtx_sh_src: *const *const u8 = &vertex_shader_source.as_ptr();

        let vertex_shader = unsafe { gl::CreateShader(gl::VERTEX_SHADER) };
        unsafe {
            gl::ShaderSource(
                vertex_shader,
                1,
                vrtx_sh_src as *const *const GLchar,
                0 as *const i32,
            );
        };
        unsafe {
            gl::CompileShader(vertex_shader);

            gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut success as *mut i32);

            if success == gl::FALSE.into() {
                gl::GetShaderInfoLog(
                    vertex_shader,
                    512,
                    0 as *mut i32,
                    info_log.as_ptr() as *mut GLchar,
                );
                eprintln!(
                    "ERROR::SHADER::VERTEX::COMPILATION_FAILED\n{}\n",
                    CStr::from_ptr(info_log.as_ptr()).to_str().unwrap()
                );
            }
        };

        const fragment_shader_source: &str = "#version 120
        void main()
        {
            gl_FragColor = vec4(1.0f, 0.5f, 0.2f, 1.0f);
        }\0";
        const frgmt_sh_src: *const *const u8 = &fragment_shader_source.as_ptr();

        let fragment_shader = unsafe { gl::CreateShader(gl::FRAGMENT_SHADER) };
        unsafe {
            gl::ShaderSource(
                fragment_shader,
                1,
                frgmt_sh_src as *const *const GLchar,
                0 as *const i32,
            );
        };
        unsafe {
            gl::CompileShader(fragment_shader);

            gl::GetShaderiv(
                fragment_shader,
                gl::COMPILE_STATUS,
                &mut success as *mut i32,
            );

            if success == gl::FALSE.into() {
                gl::GetShaderInfoLog(
                    fragment_shader,
                    512,
                    0 as *mut i32,
                    info_log.as_ptr() as *mut GLchar,
                );
                eprintln!(
                    "ERROR::SHADER::FRAGMENT::COMPILATION_FAILED\n{}\n",
                    CStr::from_ptr(info_log.as_ptr()).to_str().unwrap()
                );
            }
        };

        // SHADER PROGRAM

        *shader_program = unsafe { gl::CreateProgram() };

        unsafe {
            gl::AttachShader(*shader_program, vertex_shader);
            gl::AttachShader(*shader_program, fragment_shader);

            let foo = CString::new("vertexPosition_modelspace").unwrap();

            gl::BindAttribLocation(*shader_program, 0, foo.as_ptr() as *const GLchar);

            gl::LinkProgram(*shader_program);

            gl::GetProgramiv(*shader_program, gl::LINK_STATUS, &mut success as *mut i32);

            if success == gl::FALSE.into() {
                gl::GetProgramInfoLog(
                    *shader_program,
                    512,
                    0 as *mut GLsizei,
                    info_log.as_ptr() as *mut GLchar,
                );
                println!(
                    "ERROR::SHADER::PROGRAM::LINKING_FAILED\n{}\n",
                    CStr::from_ptr(info_log.as_ptr()).to_str().unwrap()
                );
            }
        }

        // BUFFER INIT AND BIND
        let vertices: [f32; 9] = [
            -0.5, -0.5, 0.0, // top right
            0.5, -0.5, 0.0, // bottom right
            0.0, 0.5, 0.0, // top left
        ];
        let mut vbo: GLuint = 0;

        unsafe {
            gl::GenBuffers(1, &mut vbo);
            gl::GenVertexArrays(1, vao);

            gl::BindVertexArray(*vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (9 * std::mem::size_of::<f32>()) as isize,
                vertices.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                3 * std::mem::size_of::<f32>() as i32,
                0 as *const c_void,
            );
            gl::EnableVertexAttribArray(0);

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        };
    }

    pub fn hello_triangle_draw(&self, shader_program: u32, vao: GLuint) {
        unsafe {
            gl::UseProgram(shader_program);
            gl::BindVertexArray(vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
            gl::BindVertexArray(0);
        }
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

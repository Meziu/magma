// OpenGL imports
use gl::types::*;
use gl::{self};

// std imports
use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::path::Path;
use std::fs::File;
use std::error::Error;
use std::io::Read;
use std::fmt::{self, Display, Debug};


/// Struct to handle all OpenGL API calls
pub struct OpenGLHandler {
    pub shader_program: ShaderProgram,
}

impl OpenGLHandler
{
    pub fn new() -> Result<OpenGLHandler, Box<dyn Error>> {
        let shader_program = ShaderProgram::new()?;
    
        Ok(
            OpenGLHandler{
                shader_program
            }
        )
    }
}


/// Simple struct to handle the shader program OpenGL API
pub struct ShaderProgram {
    id: GLuint,
}

impl ShaderProgram {
    fn new() -> Result<ShaderProgram, Box<dyn Error>> {
        // SHADERS INIT AND COMPILE

        let mut success: i32 = 0;
        let info_log = create_whitespace_cstring_with_len(512);

        let vertex_shader = Shader::vert_from_file(Path::new("shader.vert"))?;

        let fragment_shader = Shader::frag_from_file(Path::new("shader.frag"))?;

        // SHADER PROGRAM

        let id = unsafe { gl::CreateProgram() };

        unsafe {
            gl::AttachShader(id, vertex_shader.id);
            gl::AttachShader(id, fragment_shader.id);

            let foo = CString::new("vertexPosition_modelspace").unwrap();

            gl::BindAttribLocation(id, 0, foo.as_ptr() as *const GLchar);

            gl::LinkProgram(id);

            gl::GetProgramiv(id, gl::LINK_STATUS, &mut success as *mut i32);

            if success == gl::FALSE.into() {
                gl::GetProgramInfoLog(id, 512, 0 as *mut GLsizei, info_log.as_ptr() as *mut GLchar);
                println!(
                    "ERROR::SHADER::PROGRAM::LINKING_FAILED\n{}\n",
                    CStr::from_ptr(info_log.as_ptr()).to_str().unwrap()
                );
            }
        }

        Ok(ShaderProgram { id })
    }

    #[inline(always)]
    pub fn set_used(&self) {
        unsafe { gl::UseProgram(self.id); }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}


struct ShaderCreationError;

impl Display for ShaderCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error while creating Shader object")
    }
}

impl Debug for ShaderCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{ file: {}, line: {} }}", file!(), line!())
    }
}

impl Error for ShaderCreationError{}

struct Shader
{
    id : GLuint
}

impl Shader {
    fn new<P: AsRef<Path>> (path: P, kind: GLenum) -> Result<Shader, Box<dyn Error>> {
        let mut file = File::open(path)?;
        let mut source = String::new();
        file.read_to_string(&mut source).expect("Couldn't read file\n");

        let source = &CString::new(source).unwrap();
        match Shader::shader_from_source(source, kind)
        {
            Ok(id) => return Ok(Shader { id }),
            Err(e) => return Err(Box::new(ShaderCreationError{})),
        };
    }

    /// Create a vertex shader
    #[inline(always)]
    pub fn vert_from_file<P: AsRef<Path>> (path: P) -> Result<Shader, Box<dyn Error>> {
        Shader::new(path, gl::VERTEX_SHADER)
    }

    /// Create a fragment shader
    #[inline(always)]
    pub fn frag_from_file<P: AsRef<Path>> (path: P) -> Result<Shader, Box<dyn Error>> {
        Shader::new(path, gl::FRAGMENT_SHADER)
    }

    /// Get the shader's id
    #[inline(always)]
    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }

    /// Function to create a shader out of a string
    fn shader_from_source(source: &CStr, kind: gl::types::GLenum) -> Result<gl::types::GLuint, ()> {
        let mut success: i32 = 0;
        let info_log = create_whitespace_cstring_with_len(512);

        let src: *const *const i8 = &source.as_ptr();

        let id = unsafe { gl::CreateShader(kind) };
        unsafe {
            gl::ShaderSource(
                id,
                1,
                src as *const *const GLchar,
                0 as *const i32,
            );
        };
        unsafe {
            gl::CompileShader(id);

            gl::GetShaderiv(
                id,
                gl::COMPILE_STATUS,
                &mut success as *mut i32,
            );

            if success == gl::FALSE.into() {
                gl::GetShaderInfoLog(
                    id,
                    512,
                    0 as *mut i32,
                    info_log.as_ptr() as *mut GLchar,
                );
                
                let error = format!(
                    "ERROR::SHADER::{}::COMPILATION_FAILED\n{}\n",
                    kind,
                    CStr::from_ptr(info_log.as_ptr()).to_str().unwrap()
                );

                eprintln!("{}", error);
                return Err(());
            }
        };

        Ok(id)
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.id);
        }
    }
}


fn create_whitespace_cstring_with_len(len: usize) -> CString {
    // allocate buffer of correct size
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    // fill it with len spaces
    buffer.extend([b' '].iter().cycle().take(len));
    // convert buffer to CString
    unsafe { CString::from_vec_unchecked(buffer) }
}

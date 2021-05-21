// OpenGL imports
use gl::types::*;
use gl::{self};

// std imports
use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt::{self, Debug, Display};
use std::os::raw::c_void;
use std::path::Path;

// other imports
use image::io::Reader as ImageReader;


/// Struct to handle all OpenGL API calls
pub struct OpenGLHandler {
    pub shader_program: ShaderProgram,
}

impl OpenGLHandler {
    pub fn new() -> Result<OpenGLHandler, Box<dyn Error>> {
        let shader_program = ShaderProgram::new()?;

        Ok(OpenGLHandler { shader_program })
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

        let vertex_shader = Shader::vert_from_file(include_str!("../../../assets/triangle.vert"))?;

        let fragment_shader =
            Shader::frag_from_file(include_str!("../../../assets/triangle.frag"))?;

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

    pub fn get_id(&self) -> u32 {
        self.id
    }

    #[inline(always)]
    pub fn set_used(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
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

impl Error for ShaderCreationError {}

/// Simple struct to handle shader creation
struct Shader {
    id: GLuint,
}

impl Shader {
    fn new(source: &str, kind: GLenum) -> Result<Shader, Box<dyn Error>> {
        let source = &CString::new(source).unwrap();
        match Shader::shader_from_source(source, kind) {
            Ok(id) => return Ok(Shader { id }),
            Err(_) => return Err(Box::new(ShaderCreationError {})),
        };
    }

    /// Create a vertex shader
    #[inline(always)]
    pub fn vert_from_file(source: &str) -> Result<Shader, Box<dyn Error>> {
        Shader::new(source, gl::VERTEX_SHADER)
    }

    /// Create a fragment shader
    #[inline(always)]
    pub fn frag_from_file(source: &str) -> Result<Shader, Box<dyn Error>> {
        Shader::new(source, gl::FRAGMENT_SHADER)
    }

    /// Function to create a shader out of a string
    fn shader_from_source(source: &CStr, kind: gl::types::GLenum) -> Result<gl::types::GLuint, ()> {
        let mut success: i32 = 0;
        let info_log = create_whitespace_cstring_with_len(512);

        let src: *const *const i8 = &source.as_ptr();

        let id = unsafe { gl::CreateShader(kind) };
        unsafe {
            gl::ShaderSource(id, 1, src as *const *const GLchar, 0 as *const i32);
        };
        unsafe {
            gl::CompileShader(id);

            gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success as *mut i32);

            if success == gl::FALSE.into() {
                gl::GetShaderInfoLog(id, 512, 0 as *mut i32, info_log.as_ptr() as *mut GLchar);
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

pub struct VertexArrayObject<'a> {
    id: GLuint,
    vbo: GLuint,
    program: &'a ShaderProgram,
}

impl VertexArrayObject<'_> {
    pub fn new(
        vertex_array: [f32; 18],
        program: &ShaderProgram,
    ) -> Result<VertexArrayObject, Box<dyn Error>> {
        let mut id: GLuint = 0;
        let mut vbo: GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
            gl::GenVertexArrays(1, &mut id);

            gl::BindVertexArray(id);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            gl::BufferData(
                gl::ARRAY_BUFFER,
                (18 * std::mem::size_of::<f32>()) as isize,
                vertex_array.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );
            
            // position attribute
            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                6 * std::mem::size_of::<f32>() as i32,
                0 as *const c_void,
            );

            gl::EnableVertexAttribArray(0);

            // color attribute
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                6 * std::mem::size_of::<f32>() as i32,
                (3 * std::mem::size_of::<f32>()) as *const c_void,
            );

            gl::BindAttribLocation(program.get_id(), 1, "vertexColor".as_ptr() as *const GLchar);

            gl::EnableVertexAttribArray(1);

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        };
        
        // let img: Vec<u8> = ImageReader::open(Path::new("assets/wall.jpg"))?.decode()?.to_rgba8().to_vec();

        Ok(VertexArrayObject { id, vbo, program })
    }

    pub fn draw(&self) {
        unsafe {
            self.program.set_used();
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BindVertexArray(self.id);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
            gl::BindVertexArray(0);
        }
    }
}

impl Drop for VertexArrayObject<'_> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.vbo);
        }
    }
}

// used to create log strings of arbitrary size
fn create_whitespace_cstring_with_len(len: usize) -> CString {
    // allocate buffer of correct size
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    // fill it with len spaces
    buffer.extend([b' '].iter().cycle().take(len));
    // convert buffer to CString
    unsafe { CString::from_vec_unchecked(buffer) }
}

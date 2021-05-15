// OpenGL imports
use gl::types::*;
use gl::{self};

// std imports
use std::ffi::{CStr, CString};
use std::os::raw::c_void;


/// Struct to handle all OpenGL API calls
struct OpenGLHandler {
    shader_program: ShaderProgram,
}


/// Simple struct to handle the shader program OpenGL API
struct ShaderProgram {
    id: GLuint,
}

impl ShaderProgram {
    pub fn new() -> Result<ShaderProgram, Box<dyn Error>> {
        // SHADERS INIT AND COMPILE

        let mut success: i32 = 0;
        let info_log = create_whitespace_cstring_with_len(512);

        let vertex_shader_source = &CString::new("#version 120
            void main()
            {
                gl_Position = vec4(gl_Vertex.x, gl_Vertex.y, gl_Vertex.z, 1.0);
            }");

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

        id = unsafe { gl::CreateProgram() };

        unsafe {
            gl::AttachShader(id, vertex_shader);
            gl::AttachShader(id, fragment_shader);

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
    pub fn use() {
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


struct Shader
{
    id : GLuint
}

impl Shader {
    fn from_source(source: &CStr, kind: GLenum) -> Result<Shader, String> {
        let id = shader_from_source(source, kind)?;
        Ok(Shader { id })
    }

    pub fn from_vert_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::VERTEX_SHADER)
    }

    pub fn from_frag_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::FRAGMENT_SHADER)
    }

    pub fn id(&self) -> gl::types::GLuint {
        self.id
    }
}


pub fn hello_triangle_init(&self, vao: &mut GLuint, shader_program: &mut u32) {
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

pub fn hello_triangle_draw(shader_program: u32, vao: GLuint) {
    unsafe {
        gl::UseProgram(shader_program);
        gl::BindVertexArray(vao);
        gl::DrawArrays(gl::TRIANGLES, 0, 3);
        gl::BindVertexArray(0);
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

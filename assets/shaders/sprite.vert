#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec3 color;

layout(location = 0) out vec3 frag_color;
layout(location = 1) out vec2 tex_coords;

void main() {
    gl_Position = vec4(position.x, -position.y, 0.0, 1.0);
    frag_color = color;
    
    tex_coords = position + vec2(0.5);
    tex_coords = vec2(tex_coords.s, 1.0 - tex_coords.t);
}
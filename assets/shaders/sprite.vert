#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec3 frag_color;
layout(location = 1) out vec2 tex_coords;

layout(set = 0, binding = 1) uniform readonly SpriteData {
    vec3 color;
    vec2 global_position;
} data;


void main() {
    gl_Position = vec4(position.x, position.y, 0.0, 1.0);
    frag_color = data.color;
    
    tex_coords = position + vec2(0.5);
}

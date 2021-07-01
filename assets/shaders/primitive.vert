#version 450

layout(location = 0) in vec2 vert_pos;

void main() {
    gl_Position = vec4(vert_pos.x, vert_pos.y, 0.0, 1.0);
}
#version 450

// positions of the vertices
layout(location = 0) in vec2 vert_pos;

// color and texture coordinates for the fragment shader
layout(location = 0) out vec3 frag_color;
layout(location = 1) out vec2 tex_coords;

// Data passed by the Sprite object
layout(set = 0, binding = 1) uniform readonly SpriteData {
    vec3 color;
    vec2 global_position;
    vec2 scale;
    uvec2 image_dimensions;
} sprite_data;

// Data passed by the Graphics Handler
layout(set = 0, binding = 2) uniform readonly GlobalData {
    uvec2 window_size;
    vec2 camera_position;
} global_data;


void main() {
    frag_color = sprite_data.color; // pass the sprite color to the fragment shader
    tex_coords = clamp(vert_pos, 0.0, 1.0); // texture coordinates can't be negative

    gl_Position = vec4(vert_pos.x, vert_pos.y, 0.0, 1.0);
}

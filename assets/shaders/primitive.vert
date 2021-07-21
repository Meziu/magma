#version 450

// positions of the vertices
layout(location = 0) in vec2 vert_pos;

// color and texture coordinates for the fragment shader
layout(location = 0) out vec4 frag_color;

// Data passed by the Primitive object
layout(set = 0, binding = 0) uniform readonly PrimitiveData {
    vec4 color;
    vec4 global_position;
    vec4 scale;
} primitive_data;

// Data passed by the Graphics Handler
layout(set = 0, binding = 1) uniform readonly GlobalData {
    uvec4 window_size;
    vec4 camera_position;
    vec4 camera_scale;
} global_data;


void main() {
    frag_color = primitive_data.color; // pass the sprite color to the fragment shader

    vec4 vertex_global_position = primitive_data.global_position + (vec4(vert_pos, 0.0, 0.0) * primitive_data.scale);

    vec2 rel_position = (vertex_global_position.xy - global_data.camera_position.xy) / (global_data.window_size.xy * global_data.camera_scale.xy);

    gl_Position = vec4(rel_position, 0.0, 1.0);
}

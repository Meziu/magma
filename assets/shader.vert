#version 120

attribute vec3 vertexColor;

varying vec3 out_color;

void main()
{
    out_color = vertexColor;
    gl_Position = vec4(gl_Vertex.x, gl_Vertex.y, gl_Vertex.z, 1.0);
}

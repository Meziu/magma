#version 120

void main()
{
    gl_Position = vec4(gl_Vertex.x, gl_Vertex.y, gl_Vertex.z, 1.0);
}

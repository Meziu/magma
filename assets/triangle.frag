#version 120

varying vec3 out_color;

void main()
{
    gl_FragColor = vec4(out_color.x, out_color.y, out_color.z, 1.0f);
}

#version 450

vec2 positions[3] = vec2[](
    vec2(0.0, -0.5), // top
    vec2(0.5, 0.5), // bottom right
    vec2(-0.5, 0.5) // bottom left
);

void main() {
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
}

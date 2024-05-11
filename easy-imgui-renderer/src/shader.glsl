precision highp float;

in vec2 pos;
in vec2 uv;
in vec4 color;
out vec2 v_uv;
out vec4 v_color;

uniform mat3 matrix;

void main(void) {
    vec3 tpos = matrix * vec3(pos, 1.0);
    gl_Position = vec4(tpos.xy, 0.0, tpos.z);
    v_uv = uv;
    v_color = color;
}

###
precision highp float;

in vec2 v_uv;
in vec4 v_color;
out vec4 out_frag_color;

uniform sampler2D tex;

void main(void) {
    out_frag_color = v_color * texture(tex, v_uv);
}

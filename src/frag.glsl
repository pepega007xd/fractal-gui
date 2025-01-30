precision highp float;

uniform vec2 center;
uniform vec2 window_offset;
uniform float zoom;
uniform vec2 resolution;
uniform int cycles;
uniform vec3 start_color;
uniform vec3 end_color;

out vec4 fragColor;

#define PI 3.14159265

vec3 hsv2rgb(vec3 c) {
    vec4 K = vec4(1., 2. / 3., 1. / 3., 3.);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

vec4 get_color(float param) {
    return vec4(hsv2rgb(mix(start_color, end_color, param)), 1.);
}

vec2 iteration(vec2 previous_z, vec2 original_z) {
    vec2 z;
    z.x = previous_z.x * previous_z.x - previous_z.y * previous_z.y + original_z.x;
    z.y = 2. * previous_z.x * previous_z.y + original_z.y;

    return z;
}

void main() {
    vec2 pos = ((gl_FragCoord.xy - window_offset) / resolution) - 0.5;
    pos.y *= -1.; // invert Y axis (opengl has 0,0 at bottom left corner, egui at top left)
    pos += center; // shift center acc to zoom
    pos /= zoom; // scale pos according to `zoom`
    pos.y *= resolution.y / resolution.x; // fix squishing in non-square aspect ratio

    vec2 z = pos;

    for (int i = 0; i < cycles; i++) {
        z = iteration(z, pos);

        if (z.x * z.x + z.y * z.y > 4.0) {
            float param = float(i) / float(cycles);
            fragColor = get_color(param);
            return;
        }
    }

    float param = atan(z.y, z.x) / PI / 2. + 0.5;
    fragColor = get_color(param);
}

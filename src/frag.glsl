precision highp float;

uniform vec2 center;
uniform vec2 window_offset;
uniform float zoom;
uniform vec2 resolution;
uniform int cycles;

out vec4 fragColor;

void main() {
    vec2 pixel_pos = gl_FragCoord.xy / 2.; // ???
    vec2 pos = ((pixel_pos - window_offset) / resolution) - 0.5;
    pos += center; // shift center acc to zoom
    pos /= zoom; // scale pos according to `zoom`
    pos.y *= resolution.y / resolution.x; // fix squishing in non-square aspect ratio

    vec2 z = pos;
    float color = 0.;

    for (int i = 0; i < cycles; i++) {
        float tmp = z.x;
        z.x = z.x * z.x - z.y * z.y + pos.x;
        z.y = 2. * tmp * z.y + pos.y;

        if (z.x * z.x + z.y * z.y > 4.0) {
            color = float(i) / float(cycles);
            break;
        }
    }

    fragColor = vec4(color, color, color, 1.0);
}

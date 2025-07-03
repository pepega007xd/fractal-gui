use std::sync::Arc;

use glow::{HasContext, Program};

use crate::app::UniformData;

pub struct Renderer {
    context: Arc<glow::Context>,
    program: glow::Program,
    vertex_array: glow::VertexArray,
}

// lel
unsafe impl Send for Renderer {}

pub const SHADER_VERSION: &str = if cfg!(target_arch = "wasm32") {
    "#version 300 es\n"
} else {
    "#version 330\n"
};

pub const MANDELBROT_FUNC: &str = r#"
// `z` is the iteratively updated complex number
// `p` is previous value of `z`, `o` is the original value of `z`
vec2 iteration(vec2 p, vec2 o) {
    vec2 z;
    z.x = p.x * p.x - p.y * p.y + o.x;
    z.y = 2. * p.x * p.y + o.y;

    return z;
}
"#;

pub const JULIA_FUNC: &str = r#"
vec2 iteration(vec2 previous_z, vec2 original_z) {
    vec2 z;
    z.x = previous_z.x * previous_z.x - previous_z.y * previous_z.y + 0.3;
    z.y = 2. * previous_z.x * previous_z.y - 0.4;

    return z;
}
"#;

impl Renderer {
    pub fn new(gl: Arc<glow::Context>) -> Self {
        unsafe {
            let program = Self::create_program(gl.as_ref(), MANDELBROT_FUNC)
                .expect("The builtin mandelbrot shader should be okay");

            let vertex_array = gl.create_vertex_array().unwrap();

            Self {
                context: gl,
                program,
                vertex_array,
            }
        }
    }

    fn check_shader(&self, fractal_function: &str) -> Result<(), String> {
        unsafe {
            let shader = self
                .context
                .create_shader(glow::FRAGMENT_SHADER)
                .expect("Cannot create shader");
            self.context.shader_source(
                shader,
                &(SHADER_VERSION.to_string() + include_str!("frag.glsl") + fractal_function),
            );
            self.context.compile_shader(shader);

            if !self.context.get_shader_compile_status(shader) {
                return Err(self.context.get_shader_info_log(shader));
            }
            self.context.delete_shader(shader);
        }

        Ok(())
    }

    fn create_program(gl: &glow::Context, fractal_function: &str) -> Result<Program, String> {
        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            let (vertex_shader_source, fragment_shader_source) = (
                // don't touch this
                r#"
                    const vec2 verts[6] = vec2[6](
                        vec2(-1.0, -1.0),
                        vec2(1.0, 1.0),
                        vec2(1.0, -1.0),
                        vec2(-1.0, -1.0),
                        vec2(-1.0, 1.0),
                        vec2(1.0, 1.0)
                    );
                    out vec4 v_color;
                    void main() {
                        gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
                    }
                "#,
                include_str!("frag.glsl").to_string() + fractal_function,
            );

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, &fragment_shader_source),
            ];

            let mut shaders = Vec::with_capacity(2);

            for (shader_type, shader_source) in shader_sources {
                let shader = gl.create_shader(shader_type).expect("Cannot create shader");
                gl.shader_source(shader, &(SHADER_VERSION.to_string() + shader_source));
                gl.compile_shader(shader);

                if !gl.get_shader_compile_status(shader) {
                    return Err(gl.get_shader_info_log(shader));
                }

                gl.attach_shader(program, shader);
                shaders.push(shader);
            }

            gl.link_program(program);

            if !gl.get_program_link_status(program) {
                return Err(gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            Ok(program)
        }
    }

    pub fn set_fractal_function(&mut self, fractal_function: &str) -> Result<(), String> {
        self.check_shader(fractal_function)?;
        unsafe {
            self.context.delete_program(self.program);
            self.program = Self::create_program(self.context.as_ref(), fractal_function)?;
        }
        Ok(())
    }

    pub fn destroy(&self) {
        use glow::HasContext as _;
        unsafe {
            self.context.delete_program(self.program);
            self.context.delete_vertex_array(self.vertex_array);
        }
    }

    pub fn paint(&self, uniform_data: UniformData) {
        unsafe {
            self.context.use_program(Some(self.program));
            self.context.uniform_2_f32(
                self.context
                    .get_uniform_location(self.program, "center")
                    .as_ref(),
                uniform_data.center.x,
                uniform_data.center.y,
            );
            self.context.uniform_1_i32(
                self.context
                    .get_uniform_location(self.program, "cycles")
                    .as_ref(),
                uniform_data.cycles,
            );
            self.context.uniform_1_f32(
                self.context
                    .get_uniform_location(self.program, "zoom")
                    .as_ref(),
                uniform_data.zoom,
            );
            self.context.uniform_2_f32(
                self.context
                    .get_uniform_location(self.program, "resolution")
                    .as_ref(),
                uniform_data.resolution.x,
                uniform_data.resolution.y,
            );
            self.context.uniform_2_f32(
                self.context
                    .get_uniform_location(self.program, "window_offset")
                    .as_ref(),
                uniform_data.window_offset.x,
                uniform_data.window_offset.y,
            );
            self.context.uniform_3_f32(
                self.context
                    .get_uniform_location(self.program, "start_color")
                    .as_ref(),
                uniform_data.start_color.h,
                uniform_data.start_color.s,
                uniform_data.start_color.v,
            );
            self.context.uniform_3_f32(
                self.context
                    .get_uniform_location(self.program, "end_color")
                    .as_ref(),
                uniform_data.end_color.h,
                uniform_data.end_color.s,
                uniform_data.end_color.v,
            );

            self.context.bind_vertex_array(Some(self.vertex_array));
            self.context.draw_arrays(glow::TRIANGLES, 0, 6);
        }
    }

    pub fn render_to_buffer(&self, width: u32, height: u32, uniform_data: UniformData) -> Vec<u8> {
        use glow::HasContext as _;

        unsafe {
            // Create a texture to render into
            let texture = self
                .context
                .create_texture()
                .expect("Failed to create texture for framebuffer");
            self.context.bind_texture(glow::TEXTURE_2D, Some(texture));
            self.context.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
            self.context.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            self.context.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );

            // Create a framebuffer and attach the texture
            let framebuffer = self
                .context
                .create_framebuffer()
                .expect("Failed to create framebuffer");
            self.context
                .bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            self.context.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );

            assert!(
                self.context.check_framebuffer_status(glow::FRAMEBUFFER)
                    == glow::FRAMEBUFFER_COMPLETE,
                "Framebuffer is not complete"
            );

            // Set the viewport to the size of the texture
            self.context.viewport(0, 0, width as i32, height as i32);

            // Render the scene
            let uniform_data = UniformData {
                window_offset: (0., 0.).into(),
                ..uniform_data
            };
            self.paint(uniform_data);

            // Read the pixels back from the framebuffer
            let mut pixels: Vec<u8> = vec![0; (width * height * 4) as usize];
            self.context.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(&mut pixels),
            );

            // Cleanup
            self.context.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.context.delete_framebuffer(framebuffer);
            self.context.delete_texture(texture);

            pixels
        }
    }
}

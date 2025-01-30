use glow::HasContext;

use crate::app::UniformData;

pub struct Renderer {
    program: glow::Program,
    vertex_array: glow::VertexArray,
}

impl Renderer {
    pub fn new(gl: &glow::Context) -> Self {
        use glow::HasContext as _;

        let shader_version = if cfg!(target_arch = "wasm32") {
            "#version 300 es"
        } else {
            "#version 330"
        };

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
                include_str!("frag.glsl"),
            );

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let shaders: Vec<_> = shader_sources
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader");
                    gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
                    gl.compile_shader(shader);
                    assert!(
                        gl.get_shader_compile_status(shader),
                        "Failed to compile {shader_type}: {}",
                        gl.get_shader_info_log(shader)
                    );
                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();

            gl.link_program(program);
            assert!(
                gl.get_program_link_status(program),
                "{}",
                gl.get_program_info_log(program)
            );

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");

            Self {
                program,
                vertex_array,
            }
        }
    }

    pub fn destroy(&self, gl: &glow::Context) {
        use glow::HasContext as _;
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }

    pub fn paint(&self, gl: &glow::Context, uniform_data: UniformData) {
        unsafe {
            gl.use_program(Some(self.program));
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "center").as_ref(),
                uniform_data.center.x,
                uniform_data.center.y,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "cycles").as_ref(),
                uniform_data.cycles,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "zoom").as_ref(),
                uniform_data.zoom,
            );
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "resolution").as_ref(),
                uniform_data.resolution.x,
                uniform_data.resolution.y,
            );
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "window_offset")
                    .as_ref(),
                uniform_data.window_offset.x,
                uniform_data.window_offset.y,
            );
            gl.uniform_3_f32(
                gl.get_uniform_location(self.program, "start_color")
                    .as_ref(),
                uniform_data.start_color.h,
                uniform_data.start_color.s,
                uniform_data.start_color.v,
            );
            gl.uniform_3_f32(
                gl.get_uniform_location(self.program, "end_color").as_ref(),
                uniform_data.end_color.h,
                uniform_data.end_color.s,
                uniform_data.end_color.v,
            );

            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
        }
    }

    pub fn render_to_buffer(
        &self,
        gl: &glow::Context,
        width: u32,
        height: u32,
        uniform_data: UniformData,
    ) -> Vec<u8> {
        use glow::HasContext as _;

        unsafe {
            // Create a texture to render into
            let texture = gl
                .create_texture()
                .expect("Failed to create texture for framebuffer");
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
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
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );

            // Create a framebuffer and attach the texture
            let framebuffer = gl
                .create_framebuffer()
                .expect("Failed to create framebuffer");
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );

            assert!(
                gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE,
                "Framebuffer is not complete"
            );

            // Set the viewport to the size of the texture
            gl.viewport(0, 0, width as i32, height as i32);

            // Render the scene
            let uniform_data = UniformData {
                window_offset: (0., 0.).into(),
                ..uniform_data
            };
            println!("{uniform_data:#?}");
            self.paint(gl, uniform_data);

            // Read the pixels back from the framebuffer
            let mut pixels: Vec<u8> = vec![0; (width * height * 4) as usize];
            gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(&mut pixels),
            );

            // Cleanup
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.delete_framebuffer(framebuffer);
            gl.delete_texture(texture);

            pixels
        }
    }
}

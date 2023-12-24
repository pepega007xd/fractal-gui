#[derive(Clone, Copy)]
pub struct UniformData {
    pub center: (f32, f32),
    pub zoom: f32,
    pub resolution: (f32, f32),
    pub cycles: i32,
}

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
        use glow::HasContext as _;
        unsafe {
            gl.use_program(Some(self.program));
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "center").as_ref(),
                uniform_data.center.0,
                uniform_data.center.1,
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
                uniform_data.resolution.0,
                uniform_data.resolution.1,
            );

            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
        }
    }
}

use egui::{
    color_picker, epaint::Hsva, mutex::Mutex, vec2, Color32, DragValue, Id, Pos2, Sense, Slider,
    Vec2,
};
use std::{fs::File, io::Write, sync::Arc};

use crate::renderer::{self, *};

#[derive(Clone, Copy, Debug, Default)]
pub struct UniformData {
    pub center: Vec2,
    pub zoom: f32,
    pub resolution: Vec2,
    pub window_offset: Vec2,
    pub cycles: i32,
    pub start_color: Hsva,
    pub end_color: Hsva,
}

#[derive(Default)]
enum FractalType {
    #[default]
    Mandelbrot,
    Julia,
    Custom,
}

impl PartialEq for FractalType {
    fn eq(&self, other: &Self) -> bool {
        // compare only enum variants
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

pub struct App {
    renderer: Arc<Mutex<Renderer>>,
    uniform_data: UniformData,
    aspect_ratio: Option<f32>,
    fractal_type: FractalType,
    // data is not stored in the enum => stored even when not selected
    julia_coefficient: Vec2,
    custom_fractal_function: String,
    shader_error: Option<String>,
    settings_shown: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gl = cc
            .gl
            .as_ref()
            .expect("You need to run eframe with the glow backend");

        Self {
            renderer: Arc::new(Mutex::new(Renderer::new(gl.clone()))),
            uniform_data: UniformData {
                zoom: 0.2,
                cycles: 100,
                start_color: Hsva::new(1., 0., 1., 1.),
                end_color: Hsva::new(0., 0., 0., 1.),
                ..Default::default()
            },
            custom_fractal_function: renderer::MANDELBROT_FUNC.to_string(),
            aspect_ratio: None,
            fractal_type: FractalType::Mandelbrot,
            julia_coefficient: Vec2::ZERO,
            shader_error: None,
            settings_shown: true,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn save_image(pixels_rgba: &[u8], width: u32, height: u32) {
    let mut file = File::create("./output.ppm").unwrap();
    writeln!(file, "P6").unwrap();
    println!("{} {}", width, height);
    writeln!(file, "{} {}", width, height).unwrap();
    writeln!(file, "255").unwrap();
    for rgba in pixels_rgba.chunks_exact(4) {
        file.write(&rgba[..3]).unwrap();
    }
}

#[cfg(target_arch = "wasm32")]
fn save_image(pixels_rgba: &[u8], width: u32, height: u32) {
    use js_sys::Uint8Array;
    use web_sys::js_sys;
    use web_sys::js_sys::Array;
    use web_sys::File;
    use web_sys::FilePropertyBag;
    use web_sys::Url;
    let mut ppm_data = Vec::new();

    writeln!(ppm_data, "P6").unwrap();
    writeln!(ppm_data, "{} {}", width, height).unwrap();
    writeln!(ppm_data, "255").unwrap();

    for rgba in pixels_rgba.chunks_exact(4) {
        ppm_data.extend_from_slice(&rgba[..3]);
    }

    let u8array = Uint8Array::from(ppm_data.as_slice());
    let array = Array::new();
    array.push(&u8array.buffer());

    let mut properties = FilePropertyBag::new();
    properties.type_("application/octet-stream");
    let file =
        File::new_with_u8_array_sequence_and_options(&array, "output.ppm", &properties).unwrap();

    let url = Url::create_object_url_with_blob(&file).unwrap();

    web_sys::window().unwrap().open_with_url(&url).unwrap();

    Url::revoke_object_url(&url).unwrap();
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if !self.settings_shown {
            egui::Area::new(Id::new("settings_button")).show(ctx, |ui| {
                if ui.button("open settings").clicked() {
                    self.settings_shown = true;
                }
            });
        }

        egui::SidePanel::new(egui::panel::Side::Left, "side_panel").show_animated(
            ctx,
            self.settings_shown,
            |ui| {
                ui.heading("Settings");
                if ui.button("hide").clicked() {
                    self.settings_shown = false;
                }
                ui.separator();

                ui.label("Iterations");
                ui.add(Slider::new(&mut self.uniform_data.cycles, 1..=5000).logarithmic(true));
                ui.separator();

                ui.label("Start Color");
                color_picker::color_edit_button_hsva(
                    ui,
                    &mut self.uniform_data.start_color,
                    color_picker::Alpha::Opaque,
                );
                ui.separator();

                ui.label("End Color");
                color_picker::color_edit_button_hsva(
                    ui,
                    &mut self.uniform_data.end_color,
                    color_picker::Alpha::Opaque,
                );
                ui.separator();

                ui.label("Aspect ratio");
                match () {
                    _ if ui.radio(self.aspect_ratio.is_none(), "dynamic").clicked() => {
                        self.aspect_ratio = None
                    }
                    _ if ui
                        .radio(self.aspect_ratio == Some(16. / 9.), "16:9")
                        .clicked() =>
                    {
                        self.aspect_ratio = Some(16. / 9.)
                    }
                    _ => (),
                }
                ui.separator();

                ui.label("Fractal type");
                if ui
                    .radio_value(
                        &mut self.fractal_type,
                        FractalType::Mandelbrot,
                        "Mandelbrot",
                    )
                    .clicked()
                {
                    let _ = self
                        .renderer
                        .lock()
                        .set_fractal_function(renderer::MANDELBROT_FUNC);
                }

                if ui
                    .radio_value(&mut self.fractal_type, FractalType::Julia, "Julia set")
                    .clicked()
                {
                    let _ = self
                        .renderer
                        .lock()
                        .set_fractal_function(renderer::JULIA_FUNC);
                };

                ui.radio_value(&mut self.fractal_type, FractalType::Custom, "Custom");

                ui.separator();

                if let FractalType::Julia = self.fractal_type {
                    ui.label("Julia set constant");

                    let range = (-2.0)..=(2.0);
                    ui.add(
                        DragValue::new(&mut self.julia_coefficient.x)
                            .range(range.clone())
                            .speed(0.01),
                    );
                    ui.add(
                        DragValue::new(&mut self.julia_coefficient.y)
                            .range(range)
                            .speed(0.01),
                    );
                    ui.separator();
                }

                if ui.button("Take screenshot").clicked() {
                    let uniform_data = self.uniform_data.clone();

                    let (width, height) = (
                        uniform_data.resolution.x as u32,
                        uniform_data.resolution.y as u32,
                    );
                    let output = self
                        .renderer
                        .lock()
                        .render_to_buffer(width, height, uniform_data);
                    save_image(&output, width, height);
                };
            },
        );

        if let FractalType::Custom = &mut self.fractal_type {
            egui::TopBottomPanel::new(
                egui::panel::TopBottomSide::Bottom,
                "Custom fractal function",
            )
            .show(ctx, |ui| {
                if let Some(error) = &self.shader_error {
                    ui.colored_label(
                        Color32::LIGHT_RED,
                        "Error while compiling the shader: \n\n".to_string() + error,
                    );
                }

                ui.heading("Custom fractal function");

                let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx());

                let mut layouter = |ui: &egui::Ui, str: &str, wrap_width: f32| {
                    let mut layout_job =
                        egui_extras::syntax_highlighting::highlight(ui.ctx(), &theme, str, "c");
                    layout_job.wrap.max_width = wrap_width;
                    ui.fonts(|f| f.layout_job(layout_job))
                };
                let response = ui.add(
                    egui::TextEdit::multiline(&mut self.custom_fractal_function)
                        .font(egui::TextStyle::Monospace) // for cursor height
                        .code_editor()
                        .desired_rows(10)
                        .lock_focus(true)
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter),
                );
                if response.changed() {
                    if let Err(error) = self
                        .renderer
                        .lock()
                        .set_fractal_function(&self.custom_fractal_function)
                    {
                        self.shader_error = Some(error);
                    } else {
                        self.shader_error = None;
                    }
                }
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let (fractal_rect, response) =
                ui.allocate_exact_size(ui.max_rect().size(), Sense::drag());
            let rect_size = fractal_rect.size();
            let drag = response.drag_delta() / rect_size;

            let ppp = ctx.pixels_per_point();

            self.uniform_data.resolution = (rect_size * ppp).into();
            self.uniform_data.window_offset = (fractal_rect.left_top() * ppp).to_vec2();
            self.uniform_data.center -= drag;

            let center = self.uniform_data.center;
            let mut window_correction =
                ctx.screen_rect().left_bottom() - fractal_rect.left_bottom();
            window_correction.x *= -1.;
            let screen_to_fractal_coords = |pos: Pos2| {
                let pos = (pos.to_vec2() - window_correction) / rect_size;
                let pos = pos - vec2(0.5, 0.5);
                pos + center
            };

            ctx.input(|e| {
                let zoom = e.zoom_delta();
                if let Some(pointer) = e.pointer.latest_pos() {
                    let pointer = screen_to_fractal_coords(pointer);
                    self.uniform_data.zoom *= zoom;
                    self.uniform_data.center += pointer * (zoom - 1.);
                }
            });

            let uniform_data = self.uniform_data.clone();

            let renderer = self.renderer.clone();

            let callback = egui::PaintCallback {
                rect: fractal_rect,
                callback: Arc::new(egui_glow::CallbackFn::new(move |_, _| {
                    renderer.lock().paint(uniform_data);
                })),
            };
            ui.painter().add(callback);
        });
    }

    fn on_exit(&mut self, _: Option<&glow::Context>) {
        self.renderer.lock().destroy();
    }
}

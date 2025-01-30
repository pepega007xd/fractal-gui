use egui::{
    color_picker, epaint::Hsva, mutex::Mutex, vec2, ColorImage, Id, ImageSource, PaintCallback,
    Pos2, Rect, Sense, Slider, Vec2,
};
use std::{fs::File, io::Write, sync::Arc};

use crate::renderer::*;

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

pub struct App {
    /// Behind an `Arc<Mutex<…>>` so we can pass it to [`egui::PaintCallback`] and paint later.
    renderer: Arc<Mutex<Renderer>>,
    uniform_data: UniformData,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gl = cc
            .gl
            .as_ref()
            .expect("You need to run eframe with the glow backend");
        Self {
            renderer: Arc::new(Mutex::new(Renderer::new(gl))),
            uniform_data: UniformData {
                zoom: 0.2,
                cycles: 100,
                start_color: Hsva::new(1., 0., 1., 1.),
                end_color: Hsva::new(0., 0., 0., 1.),
                ..Default::default()
            },
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::SidePanel::new(egui::panel::Side::Left, "side_panel").show(ctx, |ui| {
            ui.heading("Settings");
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

            if ui.button("Take screenshot").clicked() {
                let renderer = self.renderer.clone();
                let uniform_data = self.uniform_data.clone();

                let (width, height) = (
                    uniform_data.resolution.x as u32,
                    uniform_data.resolution.y as u32,
                );
                let output = renderer.lock().render_to_buffer(
                    frame.gl().unwrap(),
                    width,
                    height,
                    uniform_data,
                );
                let mut file = File::create("./output.ppm").unwrap();
                writeln!(file, "P6").unwrap();
                println!("{} {}", width, height);
                writeln!(file, "{} {}", width, height).unwrap();
                writeln!(file, "255").unwrap();
                for rgba in output.chunks_exact(4) {
                    file.write(&rgba[..3]).unwrap();
                }
            };
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let (fractal_rect, response) =
                ui.allocate_exact_size((500., 500.).into(), Sense::drag());
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

            let renderer = self.renderer.clone();
            let uniform_data = self.uniform_data.clone();

            let callback = egui::PaintCallback {
                rect: fractal_rect,
                callback: Arc::new(egui_glow::CallbackFn::new(move |_, painter| {
                    renderer.lock().paint(painter.gl(), uniform_data);
                })),
            };
            ui.painter().add(callback);
        });
    }

    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            self.renderer.lock().destroy(gl);
        }
    }
}

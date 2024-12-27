use egui::{mutex::Mutex, Id, Pos2, Sense, Slider};
use std::sync::Arc;

use crate::renderer::*;

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
                center: (0., 0.).into(),
                zoom: 1.,
                resolution: (1., 1.).into(),
                window_offset: (0., 0.).into(),
                cycles: 100,
            },
        }
    }

    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        // Clone locals so we can move them into the paint callback
        let renderer = self.renderer.clone();
        let uniform_data = self.uniform_data.clone();

        let callback = egui::PaintCallback {
            rect: ui.max_rect(),
            callback: Arc::new(egui_glow::CallbackFn::new(move |_, painter| {
                renderer.lock().paint(painter.gl(), uniform_data);
            })),
        };
        ui.painter().add(callback);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::new(egui::panel::Side::Left, "side_panel").show(ctx, |ui| {
            ui.add(Slider::new(&mut self.uniform_data.cycles, 1..=500));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // adjust position by dragging
            let max_rect = ui.max_rect();
            let uniform_data = self.uniform_data;
            let rect_size = max_rect.size();
            let drag = ui
                .interact(max_rect, Id::new(0), Sense::click_and_drag())
                .drag_delta()
                / rect_size;

            let ppp = ctx.pixels_per_point();

            self.uniform_data.resolution = (rect_size * ppp).into();
            self.uniform_data.window_offset = (max_rect.left_top() * ppp).to_vec2();
            self.uniform_data.center -= drag;

            let screen_to_fractal_coords = |pos: Pos2| {
                let pos = (pos - max_rect.left_top()) / rect_size;
                let pos = pos - (0.5, 0.5).into();
                pos + uniform_data.center
            };

            // egui::Window::new("Debug").show(ctx, |ui| {
            //     ui.heading(format!("drag: {drag}"));
            //     ui.heading(format!("max_rect: {max_rect}"));
            //     ui.heading(format!("rect_size: {rect_size}"));
            //     let pointer_pos = ctx.pointer_latest_pos();
            //     ui.heading(format!("pointer_pos: {pointer_pos:?}"));
            //     ui.heading(format!("pointer: {pointer:?}"));
            //     ui.heading(format!("uniform_data: {:#?}", self.uniform_data));
            // });

            ctx.input(|e| {
                let zoom = e.zoom_delta();
                if let Some(pointer) = e.pointer.latest_pos() {
                    let pointer = screen_to_fractal_coords(pointer);
                    self.uniform_data.zoom *= zoom;
                    self.uniform_data.center += pointer * (zoom - 1.);
                }
            });

            self.custom_painting(ui);
        });
    }

    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            self.renderer.lock().destroy(gl);
        }
    }
}

use egui::{mutex::Mutex, Event, Id, Pos2, Rect, Sense, Slider, Vec2};
use std::{ops::Range, sync::Arc};

use crate::renderer::*;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct TemplateApp();

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
                center: (0., 0.),
                zoom: 1.,
                resolution: (1., 1.),
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
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                // adjust position by dragging
                let max_rect = ui.max_rect();
                let rect_size = max_rect.size();
                let drag = ui
                    .interact(max_rect, Id::new(0), Sense::drag())
                    .drag_delta()
                    / rect_size;
                self.uniform_data.resolution = (rect_size.x, rect_size.y);
                self.uniform_data.center.0 -= drag.x / self.uniform_data.zoom;
                self.uniform_data.center.1 += drag.y / self.uniform_data.zoom;

                // calculate mouse pointer location in fractal coordinates
                let pointer = ctx.pointer_latest_pos().unwrap_or(Pos2::default());

                let pointer = (pointer - Pos2::ZERO) / (max_rect.size());
                let pointer = (pointer - (0.5, 0.5).into()) / self.uniform_data.zoom;
                println!("{}  {}", pointer.x, pointer.y);

                // adjust zoom by scrolling
                ctx.input(|e| {
                    e.events.iter().for_each(|e| {
                        if let Event::Scroll(s) = e {
                            let zoom_scale = 0.01;
                            let scroll = s.y;

                            self.uniform_data.zoom *= 1. - (scroll * zoom_scale);
                            self.uniform_data.center.0 -= scroll * pointer.x * zoom_scale;
                            self.uniform_data.center.1 += scroll * pointer.y * zoom_scale;
                        }
                        // for touchscreens maybe?
                        if let Event::Zoom(z) = e {
                            self.uniform_data.zoom *= 1. + (z * 0.01);
                        }
                    })
                });

                // canvas for drawing the fractal itself
                self.custom_painting(ui);
            });
        });
    }

    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            self.renderer.lock().destroy(gl);
        }
    }
}

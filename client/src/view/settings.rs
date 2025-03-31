use egui::{DragValue, Slider};

use crate::prelude::*;

use super::DndTabImpl;

pub struct Settings {
    pixels_per_point: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pixels_per_point: 1.2,
        }
    }
}

impl DndTabImpl for Settings {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        _state: &DndState,
        _commands: &mut crate::listener::CommandQueue,
    ) {
        egui::Grid::new("settings").show(ui, |ui| {
            ui.label("UI Scale: ");
            if DragValue::new(&mut self.pixels_per_point)
                .range(0.5..=3.0)
                .update_while_editing(false)
                .ui(ui)
                .changed()
            {
                ui.ctx().set_pixels_per_point(self.pixels_per_point);
            }

            ui.end_row();
        });
    }

    fn title(&self) -> String {
        "Settings".to_owned()
    }
}

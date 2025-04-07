use egui::{DragValue, Frame, Label, ScrollArea, Slider};
use itertools::Itertools;

use crate::prelude::*;

use super::DndTabImpl;

pub struct Backpack {}

impl Default for Backpack {
    fn default() -> Self {
        Self {}
    }
}

impl DndTabImpl for Backpack {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &DndState,
        _commands: &mut crate::listener::CommandQueue,
    ) {
        Frame::canvas(ui.style()).show(ui, |ui| {
            let board = state.client_board.lock().unwrap();

            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                for (category, pieces) in board
                    .backpack_set
                    .iter()
                    .into_group_map_by(|piece| &piece.category)
                {
                    //ui.add_sized([ui.available_width(), 20.0], egui);
                    ui.label(format!("\t{}", category));

                    for piece in pieces {
                        ui.label(&piece.piece.name);
                    }
                }
            });
        });
    }

    fn title(&self) -> String {
        "Backpack".to_owned()
    }
}

use crate::{prelude::*, state::character::commands::RefreshCharacter};
use egui::{
    collapsing_header, popup_below_widget, text::LayoutJob, tooltip_id, Align, Button,
    CentralPanel, CollapsingHeader, Color32, DragValue, Frame, Label, Margin, Resize, RichText,
    TopBottomPanel, Vec2, Widget,
};

use crate::{
    listener::CommandQueue,
    state::{character::commands::UseItem, DndState},
};

use super::DndTabImpl;

pub struct StatWidget {
    name: String,
    value: i16,
}

impl StatWidget {
    pub fn new(name: impl ToString, value: i16) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }
}

impl egui::Widget for StatWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        Frame::none()
            .stroke(egui::Stroke {
                width: 1.0,
                color: Color32::LIGHT_GRAY,
            })
            .inner_margin(Margin::same(5.0))
            .show(ui, |ui| {
                Resize::default()
                    .resizable(false)
                    .default_size([40.0, 40.0])
                    .show(ui, |ui| {
                        ui.vertical_centered_justified(|ui| {
                            ui.label(self.name);
                            ui.heading(self.value.to_string());
                        });
                    });
            })
            .response
    }
}

#[derive(Default)]
pub struct Character;

impl DndTabImpl for Character {
    fn ui(&mut self, ui: &mut egui::Ui, state: &DndState, commands: &mut CommandQueue) {
        let char = &state.character.character;

        ui.horizontal(|ui| {
            ui.heading(&char.name);
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Refresh").clicked() {
                    commands.add(RefreshCharacter);
                }
            })
        });

        ui.add_space(4.0);

        ui.label(RichText::new(format!("\"{}\"", char.tagline)).italics());

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            StatWidget::new("CHR", char.chr).ui(ui);
            StatWidget::new("STR", char.str).ui(ui);
            StatWidget::new("WIS", char.wis).ui(ui);
            StatWidget::new("INT", char.int).ui(ui);
            StatWidget::new("DEX", char.dex).ui(ui);
            StatWidget::new("CON", char.con).ui(ui);
        });
    }

    fn title(&self) -> String {
        "Character".to_owned()
    }
}

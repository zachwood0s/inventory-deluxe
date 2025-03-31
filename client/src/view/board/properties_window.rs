use common::board::{BoardPiece, BoardPieceData, PlayerPieceData};
use egui::{Color32, DragValue, Ui, Widget, Window};

pub trait PropertiesDisplay {
    fn display_props(&mut self, ui: &mut Ui);
}

impl PropertiesDisplay for BoardPiece {
    fn display_props(&mut self, ui: &mut Ui) {
        Window::new("Properties").show(ui.ctx(), |ui| {
            ui.colored_label(Color32::DARK_GRAY, format!("Piece id: {:?}", self.id));

            let mut width = self.rect.width();
            let mut height = self.rect.height();
            ui.horizontal(|ui| {
                DragValue::new(&mut width)
                    .prefix("w: ")
                    .range(0.5..=100.0)
                    .ui(ui);

                DragValue::new(&mut height)
                    .prefix("h: ")
                    .range(0.5..=100.0)
                    .ui(ui);
            });

            self.rect.set_width(width);
            self.rect.set_height(height);

            match &mut self.data {
                BoardPieceData::Player(data) => data.display_props(ui),
                BoardPieceData::None => {}
            }
        });
    }
}

impl PropertiesDisplay for PlayerPieceData {
    fn display_props(&mut self, ui: &mut Ui) {
        todo!()
    }
}

use egui::{Ui, Window};

use crate::state::board::pieces::{
    BoardPiece, InternalDecorationData, MapPieceData, PlayerPieceData,
};

pub trait PropertiesDisplay {
    fn display_props(&self, ui: &mut Ui);
}

impl<T> PropertiesDisplay for BoardPiece<T>
where
    T: PropertiesDisplay,
{
    fn display_props(&self, ui: &mut Ui) {
        Window::new("Properties").show(ui.ctx(), |ui| {
            self.data.display_props(ui);
        });
    }
}

impl PropertiesDisplay for PlayerPieceData {
    fn display_props(&self, ui: &mut Ui) {
        todo!()
    }
}

impl PropertiesDisplay for MapPieceData {
    fn display_props(&self, ui: &mut Ui) {
        todo!()
    }
}

impl PropertiesDisplay for InternalDecorationData {
    fn display_props(&self, ui: &mut Ui) {
        todo!()
    }
}

use std::sync::mpsc::Receiver;

use common::message::DndMessage;
use message_io::events::EventSender;

use crate::listener::Signal;

use super::DndTabImpl;

#[derive(Default)]
pub struct Board;

impl DndTabImpl for Board {
    fn ui(&mut self, ui: &mut egui::Ui, tx: &EventSender<Signal>, rx: &Receiver<DndMessage>) {
        ui.label("Board goes here!");
    }

    fn title(&self) -> String {
        "Board".to_owned()
    }
}

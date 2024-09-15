mod board;
mod chat;

use std::sync::mpsc::Receiver;

pub use board::*;
pub use chat::*;
use common::message::DndMessage;
use egui_dock::{NodeIndex, SurfaceIndex};
use message_io::events::EventSender;

use crate::listener::Signal;

pub trait DndTabImpl {
    fn ui(&mut self, ui: &mut egui::Ui, tx: &EventSender<Signal>, rx: &Receiver<DndMessage>);
    fn title(&self) -> String;
}

pub struct DndTab {
    pub kind: Box<dyn DndTabImpl>,
    pub surface: SurfaceIndex,
    pub node: NodeIndex,
}

impl DndTab {
    pub fn from_tab<T: DndTabImpl + 'static>(
        tab: T,
        surface: SurfaceIndex,
        node: NodeIndex,
    ) -> Self {
        Self {
            kind: Box::new(tab),
            surface,
            node,
        }
    }

    pub fn title(&self) -> String {
        self.kind.title()
    }
}

pub struct TabViewer<'a> {
    pub added_nodes: &'a mut Vec<DndTab>,
    pub tx: &'a EventSender<Signal>,
    pub rx: &'a Receiver<DndMessage>,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = DndTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        tab.kind.ui(ui, &self.tx, &self.rx);
    }

    fn add_popup(&mut self, ui: &mut egui::Ui, surface: SurfaceIndex, node: NodeIndex) {
        ui.set_min_width(120.0);
        ui.style_mut().visuals.button_frame = false;

        if ui.button("Chat").clicked() {
            self.added_nodes
                .push(DndTab::from_tab(Chat::default(), surface, node))
        }
        if ui.button("Game Board").clicked() {
            self.added_nodes
                .push(DndTab::from_tab(Board, surface, node))
        }
    }
}

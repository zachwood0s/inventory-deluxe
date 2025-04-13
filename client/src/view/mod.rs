mod abilities;
mod backpack;
pub mod board;
mod character;
pub mod character_sheet;
mod chat;
mod items;
pub mod multi_select;
mod settings;

pub use abilities::*;
use backpack::Backpack;
pub use board::*;
pub use character::*;
use character_sheet::{CharacterSheet, CharacterSheetWindow};
pub use chat::*;
use egui::Color32;
use egui_dock::{NodeIndex, SurfaceIndex};
pub use items::*;

use crate::{listener::CommandQueue, state::DndState};

use self::settings::Settings;

pub trait DndTabImpl {
    fn ui(&mut self, ui: &mut egui::Ui, state: &DndState, commands: &mut CommandQueue);
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
    pub state: &'a DndState,
    pub network: CommandQueue<'a>,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = DndTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.visuals_mut().code_bg_color = Color32::TRANSPARENT;
        tab.kind.ui(ui, self.state, &mut self.network);

        if let Some(char) = self.state.data.get_character(&self.state.owned_user()) {
            CharacterSheetWindow {
                sheet: CharacterSheet::new(char, &mut self.network),
            }
            .ui(ui);
        }
    }

    fn add_popup(&mut self, ui: &mut egui::Ui, surface: SurfaceIndex, node: NodeIndex) {
        ui.set_min_width(120.0);
        ui.style_mut().visuals.button_frame = false;

        if ui.button("Chat").clicked() {
            self.added_nodes
                .push(DndTab::from_tab(Chat::default(), surface, node))
        }
        if ui.button("Board").clicked() {
            self.added_nodes
                .push(DndTab::from_tab(UiBoardState::default(), surface, node))
        }
        if ui.button("Character").clicked() {
            self.added_nodes
                .push(DndTab::from_tab(Character::default(), surface, node))
        }
        if ui.button("Abilities").clicked() {
            self.added_nodes
                .push(DndTab::from_tab(Abilities, surface, node))
        }
        if ui.button("Items").clicked() {
            self.added_nodes
                .push(DndTab::from_tab(Items::default(), surface, node))
        }
        if ui.button("Settings").clicked() {
            self.added_nodes
                .push(DndTab::from_tab(Settings::default(), surface, node))
        }
        if ui.button("Backpack").clicked() {
            self.added_nodes
                .push(DndTab::from_tab(Backpack::default(), surface, node))
        }
    }
}

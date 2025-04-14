use std::{rc::Rc, sync::Arc};

use common::data_store::CharacterStorage;
use egui_dock::TabViewer;

use crate::{listener::CommandQueue, state::DndState};

pub trait CharacterTabImpl: Send + Sync {
    fn ui(&self, ui: &mut egui::Ui, ctx: CharacterCtx);
    fn title(&self) -> &str;
}

pub type CharacterTab = Arc<dyn CharacterTabImpl>;

pub struct CharacterCtx<'a, 'q> {
    pub character: &'a CharacterStorage,
    pub state: &'a DndState,
    pub commands: &'a mut CommandQueue<'q>,
}

pub struct CharacterTabs<'a, 'q> {
    pub character: &'a CharacterStorage,
    pub state: &'a DndState,
    pub commands: &'a mut CommandQueue<'q>,
}

impl TabViewer for CharacterTabs<'_, '_> {
    type Tab = CharacterTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let ctx = CharacterCtx {
            character: self.character,
            state: self.state,
            commands: self.commands,
        };

        tab.ui(ui, ctx);
    }
}

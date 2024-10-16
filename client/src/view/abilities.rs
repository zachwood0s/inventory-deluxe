use super::DndTabImpl;

#[derive(Default)]
pub struct Abilities;

impl DndTabImpl for Abilities {
    fn ui(&mut self, ui: &mut egui::Ui, state: &crate::prelude::DndState, commands: &mut crate::listener::CommandQueue) {
        for a in state.character.abilities.iter() {
            ui.label(&a.name);
        }

    }

    fn title(&self) -> String {
        "Abilities".to_owned()
    }
}
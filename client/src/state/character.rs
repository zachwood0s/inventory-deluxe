use common::{message::DndMessage, Item};

#[derive(Default)]
pub struct CharacterState {
    pub items: Vec<Item>,
}

impl CharacterState {
    pub fn process(&mut self, message: &DndMessage) {
        #[allow(clippy::single_match)]
        match message {
            DndMessage::ItemList(items) => {
                self.items = items.clone();
            }
            _ => {}
        }
    }
}

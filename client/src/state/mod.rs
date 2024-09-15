use common::{message::DndMessage, User};

pub mod chat;

#[derive(Default)]
pub struct DndState {
    pub chat: chat::ChatState,
    pub user: Option<User>,
}

impl DndState {
    pub fn process(&mut self, message: DndMessage) {
        self.chat.process(&message);
    }
}

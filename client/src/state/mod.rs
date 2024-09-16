use common::{message::DndMessage, User};

pub mod character;
pub mod chat;

#[derive(Default)]
pub struct DndState {
    pub chat: chat::ChatState,
    pub character: character::CharacterState,
    pub user: Option<User>,
}

impl DndState {
    pub fn process(&mut self, message: DndMessage) {
        self.chat.process(&message);
        self.character.process(&message);
    }

    pub fn owned_user(&self) -> User {
        self.user.clone().unwrap()
    }
}

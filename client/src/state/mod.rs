use common::{message::DndMessage, User};

pub mod board;
pub mod character;
pub mod chat;

#[derive(Default)]
pub struct DndState {
    pub board: board::BoardState,
    pub chat: chat::ChatState,
    pub character: character::CharacterState,
    pub user: Option<User>,
}

impl DndState {
    pub fn process(&mut self, message: DndMessage) {
        self.chat.process(&message);
        self.character.process(&message);
        self.board.process(&message);
    }

    pub fn owned_user(&self) -> User {
        self.user.clone().unwrap()
    }
}

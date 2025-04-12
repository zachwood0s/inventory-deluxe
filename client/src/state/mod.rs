use board::ClientBoard;
use common::{data_store::DataStore, message::DndMessage, User};

pub mod abilities;
pub mod backpack;
pub mod board;
pub mod character;
pub mod chat;

#[derive(Default)]
pub struct DndState {
    pub data: DataStore,
    pub client_board: ClientBoard,
    pub chat: chat::ChatState,
    pub character: character::CharacterState,
    pub user: Option<User>,
}

impl DndState {
    pub fn process(&mut self, message: DndMessage) {
        self.chat.process(&message);
        self.character.process(&message);
        self.client_board.process(&message);

        match message {
            DndMessage::DataMessage(msg) => self.data.handle_message(msg),
            _ => {}
        }
    }

    pub fn owned_user(&self) -> User {
        self.user.clone().unwrap()
    }
}

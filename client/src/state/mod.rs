use board::ClientBoard;
use common::{data_store::DataStore, message::DndMessage, AbilityId, User};

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

    #[deprecated]
    pub character: character::CharacterState,
    pub user: Option<User>,

    pub ability_edit: Option<AbilityId>,
}

impl DndState {
    pub fn process(&mut self, message: DndMessage) {
        self.chat.process(&message);
        self.character.process(&message);
        self.client_board.process(&message);

        if let DndMessage::DataMessage(msg) = message {
            self.data.handle_message(msg);
        }
    }

    pub fn owned_user(&self) -> User {
        self.user.clone().unwrap()
    }
}

pub mod commands {
    use common::AbilityId;
    use message_io::events::EventSender;

    use crate::prelude::{Command, Signal};

    use super::DndState;

    pub struct EditAbility(pub AbilityId);

    impl Command for EditAbility {
        fn execute(self: Box<Self>, state: &mut DndState, _: &EventSender<Signal>) {
            state.ability_edit = Some(self.0);
        }
    }
}

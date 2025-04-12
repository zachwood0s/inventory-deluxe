use std::sync::{Arc, Mutex};

use common::board::BoardData;

use crate::prelude::*;

#[derive(Default, Debug, Clone, derive_more::DerefMut, derive_more::Deref)]
pub struct ClientBoard {
    server_board: Arc<Mutex<BoardData>>,
}

impl ClientBoard {
    pub fn process(&mut self, message: &DndMessage) {
        let DndMessage::BoardMessage(msg) = message else {
            return;
        };

        let mut board = self.server_board.lock().unwrap();
        board.handle_message(msg.clone());
    }
}

pub mod commands {
    use common::board::{BoardMessage, BoardPiece, PieceId};

    use super::*;

    pub struct AddOrUpdatePiece(pub BoardPiece);

    impl Command for AddOrUpdatePiece {
        fn execute(self: Box<Self>, _: &mut DndState, tx: &EventSender<Signal>) {
            let Self(piece) = *self;

            tx.send(DndMessage::BoardMessage(BoardMessage::AddOrUpdatePiece(piece)).into())
        }
    }

    pub struct DeletePiece(pub PieceId);
    impl Command for DeletePiece {
        fn execute(self: Box<Self>, _state: &mut DndState, tx: &EventSender<Signal>) {
            tx.send(DndMessage::BoardMessage(BoardMessage::DeletePiece(self.0)).into())
        }
    }
}

use std::sync::{Arc, Mutex};

use common::board::BoardData;

use crate::prelude::*;

pub mod commands {
    use common::board::{BoardPiece, PieceId};

    use super::*;

    pub struct StorePlayerPiece(pub BackpackPiece);

    impl Command for StorePlayerPiece {
        fn execute(self: Box<Self>, _: &mut DndState, tx: &EventSender<Signal>) {
            let Self(piece) = *self;

            tx.send(DndMessage::BoardMessage(BoardMessage::StoreBackpackPiece(piece)).into());
        }
    }
}

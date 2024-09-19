use egui::{ahash::HashMap, Rounding};
use uuid::Uuid;

use crate::prelude::*;

pub struct PlayerPiece {
    pub rect: Rect,
    pub image: Option<TextureId>,
    pub color: Option<Color32>,
    pub dragged: bool,
    pub selected: bool,
}

impl PlayerPiece {
    pub fn draw_shape(&self, to_screen: RectTransform) -> egui::Shape {
        let transformed = to_screen.transform_rect(self.rect);
        if self.dragged {
            egui::Shape::rect_filled(transformed, Rounding::ZERO, Color32::GREEN)
        } else if self.selected {
            egui::Shape::rect_filled(transformed, Rounding::ZERO, Color32::RED)
        } else {
            egui::Shape::rect_filled(transformed, Rounding::ZERO, Color32::WHITE)
        }
    }

    fn drop(&mut self) {
        let center = self.rect.center();
        let center = (center / BoardState::GRID_SIZE).round() * BoardState::GRID_SIZE;
        self.rect.set_center(center);
        self.dragged = false;
    }

    fn drag(&mut self) {
        self.dragged = true;
    }
}

#[derive(Default)]
pub struct BoardState {
    pub players: HashMap<uuid::Uuid, PlayerPiece>,
    pub dragged_id: Option<uuid::Uuid>,
    pub selected_id: Option<uuid::Uuid>,
}

impl BoardState {
    const GRID_SIZE: f32 = 0.1;

    pub fn process(&mut self, message: &DndMessage) {
        let DndMessage::BoardMessage(msg) = message else {
            return;
        };

        match msg {
            BoardMessage::AddPlayerPiece(uuid, player) => {
                self.players.insert(
                    *uuid,
                    PlayerPiece {
                        rect: Rect::from_center_size(player.position, player.size),
                        image: None,
                        color: None,
                        dragged: false,
                        selected: false,
                    },
                );
            }
            BoardMessage::UpdatePlayerLocation(uuid, new_pos) => {
                if let Some(player) = self.players.get_mut(uuid) {
                    player.rect.set_center(*new_pos)
                }
            }
            BoardMessage::DeletePlayerPiece(uuid) => {
                self.players.remove(uuid);
            }
        }
    }

    pub fn get_player_mut(&mut self, uuid: &Uuid) -> Option<&mut PlayerPiece> {
        self.players.get_mut(uuid)
    }

    pub fn get_dragged_player_mut(&mut self) -> Option<&mut PlayerPiece> {
        self.dragged_id.and_then(|x| self.get_player_mut(&x))
    }

    pub fn unselect_other_player(&mut self) {
        for player in self.players.values_mut() {
            if player.selected {
                player.selected = false;
            }
        }
        self.selected_id = None
    }

    pub fn find_selected_player_id(&self, pointer_pos: Pos2) -> Option<&Uuid> {
        for (id, player) in self.players.iter() {
            if player.rect.contains(pointer_pos) {
                return Some(id);
            }
        }
        None
    }
}

pub mod commands {
    use super::*;
    use crate::{prelude::*, view::Board};

    pub struct SetPlayerPosition {
        id: Uuid,
        new_pos: Pos2,
    }

    impl SetPlayerPosition {
        pub fn new(id: Uuid, new_pos: Pos2) -> Self {
            Self { id, new_pos }
        }
    }

    impl Command for SetPlayerPosition {
        fn execute(self: Box<Self>, _state: &mut DndState, tx: &EventSender<Signal>) {
            tx.send(
                DndMessage::BoardMessage(BoardMessage::UpdatePlayerLocation(self.id, self.new_pos))
                    .into(),
            );
        }
    }

    pub struct Drop;
    impl Command for Drop {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            if let (Some(id), Some(piece)) =
                (state.board.dragged_id, state.board.get_dragged_player_mut())
            {
                piece.drop();

                tx.send(
                    DndMessage::BoardMessage(BoardMessage::UpdatePlayerLocation(
                        id,
                        piece.rect.center(),
                    ))
                    .into(),
                );

                state.board.dragged_id = None;
            }
        }
    }

    pub struct Drag(pub Uuid);
    impl Command for Drag {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            if let Some(player) = state.board.get_player_mut(&self.0) {
                player.drag();
                state.board.dragged_id = Some(self.0);
            }
        }
    }

    pub struct Select(pub Option<Uuid>);
    impl Command for Select {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            state.board.unselect_other_player();
            if let Some((idx, player)) = self
                .0
                .and_then(|idx| state.board.get_player_mut(&idx).map(|p| (idx, p)))
            {
                player.selected = true;
                state.board.selected_id = Some(idx);
            }
        }
    }

    pub struct AddPiece(pub Pos2, pub Vec2);
    impl Command for AddPiece {
        fn execute(self: Box<Self>, _state: &mut DndState, tx: &EventSender<Signal>) {
            let center = self.0;
            let center = (center / BoardState::GRID_SIZE).round() * BoardState::GRID_SIZE;
            let uuid = Uuid::new_v4();
            let size = self.1 * Board::GRID_SIZE;

            tx.send(
                DndMessage::BoardMessage(BoardMessage::AddPlayerPiece(
                    uuid,
                    common::DndPlayerPiece {
                        position: center,
                        size,
                        image_url: None,
                        color: None,
                    },
                ))
                .into(),
            )
        }
    }

    pub struct DeletePiece(pub Uuid);
    impl Command for DeletePiece {
        fn execute(self: Box<Self>, _state: &mut DndState, tx: &EventSender<Signal>) {
            tx.send(DndMessage::BoardMessage(BoardMessage::DeletePlayerPiece(self.0)).into())
        }
    }
}

use std::cmp;

use common::SortingLayer;
use egui::{ahash::HashMap, Image, Painter, Rounding, Stroke, TextureHandle, TextureOptions};
use itertools::Itertools;
use uuid::Uuid;

use crate::{prelude::*, view::Board};

pub struct PlayerPiece {
    pub rect: Rect,
    pub image_url: Option<String>,
    pub color: Option<Color32>,
    pub dragged: bool,
    pub selected: bool,
    pub sorting_layer: SortingLayer,
    pub visible_by: Vec<String>,
    pub locked: bool,
}

impl PlayerPiece {
    pub fn draw_shape(&self, ui: &mut egui::Ui, painter: &Painter, to_screen: RectTransform) {
        let transformed = to_screen.transform_rect(self.rect);

        let alpha = if self.dragged { u8::MAX / 10 } else { u8::MAX };

        if let Some(url) = &self.image_url {
            Image::new(url)
                .texture_options(
                    TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear)),
                )
                .tint(Color32::from_white_alpha(alpha))
                .paint_at(ui, transformed);
        } else {
            painter.rect_filled(
                transformed,
                Rounding::ZERO,
                Color32::from_white_alpha(alpha),
            );
        }

        if self.selected {
            painter.rect_stroke(
                transformed,
                Rounding::ZERO,
                Stroke::new(3.0, Color32::LIGHT_RED),
            );
        }
    }

    fn drop(&mut self) {
        let pos = commands::snap_to_grid(self.rect.left_top());
        self.rect = Rect::from_two_pos(pos, pos + self.rect.size());
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
                        rect: Rect::from_two_pos(player.position, player.position + player.size),
                        image_url: player.image_url.clone(),
                        color: None,
                        dragged: false,
                        selected: false,
                        sorting_layer: player.sorting_layer,
                        visible_by: player.visible_by.clone(),
                        locked: player.locked,
                    },
                );
            }
            BoardMessage::UpdatePlayerPiece(uuid, new_player) => {
                if let Some(player) = self.players.get_mut(uuid) {
                    player.rect = Rect::from_two_pos(
                        new_player.position,
                        new_player.position + new_player.size,
                    );
                    player.image_url = new_player.image_url.clone();
                    player.sorting_layer = new_player.sorting_layer;
                    player.visible_by = new_player.visible_by.clone();
                    player.locked = new_player.locked;
                }
            }
            BoardMessage::UpdatePlayerLocation(uuid, new_pos) => {
                if let Some(player) = self.players.get_mut(uuid) {
                    player.rect = Rect::from_two_pos(*new_pos, *new_pos + player.rect.size());
                }
            }
            BoardMessage::DeletePlayerPiece(uuid) => {
                self.players.remove(uuid);
            }
            BoardMessage::ClearBoard => {
                self.players.clear();
                self.dragged_id = None;
                self.selected_id = None;
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
        for (id, player) in self
            .players
            .iter()
            .sorted_by_key(|x| cmp::Reverse(x.1.sorting_layer))
        {
            if player.rect.contains(pointer_pos) {
                return Some(id);
            }
        }
        None
    }

    pub fn is_locked(&self, selected: &Uuid) -> bool {
        self.players
            .get(selected)
            .map(|x| x.locked)
            .unwrap_or_default()
    }

    pub fn get_position(&self, uuid: &Uuid) -> Option<Pos2> {
        self.players.get(uuid).map(|x| x.rect.left_top())
    }
}

pub mod commands {
    use common::SortingLayer;

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
                        piece.rect.left_top(),
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

    pub struct PieceParams {
        pub pos: Pos2,
        pub size: Vec2,
        pub url: Option<String>,
        pub visible_by: Vec<String>,
        pub sorting_layer: SortingLayer,
        pub locked: bool,
    }

    pub struct AddPiece {
        pub params: PieceParams,
    }

    impl Command for AddPiece {
        fn execute(self: Box<Self>, _state: &mut DndState, tx: &EventSender<Signal>) {
            let AddPiece {
                params:
                    PieceParams {
                        pos,
                        size,
                        url,
                        visible_by,
                        sorting_layer,
                        locked,
                    },
            } = *self;

            let uuid = Uuid::new_v4();
            let size = size * Board::GRID_SIZE;
            let pos = snap_to_grid(pos);

            tx.send(
                DndMessage::BoardMessage(BoardMessage::AddPlayerPiece(
                    uuid,
                    common::DndPlayerPiece {
                        position: pos,
                        size,
                        image_url: url,
                        color: None,
                        sorting_layer,
                        visible_by,
                        locked,
                    },
                ))
                .into(),
            )
        }
    }

    pub struct UpdatePiece {
        pub piece_id: Uuid,
        pub params: PieceParams,
    }

    impl Command for UpdatePiece {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            let UpdatePiece {
                piece_id,
                params:
                    PieceParams {
                        pos: _pos,
                        size,
                        url,
                        visible_by,
                        sorting_layer,
                        locked,
                    },
            } = *self;

            let size = size * Board::GRID_SIZE;
            let piece_pos = snap_to_grid(state.board.get_position(&piece_id).unwrap());

            tx.send(
                DndMessage::BoardMessage(BoardMessage::UpdatePlayerPiece(
                    piece_id,
                    common::DndPlayerPiece {
                        position: piece_pos,
                        size,
                        image_url: url,
                        color: None,
                        sorting_layer,
                        visible_by,
                        locked,
                    },
                ))
                .into(),
            )
        }
    }

    pub fn snap_to_grid(pos: Pos2) -> Pos2 {
        // Get back to a grid cell count
        (pos / BoardState::GRID_SIZE).round() * BoardState::GRID_SIZE
    }

    pub struct DeletePiece(pub Uuid);
    impl Command for DeletePiece {
        fn execute(self: Box<Self>, _state: &mut DndState, tx: &EventSender<Signal>) {
            tx.send(DndMessage::BoardMessage(BoardMessage::DeletePlayerPiece(self.0)).into())
        }
    }
}

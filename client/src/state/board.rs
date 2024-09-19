use egui::Rounding;

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

pub struct BoardState {
    pub players: Vec<PlayerPiece>,
    pub dragged_idx: Option<usize>,
    pub selected_idx: Option<usize>,
}

impl Default for BoardState {
    fn default() -> Self {
        Self {
            players: vec![PlayerPiece {
                rect: Rect::from_center_size(Pos2::new(0.0, 0.0), emath::Vec2::new(0.1, 0.1)),
                image: None,
                color: None,
                dragged: false,
                selected: false,
            }],
            dragged_idx: Default::default(),
            selected_idx: Default::default(),
        }
    }
}

impl BoardState {
    const GRID_SIZE: f32 = 0.1;

    pub fn process(&mut self, message: &DndMessage) {}

    pub fn get_player_mut(&mut self, idx: usize) -> Option<&mut PlayerPiece> {
        self.players.get_mut(idx)
    }

    pub fn get_dragged_player_mut(&mut self) -> Option<&mut PlayerPiece> {
        self.dragged_idx.and_then(|x| self.get_player_mut(x))
    }

    pub fn unselect_other_player(&mut self) {
        for player in self.players.iter_mut() {
            if player.selected {
                player.selected = false;
            }
        }
        self.selected_idx = None
    }

    pub fn find_selected_player_idx(&self, pointer_pos: Pos2) -> Option<usize> {
        for (idx, player) in self.players.iter().enumerate() {
            if player.rect.contains(pointer_pos) {
                return Some(idx);
            }
        }
        None
    }
}

pub mod commands {
    use super::*;
    use crate::{prelude::*, view::Board};

    pub struct SetPlayerPosition {
        index: usize,
        new_pos: Pos2,
    }

    impl SetPlayerPosition {
        pub fn new(index: usize, new_pos: Pos2) -> Self {
            Self { index, new_pos }
        }
    }

    impl Command for SetPlayerPosition {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            if let Some(piece) = state.board.get_player_mut(self.index) {
                piece.rect.set_center(self.new_pos);
            }
        }
    }

    pub struct Drop;
    impl Command for Drop {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            if let Some(piece) = state.board.get_dragged_player_mut() {
                piece.drop();
                state.board.dragged_idx = None;
            }
        }
    }

    pub struct Drag(pub usize);
    impl Command for Drag {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            if let Some(player) = state.board.get_player_mut(self.0) {
                player.drag();
                state.board.dragged_idx = Some(self.0);
            }
        }
    }

    pub struct Select(pub Option<usize>);
    impl Command for Select {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            state.board.unselect_other_player();
            if let Some((idx, player)) = self
                .0
                .and_then(|idx| state.board.get_player_mut(idx).map(|p| (idx, p)))
            {
                player.selected = true;
                state.board.selected_idx = Some(idx);
            }
        }
    }

    pub struct AddPiece(pub Pos2, pub Vec2);
    impl Command for AddPiece {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            let center = self.0;
            let center = (center / BoardState::GRID_SIZE).round() * BoardState::GRID_SIZE;
            state.board.players.push(PlayerPiece {
                rect: Rect::from_center_size(center, self.1 * Board::GRID_SIZE),
                image: None,
                color: None,
                dragged: false,
                selected: false,
            })
        }
    }

    pub struct DeletePiece(pub usize);
    impl Command for DeletePiece {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            state.board.players.remove(self.0);
        }
    }
}

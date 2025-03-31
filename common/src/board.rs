use std::collections::HashMap;

use emath::{Pos2, Rect};
use itertools::Itertools;

// Common:
// - Position
// - Size
// - Layer
//
// Player Pieces
// - Modifiable properties
//   - Status effects
//   - Health
//   - name
//   - Image
//   - Layer
//   - Size
//   - Position
// Map Piece (map decoration)
// - Modifiable properties
//   - Image
//   - Layer
//   - Size
//   - Position
// InternalDecoration
// - Image (internal)
// - draggable (true/false)

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Default,
    Copy,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
)]
pub struct SortingLayer(pub u32);

#[derive(
    Clone,
    Copy,
    Debug,
    derive_more::Deref,
    derive_more::DerefMut,
    Eq,
    Hash,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct PieceId(uuid::Uuid);

impl Default for PieceId {
    fn default() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BoardData {
    pub piece_set: BoardPieceSet,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BoardPieceSet {
    pieces: HashMap<PieceId, BoardPiece>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BoardPiece {
    pub id: PieceId,
    pub rect: Rect,
    pub image_url: Option<String>,
    pub sorting_layer: SortingLayer,
    pub data: BoardPieceData,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum BoardPieceData {
    Player(PlayerPieceData),
    None,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlayerPieceData {
    pub name: String,
}

impl BoardPieceSet {
    /// Retrieves the ID of the topmost (by `sorting_layer`) piece on the board in the
    /// specified location. Returns None if no piece is in that location
    pub fn get_topmost_piece_at_position(&self, position: Pos2) -> Option<&PieceId> {
        for (id, piece) in self.sorted_iter_items() {
            if piece.rect.contains(position) {
                return Some(id);
            }
        }

        None
    }

    pub fn sorted_iter_items(&self) -> impl Iterator<Item = (&PieceId, &BoardPiece)> {
        self.pieces
            .iter()
            .sorted_by_key(|(_, piece)| std::cmp::Reverse(piece.sorting_layer))
    }

    pub fn sorted_iter(&self) -> impl Iterator<Item = &BoardPiece> {
        self.sorted_iter_items().map(|(_, x)| x)
    }

    pub fn get_piece(&self, id: &PieceId) -> Option<&BoardPiece> {
        self.pieces.get(id)
    }

    pub fn get_piece_mut(&mut self, id: &PieceId) -> Option<&mut BoardPiece> {
        self.pieces.get_mut(id)
    }

    pub fn add_or_update_piece(&mut self, piece: BoardPiece) {
        self.pieces.insert(piece.id, piece);
    }
}

impl BoardPiece {
    pub fn from_rect(id: PieceId, rect: Rect) -> Self {
        Self {
            id,
            rect,
            image_url: None,
            sorting_layer: SortingLayer::default(),
            data: BoardPieceData::None,
        }
    }
}

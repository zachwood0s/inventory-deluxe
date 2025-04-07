use std::{
    collections::{HashMap, HashSet},
    i32,
};

use emath::{Pos2, Rect};
use itertools::{Itertools, MinMaxResult};

use crate::{
    message::{BackpackPiece, BoardMessage},
    User,
};

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
pub struct SortingLayer(pub i32);

impl SortingLayer {
    pub fn one_lower(self) -> Self {
        SortingLayer(self.0.saturating_sub(1))
    }
    pub fn one_higher(self) -> Self {
        SortingLayer(self.0.saturating_add(1))
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialOrd,
    Ord,
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
    pub backpack_set: BackpackSet,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BoardPieceSet {
    pieces: HashMap<PieceId, BoardPiece>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BackpackSet {
    pieces: HashMap<String, BackpackPiece>,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct LayerInfo {
    pub next_highest_layer: SortingLayer,
    pub next_lowest_layer: SortingLayer,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum GridSnap {
    MajorSpacing(f32),
    None,
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, derive_more::Deref, derive_more::DerefMut,
)]
pub struct Color([f32; 4]);

impl Default for Color {
    fn default() -> Self {
        Self([1.0, 1.0, 1.0, 1.0])
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BoardPiece {
    pub id: PieceId,
    pub name: String,
    pub rect: Rect,
    pub image_url: String,
    pub color: Color,
    pub sorting_layer: SortingLayer,
    pub snap_to_grid: GridSnap,
    pub locked: bool,
    pub display_name: bool,
    pub data: BoardPieceData,
}

#[derive(Clone, Debug, derive_more::Display, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum BoardPieceData {
    #[display("Character")]
    Character(CharacterPieceData),
    None,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CharacterPieceData {
    pub link_stats_to: Option<User>,
}

impl BoardData {
    pub fn handle_message(&mut self, msg: BoardMessage) {
        match msg {
            BoardMessage::AddOrUpdatePiece(piece) => {
                self.piece_set.add_or_update(piece);
            }
            BoardMessage::DeletePiece(piece) => {
                self.piece_set.remove(&piece);
            }
            BoardMessage::StoreBackpackPiece(piece) => {
                self.backpack_set
                    .pieces
                    .insert(piece.piece.name.clone(), piece);
            }
            BoardMessage::RemoveBackpackPiece(piece) => {
                self.backpack_set.pieces.remove(&piece);
            }
        }
    }
}

impl BackpackSet {
    pub fn iter(&self) -> impl Iterator<Item = &BackpackPiece> {
        self.pieces.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut BackpackPiece> {
        self.pieces.values_mut()
    }

    pub fn get_piece(&self, id: &String) -> Option<&BackpackPiece> {
        self.pieces.get(id)
    }

    pub fn get_piece_mut(&mut self, id: &String) -> Option<&mut BackpackPiece> {
        self.pieces.get_mut(id)
    }

    pub fn add_or_update(&mut self, piece: BackpackPiece) {
        self.pieces.insert(piece.piece.name.clone(), piece);
    }

    pub fn remove(&mut self, piece_id: &String) {
        self.pieces.remove(piece_id);
    }

    pub fn len(&self) -> usize {
        self.pieces.len()
    }
}

impl BoardPieceSet {
    /// Retrieves the ID of the topmost (by `sorting_layer`) piece on the board in the
    /// specified location. Returns None if no piece is in that location
    pub fn get_topmost_piece_at_position(&self, position: Pos2) -> Option<&PieceId> {
        for (id, piece) in self.sorted_iter_items_reversed() {
            if piece.rect.contains(position) {
                return Some(id);
            }
        }

        None
    }

    pub fn sorted_iter_items_reversed(&self) -> impl Iterator<Item = (&PieceId, &BoardPiece)> {
        self.pieces
            .iter()
            .sorted_by_key(|(_, piece)| std::cmp::Reverse(piece.sort_key()))
    }

    pub fn sorted_iter_items(&self) -> impl Iterator<Item = (&PieceId, &BoardPiece)> {
        self.pieces
            .iter()
            .sorted_by_key(|(_, piece)| piece.sort_key())
    }

    pub fn sorted_iter(&self) -> impl Iterator<Item = &BoardPiece> {
        self.sorted_iter_items().map(|(_, x)| x)
    }

    pub fn iter(&self) -> impl Iterator<Item = &BoardPiece> {
        self.pieces.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut BoardPiece> {
        self.pieces.values_mut()
    }

    pub fn get_piece(&self, id: &PieceId) -> Option<&BoardPiece> {
        self.pieces.get(id)
    }

    pub fn get_piece_mut(&mut self, id: &PieceId) -> Option<&mut BoardPiece> {
        self.pieces.get_mut(id)
    }

    pub fn add_or_update(&mut self, piece: BoardPiece) {
        self.pieces.insert(piece.id, piece);
    }

    pub fn remove(&mut self, piece_id: &PieceId) {
        self.pieces.remove(piece_id);
    }

    pub fn layer_info(&self) -> LayerInfo {
        let (next_lowest_layer, next_highest_layer) =
            match self.pieces.values().map(|x| x.sorting_layer).minmax() {
                MinMaxResult::NoElements => (SortingLayer::default(), SortingLayer::default()),
                MinMaxResult::OneElement(x) => (x.one_lower(), x.one_higher()),
                MinMaxResult::MinMax(min, max) => (min.one_lower(), max.one_higher()),
            };

        LayerInfo {
            next_highest_layer,
            next_lowest_layer,
        }
    }
}

impl BoardPiece {
    pub fn from_rect(id: PieceId, name: String, rect: Rect) -> Self {
        Self {
            id,
            rect,
            name,
            color: Color::default(),
            image_url: String::default(),
            sorting_layer: SortingLayer::default(),
            snap_to_grid: GridSnap::MajorSpacing(1.0),
            locked: false,
            display_name: false,
            data: BoardPieceData::None,
        }
    }

    pub fn sort_key(&self) -> impl Ord {
        (self.sorting_layer, self.id)
    }
}

impl GridSnap {
    pub fn is_snap(&self) -> bool {
        match self {
            GridSnap::MajorSpacing(_) => true,
            GridSnap::None => false,
        }
    }
}

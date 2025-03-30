use std::collections::HashMap;

use common::SortingLayer;
use egui::{Color32, Pos2, Rect};
use itertools::Itertools;

use crate::view::{
    board_render::{BoardRender, ChildRender},
    properties_window::PropertiesDisplay,
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

pub trait BoardPieceImpl: PropertiesDisplay + BoardRender {
    fn sorting_layer(&self) -> SortingLayer;
    fn get_rect(&self) -> &Rect;
    fn get_rect_mut(&mut self) -> &mut Rect;
}

#[derive(Clone, Copy, Debug, derive_more::Deref, derive_more::DerefMut, Eq, Hash, PartialEq)]
pub struct PieceId(uuid::Uuid);

#[derive(Default)]
pub struct BoardPieceSet {
    pieces: HashMap<PieceId, Box<dyn BoardPieceImpl>>,
}

impl BoardPieceSet {
    /// Retrieves the ID of the topmost (by `sorting_layer`) piece on the board in the
    /// specified location. Returns None if no piece is in that location
    pub fn get_topmost_piece_at_position(&self, position: Pos2) -> Option<&PieceId> {
        for (id, piece) in self.sorted_iter_items() {
            if piece.get_rect().contains(position) {
                return Some(id);
            }
        }

        None
    }

    pub fn sorted_iter_items(&self) -> impl Iterator<Item = (&PieceId, &dyn BoardPieceImpl)> {
        self.pieces
            .iter()
            .sorted_by_key(|(_, piece)| std::cmp::Reverse(piece.sorting_layer()))
            .map(|(id, v)| (id, &**v))
    }

    pub fn sorted_iter(&self) -> impl Iterator<Item = &dyn BoardPieceImpl> {
        self.sorted_iter_items().map(|(_, x)| x)
    }

    pub fn get_piece(&self, id: &PieceId) -> Option<&dyn BoardPieceImpl> {
        self.pieces.get(id).map(|v| &**v)
    }

    pub fn get_piece_mut(
        &mut self,
        id: &mut PieceId,
    ) -> Option<&mut (dyn BoardPieceImpl + 'static)> {
        self.pieces.get_mut(id).map(|v| &mut **v)
    }
}

pub struct BoardPiece<T> {
    pub id: PieceId,
    pub rect: Rect,
    pub image_url: Option<String>,
    pub sorting_layer: SortingLayer,
    pub data: T,
}

pub struct PlayerPieceData {
    pub name: String,
}

pub struct MapPieceData {}

pub struct InternalDecorationData {}

impl<T> BoardPieceImpl for BoardPiece<T>
where
    T: PropertiesDisplay + BoardRender + ChildRender,
{
    fn sorting_layer(&self) -> SortingLayer {
        self.sorting_layer
    }

    fn get_rect(&self) -> &Rect {
        &self.rect
    }

    fn get_rect_mut(&mut self) -> &mut Rect {
        &mut self.rect
    }
}

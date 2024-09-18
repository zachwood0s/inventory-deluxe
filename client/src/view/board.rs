use clap::builder::styling::Color;
use egui::{
    epaint::PathStroke, Color32, Frame, Image, Painter, Pos2, Rect, Rounding, Shape, TextureId,
};
use emath::{Rangef, RectTransform};

use crate::{
    listener::{CommandQueue, Signal},
    state::DndState,
};

use super::DndTabImpl;

pub struct PlayerPiece {
    rect: Rect,
    image: Option<TextureId>,
    color: Option<Color32>,
    dragged: bool,
    selected: bool,
}

impl PlayerPiece {
    fn drop(&mut self) {
        let center = self.rect.center();
        let center = (center / Board::GRID_SIZE).round() * Board::GRID_SIZE;
        self.rect.set_center(center);
        self.dragged = false;
    }

    fn drag(&mut self) {
        self.dragged = true;
    }

    fn draw_shape(&self, to_screen: RectTransform) -> egui::Shape {
        let transformed = to_screen.transform_rect(self.rect);
        if self.dragged {
            egui::Shape::rect_filled(transformed, Rounding::ZERO, Color32::GREEN)
        } else if self.selected {
            egui::Shape::rect_filled(transformed, Rounding::ZERO, Color32::RED)
        } else {
            egui::Shape::rect_filled(transformed, Rounding::ZERO, Color32::WHITE)
        }
    }
}

pub struct Board {
    players: Vec<PlayerPiece>,
    dragged_idx: Option<usize>,
    selected_idx: Option<usize>,
    grid_origin: Pos2,
    zoom: f32,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            players: vec![
                PlayerPiece {
                    rect: Rect::from_center_size(Pos2::ZERO, egui::Vec2::new(0.1, 0.1)),
                    image: None,
                    color: None,
                    dragged: false,
                    selected: false,
                },
                PlayerPiece {
                    rect: Rect::from_center_size(Pos2::new(0.2, 0.0), egui::Vec2::new(0.1, 0.1)),
                    image: None,
                    color: None,
                    dragged: false,
                    selected: false,
                },
            ],
            grid_origin: Pos2::ZERO,
            dragged_idx: None,
            selected_idx: None,
            zoom: 1.0,
        }
    }
}

impl Board {
    const GRID_SIZE: f32 = 0.1;

    fn ui_content(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let (mut response, painter) = ui.allocate_painter(
            ui.available_size_before_wrap(),
            egui::Sense::click_and_drag(),
        );

        let dims = response.rect.square_proportions() * self.zoom.sqrt();
        let to_screen = emath::RectTransform::from_to(
            Rect::from_center_size(self.grid_origin, dims),
            response.rect,
        );

        let from_screen = to_screen.inverse();

        if let Some(dragged) = self.get_dragged_player_mut() {
            // We have a selected piece so move its position
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let canvas_pos = from_screen * pointer_pos;
                dragged.rect.set_center(canvas_pos);
                response.mark_changed();
            } else {
                dragged.drop();
                self.dragged_idx = None;
                response.mark_changed();
            }
        } else if response.dragged_by(egui::PointerButton::Primary) {
            // Handle initial dragging of a piece
            if let Some((idx, player)) = response
                .interact_pointer_pos()
                .and_then(|x| self.find_selected_player_mut(from_screen * x))
            {
                player.drag();
                self.dragged_idx = Some(idx);
                response.mark_changed();
            }
        } else if response.clicked_by(egui::PointerButton::Primary) {
            // Handle selection of a piece
            self.unselect_other_player();
            if let Some((idx, player)) = response
                .interact_pointer_pos()
                .and_then(|x| self.find_selected_player_mut(from_screen * x))
            {
                player.selected = true;
                self.selected_idx = Some(idx);
            }
            response.mark_changed();
        } else if response.dragged_by(egui::PointerButton::Middle) {
            let screen_origin = to_screen * self.grid_origin;
            self.grid_origin = from_screen * (screen_origin - response.drag_delta());
        }

        self.handle_zoom(ui);

        self.draw_grid(dims, &painter, &to_screen);

        let shapes = self
            .players
            .iter()
            .map(|player| player.draw_shape(to_screen));

        painter.extend(shapes);

        response
    }

    fn draw_grid(&self, dims: egui::Vec2, painter: &Painter, to_screen: &RectTransform) {
        let num_x = (dims.x / Board::GRID_SIZE) as i32 + 1;
        let num_y = (dims.y / Board::GRID_SIZE) as i32 + 1;

        let topleft_boundary = self.grid_origin - dims / 2.0;

        let round = topleft_boundary.y.rem_euclid(Board::GRID_SIZE);
        let y_start = topleft_boundary.y - round + Board::GRID_SIZE / 2.0;
        for y in (0..num_y).map(|x| x as f32 * Board::GRID_SIZE + y_start) {
            painter.add(Shape::line_segment(
                [
                    to_screen * Pos2::new(-dims.x, y),
                    to_screen * Pos2::new(dims.x, y),
                ],
                PathStroke::new(1.0, Color32::DARK_GRAY),
            ));
        }

        let round = topleft_boundary.x.rem_euclid(Board::GRID_SIZE);
        let x_start = topleft_boundary.x - round + Board::GRID_SIZE / 2.0;
        for x in (0..num_x).map(|x| x as f32 * Board::GRID_SIZE + x_start) {
            painter.add(Shape::line_segment(
                [
                    to_screen * Pos2::new(x, -dims.y),
                    to_screen * Pos2::new(x, dims.y),
                ],
                PathStroke::new(1.0, Color32::DARK_GRAY),
            ));
        }
    }

    fn handle_zoom(&mut self, ui: &mut egui::Ui) {
        const ZOOM_FACTOR: f32 = 0.01;
        const MAX_ZOOM: f32 = 10.0;
        const MIN_ZOOM: f32 = 0.5;
        self.zoom += ui.input(|i| i.smooth_scroll_delta.y) * ZOOM_FACTOR;
        self.zoom = self.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    fn get_dragged_player_mut(&mut self) -> Option<&mut PlayerPiece> {
        self.dragged_idx.and_then(|x| self.players.get_mut(x))
    }

    fn get_selected_player_mut(&mut self) -> Option<&mut PlayerPiece> {
        self.selected_idx.and_then(|x| self.players.get_mut(x))
    }

    fn unselect_other_player(&mut self) {
        for player in self.players.iter_mut() {
            if player.selected {
                player.selected = false;
            }
        }
        self.selected_idx = None
    }

    fn find_selected_player_mut(&mut self, pointer_pos: Pos2) -> Option<(usize, &mut PlayerPiece)> {
        for (idx, player) in self.players.iter_mut().enumerate() {
            if player.rect.contains(pointer_pos) {
                return Some((idx, player));
            }
        }
        None
    }
}

impl DndTabImpl for Board {
    // TODO: Problem: how to get the server updates into the Tab here. Typically I've been wanting
    // to keep the state modifications that need to be synced out of the tab itself but because I
    // want responsive grid I think its best to put the updates in the tab itself. I'll need to
    // route them from the rx portion into the tab somehow. I could also have a shadow copy of the
    // state where I route commands to but not sure that fixes my problem. If I store in the tab,
    // if I close and need to reopen I'll have to ask the server for the current board data (not
    // hard). Maybe I'm dumb and the commands would actually fix the issue (more I  think about it
    // the more I think that might be the case. Just need careful handling of the repaint logic)
    //
    // TODO: Most likely when doing updates I'll just want to pass the updates, not the full board
    // state. That way we won't have people overriding eachothers changes (or at least will
    // mimimize that).
    //
    // TODO: Need to be able to add/remove things to the board (and obviously tell the server something
    // has been added/removed)
    fn ui(&mut self, ui: &mut egui::Ui, state: &DndState, network: &mut CommandQueue) {
        Frame::canvas(ui.style()).show(ui, |ui| self.ui_content(ui));
    }

    fn title(&self) -> String {
        "Board".to_owned()
    }
}

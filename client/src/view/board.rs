use crate::prelude::*;
use egui::{epaint::PathStroke, Color32, DragValue, Frame, Painter, Rect, Shape, Widget};
use emath::RectTransform;
use log::info;

use crate::{
    listener::CommandQueue,
    state::{
        board::{self},
        DndState,
    },
};

use super::DndTabImpl;

pub struct Board {
    mouse_pos: Pos2,
    grid_origin: Pos2,
    zoom: f32,
    show_grid: bool,
    width: u32,
    height: u32,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            mouse_pos: Pos2::ZERO,
            grid_origin: Pos2::ZERO,
            zoom: 1.0,
            show_grid: false,
            width: 0,
            height: 0,
        }
    }
}

impl Board {
    pub const GRID_SIZE: f32 = 0.1;

    fn ui_content(
        &mut self,
        ui: &mut egui::Ui,
        state: &DndState,
        commands: &mut CommandQueue,
    ) -> egui::Response {
        let (response, painter) = ui.allocate_painter(
            ui.available_size_before_wrap(),
            egui::Sense::click_and_drag(),
        );

        if let Some(pos) = response.interact_pointer_pos() {
            self.mouse_pos = pos;
        }

        let dims = response.rect.square_proportions() * self.zoom.sqrt();
        let to_screen = emath::RectTransform::from_to(
            Rect::from_center_size(self.grid_origin, dims),
            response.rect,
        );

        let from_screen = to_screen.inverse();

        if let Some(dragged) = state.board.dragged_id {
            // We have a selected piece so move its position
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let canvas_pos = from_screen * pointer_pos;
                commands.add(board::commands::SetPlayerPosition::new(dragged, canvas_pos));
            } else {
                commands.add(board::commands::Drop)
            }
        } else if response.dragged_by(egui::PointerButton::Primary) {
            // Handle initial dragging of a piece
            if let Some(idx) = response
                .interact_pointer_pos()
                .and_then(|x| state.board.find_selected_player_id(from_screen * x))
            {
                commands.add(board::commands::Drag(*idx));
            }
        } else if response.clicked_by(egui::PointerButton::Primary) {
            // Handle selection of a piece
            let selected_idx = response
                .interact_pointer_pos()
                .and_then(|x| state.board.find_selected_player_id(from_screen * x))
                .copied();

            commands.add(board::commands::Select(selected_idx));
        } else if response.dragged_by(egui::PointerButton::Middle) {
            let screen_origin = to_screen * self.grid_origin;
            self.grid_origin = from_screen * (screen_origin - response.drag_delta());
        } else if ui.input(|input| input.key_pressed(egui::Key::Delete)) {
            if let Some(selected) = state.board.selected_id {
                commands.add(board::commands::DeletePiece(selected));
            }
        }

        response.context_menu(|ui| {
            ui.menu_button("Add Piece", |ui| {
                DragValue::new(&mut self.width)
                    .prefix("w: ")
                    .range(1..=10)
                    .ui(ui);
                DragValue::new(&mut self.height)
                    .prefix("h: ")
                    .range(1..=10)
                    .ui(ui);
                if ui.button("Add").clicked() {
                    info!("{} {}", from_screen * self.mouse_pos, self.mouse_pos);
                    commands.add(board::commands::AddPiece(
                        from_screen * self.mouse_pos,
                        Vec2::new(self.width as f32, self.height as f32),
                    ));
                }
            });

            ui.checkbox(&mut self.show_grid, "Grid");
        });

        self.handle_zoom(ui);

        if self.show_grid {
            self.draw_grid(dims, &painter, &to_screen);
        }

        let shapes = state
            .board
            .players
            .values()
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
                    to_screen * Pos2::new(-dims.x + self.grid_origin.x, y),
                    to_screen * Pos2::new(dims.x + self.grid_origin.x, y),
                ],
                PathStroke::new(1.0, Color32::DARK_GRAY),
            ));
        }

        let round = topleft_boundary.x.rem_euclid(Board::GRID_SIZE);
        let x_start = topleft_boundary.x - round + Board::GRID_SIZE / 2.0;
        for x in (0..num_x).map(|x| x as f32 * Board::GRID_SIZE + x_start) {
            painter.add(Shape::line_segment(
                [
                    to_screen * Pos2::new(x, -dims.y + self.grid_origin.y),
                    to_screen * Pos2::new(x, dims.y + self.grid_origin.y),
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
    fn ui(&mut self, ui: &mut egui::Ui, state: &DndState, commands: &mut CommandQueue) {
        Frame::canvas(ui.style()).show(ui, |ui| self.ui_content(ui, state, commands));
    }

    fn title(&self) -> String {
        "Board".to_owned()
    }
}

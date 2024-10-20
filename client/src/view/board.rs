use crate::{prelude::*, state::board::commands::PieceParams};
use egui::{epaint::PathStroke, Color32, DragValue, Frame, Image, Painter, Rect, Shape, Widget};
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
    width: u32,
    height: u32,
    new_url: String,

    show_grid: bool,
    hidden_to_others: bool,
    background: bool,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            mouse_pos: Pos2::ZERO,
            grid_origin: Pos2::ZERO,
            zoom: 1.0,
            width: 0,
            height: 0,
            new_url: String::new(),

            show_grid: false,
            hidden_to_others: false,
            background: false,
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

        let dims = response.rect.square_proportions() * self.zoom;
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

            if let Some(selected) = selected_idx {
                self.new_url = state.board.players[&selected]
                    .image_url
                    .clone()
                    .unwrap_or_default()
            }

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

                ui.horizontal(|ui| {
                    ui.label("url: ");
                    ui.text_edit_singleline(&mut self.new_url);
                });

                ui.checkbox(&mut self.hidden_to_others, "Hide from others");
                ui.checkbox(&mut self.background, "Background");

                if let Some(selected) = state.board.selected_id {
                    if ui.button("Update").clicked() {
                        info!(
                            "Updating {} {}",
                            from_screen * self.mouse_pos,
                            self.mouse_pos
                        );

                        let image_url = if self.new_url.is_empty() {
                            None
                        } else {
                            Some(self.new_url.clone())
                        };

                        commands.add(board::commands::UpdatePiece {
                            piece_id: selected,
                            params: PieceParams {
                                pos: from_screen * self.mouse_pos,
                                size: Vec2::new(self.width as f32, self.height as f32),
                                url: image_url,
                                hidden_to_others: self.hidden_to_others,
                                background: self.background,
                            },
                        });
                    }
                } else if ui.button("Add").clicked() {
                    info!("Adding {} {}", from_screen * self.mouse_pos, self.mouse_pos);

                    let image_url = if self.new_url.is_empty() {
                        None
                    } else {
                        Some(self.new_url.clone())
                    };

                    commands.add(board::commands::AddPiece {
                        params: PieceParams {
                            pos: from_screen * self.mouse_pos,
                            size: Vec2::new(self.width as f32, self.height as f32),
                            url: image_url,
                            hidden_to_others: self.hidden_to_others,
                            background: self.background,
                        },
                    });
                }
            });

            ui.checkbox(&mut self.show_grid, "Grid");
        });

        self.handle_zoom(ui);

        if self.show_grid {
            self.draw_grid(dims, &painter, &to_screen);
        }

        for player in state.board.players.values() {
            player.draw_shape(ui, &painter, to_screen);
        }

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
        self.zoom /= (ui.input(|i| i.smooth_scroll_delta.y) * ZOOM_FACTOR) + 1.0;
        self.zoom = self.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }
}

impl DndTabImpl for Board {
    fn ui(&mut self, ui: &mut egui::Ui, state: &DndState, commands: &mut CommandQueue) {
        Frame::canvas(ui.style()).show(ui, |ui| self.ui_content(ui, state, commands));
    }

    fn title(&self) -> String {
        "Board".to_owned()
    }
}

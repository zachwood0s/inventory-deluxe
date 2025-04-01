use board_render::{BoardRender, Grid, RenderContext};
use common::{
    board::{BoardPiece, BoardPieceSet, GridSnap, PieceId},
    SortingLayer,
};
use egui::{
    epaint::PathStroke, Color32, DragValue, Frame, Image, Painter, Pos2, Rect, Response, Rounding,
    Shape, Stroke, Vec2, Widget,
};
use emath::RectTransform;
use itertools::Itertools;
use log::info;
use properties_window::{PropertiesCtx, PropertiesDisplay};
use uuid::Uuid;

use crate::{
    listener::CommandQueue,
    state::{
        board::{self, commands::AddPiece},
        DndState,
    },
};

use super::DndTabImpl;

pub mod board_render;
pub mod properties_window;

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

#[derive(Default, Clone, Copy)]
pub struct SelectionState {
    view_properties: Option<PieceId>,
    selected: Option<PieceId>,
    dragged: Option<PieceId>,
}

#[derive(Default, Clone, Copy)]
pub struct InputState {
    board_mouse_pos: Pos2,
    screen_mouse_pos: Pos2,
}

pub struct UiBoardState {
    grid: Grid,

    selection: SelectionState,
    input: InputState,

    view_origin: Pos2,
    zoom: f32,
}

impl Default for UiBoardState {
    fn default() -> Self {
        Self {
            grid: Grid::new(0.1),

            selection: SelectionState::default(),
            input: InputState::default(),

            view_origin: Pos2::default(),
            zoom: 1.0,
        }
    }
}

impl UiBoardState {
    fn view_properties<'a>(&self, piece_set: &'a mut BoardPieceSet) -> Option<&'a mut BoardPiece> {
        let piece_id = self.selection.view_properties?;

        piece_set.get_piece_mut(&piece_id)
    }

    fn clear_selected(&mut self) {
        self.selection.selected = None;
        self.selection.view_properties = None;
    }

    fn handle_zoom(&mut self, ui: &mut egui::Ui) {
        const ZOOM_FACTOR: f32 = 0.01;
        const MAX_ZOOM: f32 = 10.0;
        const MIN_ZOOM: f32 = 0.5;
        self.zoom /= (ui.input(|i| i.smooth_scroll_delta.y) * ZOOM_FACTOR) + 1.0;
        self.zoom = self.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    fn handle_board_input(
        &mut self,
        ctx: &mut RenderContext,
        response: &Response,
        piece_set: &BoardPieceSet,
        commands: &mut CommandQueue,
    ) {
        self.handle_zoom(ctx.ui);

        if let Some(pos) = response.interact_pointer_pos() {
            self.input.screen_mouse_pos = pos;
            self.input.board_mouse_pos = ctx.to_grid * (ctx.from_screen * pos);
        }

        // Handle Dragging
        if response.dragged_by(egui::PointerButton::Middle) {
            let screen_origin = ctx.to_screen * self.view_origin;
            self.view_origin = ctx.from_screen * (screen_origin - response.drag_delta());
        }

        // Handle selection of a piece, both right and left click select
        if response.clicked_by(egui::PointerButton::Primary)
            || response.clicked_by(egui::PointerButton::Secondary)
        {
            let selected_idx = piece_set.get_topmost_piece_at_position(self.input.board_mouse_pos);

            info!("New selected {selected_idx:?}");

            if selected_idx.is_some() {
                self.selection.selected = selected_idx.copied();
            } else {
                self.clear_selected();
            }
        }

        response.context_menu(|ui| {
            ui.set_width(100.0);

            if let Some(selected) = self.selection.selected {
                if ui.button("View Properties").clicked() {
                    self.selection.view_properties = Some(selected);
                }
            } else if ui.button("Add Piece").clicked() {
                let new_id = PieceId::default();
                self.selection.view_properties = Some(new_id);
                self.selection.selected = Some(new_id);

                let new_rect = self.grid.unit_rect(self.input.board_mouse_pos);
                let new_piece = BoardPiece::from_rect(new_id, "New Piece".into(), new_rect);

                commands.add(AddPiece(new_piece))
            }
        });
    }

    fn snap_to_grid(&mut self, piece_set: &mut BoardPieceSet) {
        for piece in piece_set.iter_mut() {
            if let GridSnap::MajorSpacing(spacing) = piece.snap_to_grid {
                let current_pos = piece.rect.min;

                let new_pos = (current_pos / spacing).floor() * spacing;

                piece.rect = piece.rect.translate(new_pos - current_pos);
            }
        }
    }

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

        let render_dimensions = response.rect.square_proportions() * self.zoom;
        let to_screen = emath::RectTransform::from_to(
            Rect::from_center_size(self.view_origin, render_dimensions),
            response.rect,
        );

        let from_grid = self.grid.from_grid();

        let mut ctx = RenderContext {
            ui,
            painter,
            selection_state: self.selection,
            from_grid,
            to_grid: from_grid.inverse(),
            to_screen,
            from_screen: to_screen.inverse(),
            render_dimensions,
        };

        let mut board = state.client_board.lock().unwrap();

        self.handle_board_input(&mut ctx, &response, &board.piece_set, commands);
        self.snap_to_grid(&mut board.piece_set);
        self.grid.render(&ctx);
        board.piece_set.render(&ctx);

        let mut ctx = PropertiesCtx {
            state,
            changed: false,
        };

        if let Some(piece) = self.view_properties(&mut board.piece_set) {
            piece.display_props(ui, &mut ctx);
        }

        if ctx.changed {
            info!("Changed!");
        }

        response
    }
}

pub struct Board {
    mouse_pos: Pos2,
    grid_origin: Pos2,
    drag_offset: Vec2,
    highlight_start_pos: Option<Pos2>,
    highlight_end_pos: Pos2,
    zoom: f32,
    width: u32,
    height: u32,
    new_url: String,

    show_grid: bool,
    player_list: Vec<String>,
    sorting_layer: SortingLayer,

    locked: bool,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            mouse_pos: Pos2::ZERO,
            grid_origin: Pos2::ZERO,
            drag_offset: Vec2::ZERO,
            highlight_start_pos: None,
            highlight_end_pos: Pos2::ZERO,
            zoom: 1.0,
            width: 0,
            height: 0,
            new_url: String::new(),

            show_grid: false,
            player_list: Vec::default(),
            sorting_layer: SortingLayer::default(),

            locked: false,
        }
    }
}

impl Board {
    pub const GRID_SIZE: f32 = 0.1;

    fn copy_selected_stats(&mut self, state: &DndState, selected: &Uuid) {
        let selected = &state.board.players[selected];
        self.new_url = selected.image_url.clone().unwrap_or_default();

        let dims = (selected.rect.size() / Board::GRID_SIZE).round();
        self.width = dims.x as u32;
        self.height = dims.y as u32;

        self.sorting_layer = selected.sorting_layer;
        self.locked = selected.locked;
        self.player_list = selected.visible_by.clone();
    }

    fn character_selection(&mut self, ui: &mut egui::Ui, state: &DndState) {
        let mut new_list = Vec::new();
        for c in state.character_list.iter() {
            let mut checked = self.player_list.contains(c);
            ui.checkbox(&mut checked, c);

            if checked {
                new_list.push(c.clone());
            }
        }
        self.player_list = new_list;
    }

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
                commands.add(board::commands::SetPlayerPosition::new(
                    dragged,
                    canvas_pos + self.drag_offset,
                ));
            } else {
                commands.add(board::commands::Drop)
            }
        } else if response.dragged_by(egui::PointerButton::Primary)
            && ui.input(|input| !input.modifiers.any())
        {
            // Handle initial dragging of a piece
            if let Some(uuid) = response
                .interact_pointer_pos()
                .and_then(|x| state.board.find_selected_player_id(from_screen * x))
            {
                if !state.board.is_locked(uuid) {
                    // Get dragging offset
                    let pointer_canvas_pos = from_screen * response.interact_pointer_pos().unwrap();
                    let piece_canvas_pos = state.board.get_position(uuid).unwrap();

                    self.drag_offset = piece_canvas_pos - pointer_canvas_pos;

                    commands.add(board::commands::Drag(*uuid));

                    // Dragging also selects the piece
                    commands.add(board::commands::Select(Some(*uuid)));

                    self.copy_selected_stats(state, uuid)
                }
            }
        } else if let Some(_) = self.highlight_start_pos {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                self.highlight_end_pos = pointer_pos;
            } else {
                let size_rect = Rect::from_two_pos(
                    (from_screen * self.highlight_end_pos * 10.0).round(),
                    (from_screen * self.highlight_start_pos.unwrap() * 10.0).round(),
                );

                let center_rect = Rect::from_two_pos(
                    (from_screen * self.highlight_end_pos / Board::GRID_SIZE).round()
                        * Board::GRID_SIZE,
                    (from_screen * self.highlight_start_pos.unwrap() / Board::GRID_SIZE).round()
                        * Board::GRID_SIZE,
                );

                /*
                commands.add(board::commands::AddPiece {
                    params: PieceParams {
                        pos: center_rect.left_top(),
                        size: size_rect.size(),
                        url: None,
                        visible_by: vec![],
                        sorting_layer: common::SortingLayer(10),
                        locked: false,
                    },
                });
                */

                self.highlight_start_pos = None;
            }
        } else if ui.input(|input| input.modifiers.ctrl) && response.is_pointer_button_down_on() {
            self.highlight_start_pos = response.interact_pointer_pos();
            self.highlight_end_pos = response.interact_pointer_pos().unwrap();
        } else if response.clicked_by(egui::PointerButton::Primary) {
            // Handle selection of a piece
            let selected_idx = response
                .interact_pointer_pos()
                .and_then(|x| state.board.find_selected_player_id(from_screen * x))
                .copied();

            if let Some(selected) = &selected_idx {
                self.copy_selected_stats(state, selected)
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
            let menu_text = if state.board.selected_id.is_some() {
                "Update Piece"
            } else {
                "Add Piece"
            };

            ui.menu_button(menu_text, |ui| {
                ui.menu_button("Visible By", |ui| {
                    self.character_selection(ui, state);
                });

                DragValue::new(&mut self.width)
                    .prefix("w: ")
                    .range(1..=100)
                    .ui(ui);

                DragValue::new(&mut self.height)
                    .prefix("h: ")
                    .range(1..=100)
                    .ui(ui);

                DragValue::new(&mut self.sorting_layer.0)
                    .prefix("layer: ")
                    .range(1..=10)
                    .ui(ui);

                ui.horizontal(|ui| {
                    ui.label("url: ");
                    ui.text_edit_singleline(&mut self.new_url);
                });

                ui.checkbox(&mut self.locked, "Locked: ");

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

                        /*
                        commands.add(board::commands::UpdatePiece {
                            piece_id: selected,
                            params: PieceParams {
                                pos: Pos2::ZERO,
                                size: Vec2::new(self.width as f32, self.height as f32),
                                url: image_url,
                                visible_by: self.player_list.clone(),
                                sorting_layer: self.sorting_layer,
                                locked: self.locked,
                            },
                        });
                        */
                    }
                } else if ui.button("Add").clicked() {
                    info!("Adding {} {}", from_screen * self.mouse_pos, self.mouse_pos);

                    let image_url = if self.new_url.is_empty() {
                        None
                    } else {
                        Some(self.new_url.clone())
                    };

                    /*
                    commands.add(board::commands::AddPiece {
                        params: PieceParams {
                            pos: from_screen * self.mouse_pos,
                            size: Vec2::new(self.width as f32, self.height as f32),
                            url: image_url,
                            visible_by: self.player_list.clone(),
                            sorting_layer: self.sorting_layer,
                            locked: self.locked,
                        },
                    });
                    */
                }
            });

            ui.checkbox(&mut self.show_grid, "Grid");
        });

        self.handle_zoom(ui);

        if self.show_grid {
            self.draw_grid(dims, &painter, &to_screen);
        }

        for player in state
            .board
            .players
            .values()
            .sorted_by_key(|x| x.sorting_layer)
            .filter(|x| x.visible_by.contains(&state.owned_user().name) || x.visible_by.is_empty())
        {
            player.draw_shape(ui, &painter, to_screen);
        }

        if let Some(pointer_pos) = self.highlight_start_pos {
            //Draw highlight rect
            let rect = Rect::from_two_pos(pointer_pos, self.highlight_end_pos);
            painter.rect_stroke(rect, Rounding::ZERO, Stroke::new(1.0, Color32::LIGHT_BLUE));
        }

        response
    }

    fn draw_grid(&self, dims: egui::Vec2, painter: &Painter, to_screen: &RectTransform) {
        let num_x = (dims.x / Board::GRID_SIZE) as i32 + 1;
        let num_y = (dims.y / Board::GRID_SIZE) as i32 + 1;

        let topleft_boundary = self.grid_origin - dims / 2.0;

        let round = topleft_boundary.y.rem_euclid(Board::GRID_SIZE);
        let y_start = topleft_boundary.y - round;
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
        let x_start = topleft_boundary.x - round;
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

impl DndTabImpl for UiBoardState {
    fn ui(&mut self, ui: &mut egui::Ui, state: &DndState, commands: &mut CommandQueue) {
        Frame::canvas(ui.style()).show(ui, |ui| self.ui_content(ui, state, commands));
    }

    fn title(&self) -> String {
        "BoardNew".to_owned()
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

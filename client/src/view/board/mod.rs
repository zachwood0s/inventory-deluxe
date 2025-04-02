use board_render::{BoardRender, Grid, RenderContext};
use common::board::{BoardPiece, BoardPieceSet, GridSnap, PieceId};
use egui::{Frame, Pos2, Rect, Response, Vec2};
use properties_window::{PropertiesCtx, PropertiesDisplay};

use crate::{
    listener::CommandQueue,
    state::{
        board::{self, commands::AddOrUpdatePiece},
        DndState,
    },
};

use super::DndTabImpl;

pub mod board_render;
pub mod properties_window;

pub trait SnapToGrid: Sized {
    fn snap_to_grid(self, grid_snap: GridSnap) -> Self;
    fn snap_to_grid_up(self, grid_snap: GridSnap) -> Self;
}

impl SnapToGrid for Pos2 {
    fn snap_to_grid(self, grid_snap: GridSnap) -> Self {
        match grid_snap {
            GridSnap::MajorSpacing(spacing) => (self / spacing).floor() * spacing,
            GridSnap::None => self,
        }
    }

    fn snap_to_grid_up(self, grid_snap: GridSnap) -> Self {
        match grid_snap {
            GridSnap::MajorSpacing(spacing) => (self / spacing).ceil() * spacing,
            GridSnap::None => self,
        }
    }
}

impl SnapToGrid for Vec2 {
    fn snap_to_grid(self, grid_snap: GridSnap) -> Self {
        self.to_pos2().snap_to_grid(grid_snap).to_vec2()
    }
    fn snap_to_grid_up(self, grid_snap: GridSnap) -> Self {
        self.to_pos2().snap_to_grid_up(grid_snap).to_vec2()
    }
}

#[derive(Default, Clone, Copy)]
pub struct SelectionState {
    view_properties: bool,
    selected: Option<PieceId>,
    dragged: Option<PieceId>,
}

#[derive(Default, Clone, Copy)]
pub struct InputState {
    have_interract_mouse_input: bool,
    board_mouse_pos: Pos2,
    screen_mouse_pos: Pos2,
}

pub struct DragState {
    id: PieceId,
    object_offset: Vec2,
}

pub struct UiBoardState {
    grid: Grid,

    selection: SelectionState,
    drag_state: Option<DragState>,
    input: InputState,

    view_origin: Pos2,
    zoom: f32,
}

impl Default for UiBoardState {
    fn default() -> Self {
        Self {
            grid: Grid::new(0.1),

            selection: SelectionState::default(),
            drag_state: None,
            input: InputState::default(),

            view_origin: Pos2::default(),
            zoom: 1.0,
        }
    }
}

impl UiBoardState {
    fn clear_selected(&mut self) {
        self.selection.selected = None;
        self.selection.view_properties = false;
    }

    fn clear_dragging(&mut self) {
        self.drag_state = None;
    }

    fn get_selected_piece<'a>(&self, piece_set: &'a BoardPieceSet) -> Option<&'a BoardPiece> {
        let selected = self.selection.selected?;
        piece_set.get_piece(&selected)
    }

    fn handle_view_props(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &mut PropertiesCtx,
        piece_set: &mut BoardPieceSet,
        changed_set: &mut Vec<PieceId>,
    ) {
        if !self.selection.view_properties {
            return;
        }

        let Some(piece_id) = self.selection.selected else {
            return;
        };

        if let Some(piece) = piece_set.get_piece_mut(&piece_id) {
            piece.display_props(ui, ctx);

            if ctx.changed {
                changed_set.push(piece.id);
            }
        }
    }

    fn handle_dragging(&mut self, piece_set: &mut BoardPieceSet, changed_set: &mut Vec<PieceId>) {
        let Some(dragged) = &self.drag_state else {
            return;
        };

        let Some(piece) = piece_set.get_piece_mut(&dragged.id) else {
            self.clear_dragging();
            return;
        };

        // If this piece is now locked, stop the dragging
        if piece.locked {
            self.clear_dragging();
            return;
        }

        let new_pos = self.input.board_mouse_pos.snap_to_grid(piece.snap_to_grid)
            + dragged.object_offset.snap_to_grid_up(piece.snap_to_grid);

        piece.rect = Rect::from_min_size(new_pos, piece.rect.size());

        changed_set.push(piece.id);
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

        if let Some(pos) = response.hover_pos() {
            self.input.screen_mouse_pos = pos;
            self.input.board_mouse_pos = ctx.to_grid * (ctx.from_screen * pos);
        }

        self.input.have_interract_mouse_input = response.interact_pointer_pos().is_some();

        // Handle Dragging
        if response.dragged_by(egui::PointerButton::Middle) {
            let screen_origin = ctx.to_screen * self.view_origin;
            self.view_origin = ctx.from_screen * (screen_origin - response.drag_delta());
        }

        // Handle selection of a piece, both right and left click select
        let primary_down = ctx.ui.input(|input| input.pointer.primary_down());
        let secondary_down = ctx.ui.input(|input| input.pointer.secondary_down());

        if response.contains_pointer() && (primary_down || secondary_down) {
            let selected_idx = piece_set.get_topmost_piece_at_position(self.input.board_mouse_pos);

            if selected_idx.is_some() {
                self.selection.selected = selected_idx.copied();
            } else if self.drag_state.is_none() {
                // Only clear the selected if we're not dragging
                self.clear_selected();
            }
        }

        let selected = self.get_selected_piece(piece_set);

        // We have an item selected, and we haven't started dragging
        if response.dragged_by(egui::PointerButton::Primary)
            && selected.is_some()
            && self.drag_state.is_none()
        {
            // SAFE: we just checked if it was Some first
            let selected = selected.unwrap();
            let object_offset = selected.rect.min - self.input.board_mouse_pos;

            // Start the drag
            self.drag_state = Some(DragState {
                id: selected.id,
                object_offset,
            })
        }

        // No mouse input, stop dragging
        if !self.input.have_interract_mouse_input {
            self.drag_state = None;
        }

        if ctx.ui.input(|input| input.key_pressed(egui::Key::Delete)) {
            if let Some(id) = self.selection.selected {
                commands.add(board::commands::DeletePiece(id));
            }
        }

        response.context_menu(|ui| {
            ui.set_width(100.0);

            if self.selection.selected.is_some() {
                if ui.button("View Properties").clicked() {
                    self.selection.view_properties = true;
                }
            } else if ui.button("Add Piece").clicked() {
                let new_id = PieceId::default();
                self.selection.view_properties = true;
                self.selection.selected = Some(new_id);

                let new_rect = self.grid.unit_rect(self.input.board_mouse_pos);
                let new_piece = BoardPiece::from_rect(new_id, "New Piece".into(), new_rect);

                commands.add(AddOrUpdatePiece(new_piece))
            }
        });
    }

    fn snap_to_grid(&mut self, piece_set: &mut BoardPieceSet) {
        for piece in piece_set.iter_mut() {
            let new_pos = piece.rect.min.snap_to_grid(piece.snap_to_grid);
            piece.rect = Rect::from_min_size(new_pos, piece.rect.size());
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
        let mut changed_set = Vec::new();

        self.handle_dragging(&mut board.piece_set, &mut changed_set);
        self.snap_to_grid(&mut board.piece_set);

        self.handle_board_input(&mut ctx, &response, &board.piece_set, commands);
        self.grid.render(&ctx);
        board.piece_set.render(&ctx);

        let mut ctx = PropertiesCtx {
            state,
            changed: false,
        };

        self.handle_view_props(ui, &mut ctx, &mut board.piece_set, &mut changed_set);

        // Send out all the updates for the pieces that were modified
        for piece_id in changed_set {
            if let Some(piece) = board.piece_set.get_piece(&piece_id) {
                commands.add(AddOrUpdatePiece(piece.clone()))
            }
        }

        response
    }
}

impl DndTabImpl for UiBoardState {
    fn ui(&mut self, ui: &mut egui::Ui, state: &DndState, commands: &mut CommandQueue) {
        Frame::canvas(ui.style()).show(ui, |ui| self.ui_content(ui, state, commands));
    }

    fn title(&self) -> String {
        "Board".to_owned()
    }
}

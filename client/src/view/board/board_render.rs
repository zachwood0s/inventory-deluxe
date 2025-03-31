use common::board::{BoardPiece, BoardPieceData, BoardPieceSet, PlayerPieceData};
use egui::{
    epaint::PathStroke, Color32, Image, Painter, Pos2, Rect, Rounding, Shape, Stroke,
    TextureOptions, Ui, Vec2,
};
use emath::RectTransform;
use log::info;

use super::SelectionState;

pub struct RenderContext<'r> {
    pub ui: &'r mut egui::Ui,
    pub painter: Painter,
    pub selection_state: SelectionState,
    pub from_grid: RectTransform,
    pub to_grid: RectTransform,
    pub to_screen: RectTransform,
    pub from_screen: RectTransform,
    pub render_dimensions: Vec2,
}

pub trait BoardRender {
    fn render(&self, render_context: &RenderContext);
}

impl BoardRender for BoardPiece {
    fn render(&self, ctx: &RenderContext) {
        let transformed = ctx.from_grid.transform_rect(self.rect);
        let transformed = ctx.to_screen.transform_rect(transformed);

        let mut alpha = u8::MAX;
        if Some(self.id) == ctx.selection_state.dragged {
            alpha /= 10;
        }

        if let Some(url) = &self.image_url {
            Image::new(url)
                .texture_options(
                    TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear)),
                )
                .tint(Color32::from_white_alpha(alpha))
                .paint_at(ctx.ui, transformed);
        } else {
            ctx.painter.rect_filled(
                transformed,
                Rounding::ZERO,
                Color32::from_white_alpha(alpha),
            );
        }

        if Some(self.id) == ctx.selection_state.selected {
            ctx.painter.rect_stroke(
                transformed,
                Rounding::ZERO,
                Stroke::new(3.0, Color32::LIGHT_RED),
            );
        }

        match &self.data {
            BoardPieceData::Player(data) => data.render(ctx, self),
            BoardPieceData::None => {}
        }
    }
}

pub trait ChildRender {
    #[allow(unused)]
    fn render(&self, render_context: &RenderContext, parent: &BoardPiece) {}
}

impl ChildRender for PlayerPieceData {
    fn render(&self, render_context: &RenderContext, parent: &BoardPiece) {}
}

pub struct Grid {
    grid_size: f32,
    visible: bool,
}

impl Grid {
    pub fn new(grid_size: f32) -> Self {
        Self {
            grid_size,
            visible: true,
        }
    }

    pub fn unit_rect(&self, top_left: Pos2) -> Rect {
        Rect::from_two_pos(top_left, top_left + Vec2::new(1.0, 1.0))
    }

    pub fn from_grid(&self) -> RectTransform {
        RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, Vec2::new(1.0, 1.0)),
            Rect::from_min_size(Pos2::ZERO, Vec2::new(self.grid_size, self.grid_size)),
        )
    }
}

impl BoardRender for Grid {
    fn render(&self, ctx: &RenderContext) {
        if !self.visible {
            // Nothing to render in this case
            return;
        }

        let dims = ctx.render_dimensions;
        let grid_origin = ctx.to_screen.from().center();

        let num_x = (dims.x / self.grid_size) as i32 + 1;
        let num_y = (dims.y / self.grid_size) as i32 + 1;

        let topleft_boundary = grid_origin - dims / 2.0;

        let round = topleft_boundary.y.rem_euclid(self.grid_size);
        let y_start = topleft_boundary.y - round;
        for y in (0..num_y).map(|x| x as f32 * self.grid_size + y_start) {
            ctx.painter.add(Shape::line_segment(
                [
                    ctx.to_screen * Pos2::new(-dims.x + grid_origin.x, y),
                    ctx.to_screen * Pos2::new(dims.x + grid_origin.x, y),
                ],
                PathStroke::new(1.0, Color32::DARK_GRAY),
            ));
        }

        let round = topleft_boundary.x.rem_euclid(self.grid_size);
        let x_start = topleft_boundary.x - round;
        for x in (0..num_x).map(|x| x as f32 * self.grid_size + x_start) {
            ctx.painter.add(Shape::line_segment(
                [
                    ctx.to_screen * Pos2::new(x, -dims.y + grid_origin.y),
                    ctx.to_screen * Pos2::new(x, dims.y + grid_origin.y),
                ],
                PathStroke::new(1.0, Color32::DARK_GRAY),
            ));
        }
    }
}

impl BoardRender for BoardPieceSet {
    fn render(&self, render_context: &RenderContext) {
        for piece in self.sorted_iter() {
            piece.render(render_context);
        }
    }
}

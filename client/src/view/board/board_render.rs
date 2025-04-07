use core::f32;

use common::board::{BoardPiece, BoardPieceData, BoardPieceSet, CharacterPieceData};
use egui::{
    epaint::{CornerRadiusF32, PathStroke},
    vec2, Color32, CornerRadius, Image, Painter, Pos2, Rect, Rgba, Rounding, Shape, Stroke,
    TextStyle, TextureOptions, Vec2,
};
use emath::RectTransform;
use log::info;

use crate::{state::DndState, widgets::WithAlpha};

use super::SelectionState;

pub struct RenderContext<'r> {
    pub ui: &'r mut egui::Ui,
    pub painter: Painter,
    pub state: &'r DndState,
    pub selection_state: SelectionState,
    pub from_grid: RectTransform,
    pub to_grid: RectTransform,
    pub to_screen: RectTransform,
    pub from_screen: RectTransform,
    pub render_dimensions: Vec2,
    pub ui_opacity: f32,
    pub changed: bool,
}

impl RenderContext<'_> {
    pub fn to_screen(&self, rect: Rect) -> Rect {
        let new_rect = self.from_grid.transform_rect(rect);
        self.to_screen.transform_rect(new_rect)
    }
}

pub trait BoardRender {
    fn render(&self, render_context: &mut RenderContext);
}

impl BoardRender for BoardPiece {
    fn render(&self, ctx: &mut RenderContext) {
        let transformed = ctx.to_screen(self.rect);

        let mut alpha = 1.0;
        if Some(self.id) == ctx.selection_state.dragged {
            alpha /= 10.0;
        }

        let mut color = Rgba::from_rgba_unmultiplied(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        );

        color[3] = alpha;

        if !self.image_url.is_empty() {
            Image::new(&self.image_url)
                .texture_options(
                    TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear)),
                )
                .tint(Color32::from(color))
                .paint_at(ctx.ui, transformed);
        } else {
            ctx.painter
                .rect_filled(transformed, CornerRadius::ZERO, Color32::from(color));
        }

        if Some(self.id) == ctx.selection_state.selected {
            ctx.painter.rect_stroke(
                transformed,
                CornerRadius::ZERO,
                Stroke::new(3.0, Color32::LIGHT_RED),
                egui::StrokeKind::Outside,
            );
        }

        if self.display_name {
            let font = TextStyle::Body.resolve(ctx.ui.style());

            let text_color = Rgba::from_white_alpha(ctx.ui_opacity).into();

            let galley = ctx
                .painter
                .layout(self.name.clone(), font, text_color, f32::INFINITY);

            let anchor = egui::Align2::CENTER_CENTER;
            let text_rect = anchor.anchor_size(transformed.center_bottom(), galley.size());
            let box_rect = text_rect.expand(2.0);

            // Faint black box behind the text
            ctx.painter.rect_filled(
                box_rect,
                CornerRadiusF32::same(2.0),
                Rgba::from_rgba_unmultiplied(0.0, 0.0, 0.0, 0.7 * ctx.ui_opacity),
            );

            ctx.painter.galley(text_rect.min, galley, text_color);
        }

        match &self.data {
            BoardPieceData::Character(data) => data.render(ctx, self),
            BoardPieceData::None => {}
        }
    }
}

pub trait ChildRender {
    #[allow(unused)]
    fn render(&self, render_context: &RenderContext, parent: &BoardPiece) {}
}

impl ChildRender for CharacterPieceData {
    fn render(&self, ctx: &RenderContext, piece: &BoardPiece) {
        let linked = self
            .link_stats_to
            .as_ref()
            .and_then(|x| ctx.state.character.characters.get(x));

        if let Some(linked) = linked {
            let filled_hp_perc = linked.curr_hp as f32 / linked.max_hp as f32;

            // Render healthbar
            let transformed = ctx.to_screen(piece.rect);
            let health_pos = transformed.center_bottom() + vec2(0.0, 15.0);
            let health_bar_rect = Rect::from_center_size(health_pos, vec2(100.0, 8.0));
            let filled_rect = Rect::from_min_size(
                health_bar_rect.min,
                health_bar_rect.size() * vec2(filled_hp_perc, 1.0),
            );

            let stroke_color = Rgba::from_black_alpha(ctx.ui_opacity);
            let background_color = Rgba::from_black_alpha(0.7 * ctx.ui_opacity);
            let fill_color = if filled_hp_perc >= 0.5 {
                Color32::GREEN
            } else if filled_hp_perc >= 0.1 {
                Color32::YELLOW
            } else {
                Color32::RED
            }
            .gamma_multiply(ctx.ui_opacity);

            let rounding = CornerRadius::ZERO;

            ctx.painter.rect(
                health_bar_rect,
                rounding,
                background_color,
                Stroke::new(2.0, stroke_color),
                egui::StrokeKind::Outside,
            );

            ctx.painter.rect_filled(filled_rect, rounding, fill_color);
        }
    }
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
    fn render(&self, ctx: &mut RenderContext) {
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
                Stroke::new(1.0, Color32::DARK_GRAY),
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
                Stroke::new(1.0, Color32::DARK_GRAY),
            ));
        }
    }
}

impl BoardRender for BoardPieceSet {
    fn render(&self, render_context: &mut RenderContext) {
        for piece in self.sorted_iter() {
            piece.render(render_context);
        }
    }
}

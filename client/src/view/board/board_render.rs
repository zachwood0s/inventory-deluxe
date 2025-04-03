use core::f32;
use std::sync::Arc;

use common::board::{BoardPiece, BoardPieceData, BoardPieceSet, CharacterPieceData};
use egui::{
    epaint::PathStroke, Button, Color32, FontId, Galley, Id, Image, Painter, Pos2, Rect, Response,
    Rgba, RichText, Rounding, Shape, Stroke, TextStyle, TextureOptions, Vec2, Widget, Window,
};
use emath::RectTransform;
use log::info;

use crate::widgets::group::Group;

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
    pub ui_opacity: f32,
}

pub trait BoardRender {
    fn render(&self, render_context: &mut RenderContext);
}

impl BoardRender for BoardPiece {
    fn render(&self, ctx: &mut RenderContext) {
        let transformed = ctx.from_grid.transform_rect(self.rect);
        let transformed = ctx.to_screen.transform_rect(transformed);

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
                .rect_filled(transformed, Rounding::ZERO, Color32::from(color));
        }

        if Some(self.id) == ctx.selection_state.selected {
            ctx.painter.rect_stroke(
                transformed,
                Rounding::ZERO,
                Stroke::new(3.0, Color32::LIGHT_RED),
            );

            // Move to front/back selection icons
            const SIDE_WIDTH: f32 = 25.0;

            // Expand vertically to account for when the piece is small
            let mut side_rect = transformed
                .translate(Vec2::new(-SIDE_WIDTH, 0.0))
                .expand2(Vec2::new(0.0, 20.0));
            side_rect.set_width(SIDE_WIDTH);

            let new_ui = egui::UiBuilder::new()
                .layer_id(egui::LayerId::new(
                    egui::Order::Middle,
                    Id::new("render_button"),
                ))
                .max_rect(side_rect);

            ctx.ui.scope_builder(new_ui, |ui| {
                ui.set_opacity(ctx.ui_opacity);
                ui.horizontal_centered(|ui| {
                    Group::new("inner_group").show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            if Button::new(egui_phosphor::regular::ARROW_LINE_UP)
                                .ui(ui)
                                .clicked()
                            {
                                info!("Clicked2");
                            }
                            if Button::new(egui_phosphor::regular::ARROW_LINE_DOWN)
                                .ui(ui)
                                .clicked()
                            {
                                info!("Clicked1");
                            }
                        })
                    })
                })
            });
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
                Rounding::same(2.0),
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
    fn render(&self, _: &RenderContext, _: &BoardPiece) {}
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
    fn render(&self, render_context: &mut RenderContext) {
        for piece in self.sorted_iter() {
            piece.render(render_context);
        }
    }
}

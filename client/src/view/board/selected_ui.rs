use common::board::{BoardPiece, BoardPieceSet, LayerInfo};
use egui::{Button, Id, Vec2, Widget};
use log::info;

use crate::{state::DndState, widgets::group::Group};

use super::board_render::RenderContext;

pub trait SelectedUi {
    fn ui(&mut self, ctx: &mut RenderContext, state: &DndState, layer_info: &LayerInfo);
}

impl SelectedUi for BoardPiece {
    fn ui(&mut self, ctx: &mut RenderContext, state: &DndState, layer_info: &LayerInfo) {
        let transformed = ctx.from_grid.transform_rect(self.rect);
        let transformed = ctx.to_screen.transform_rect(transformed);

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
                            self.sorting_layer = layer_info.next_highest_layer;
                        }
                        if Button::new(egui_phosphor::regular::ARROW_LINE_DOWN)
                            .ui(ui)
                            .clicked()
                        {
                            self.sorting_layer = layer_info.next_lowest_layer;
                        }
                    })
                })
            })
        });
    }
}

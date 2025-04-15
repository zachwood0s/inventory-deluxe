use egui_extras::{Column, TableBuilder};
use itertools::Itertools;

use crate::widgets::No as _;

use super::CharacterTabImpl;

#[derive(Clone)]
pub struct InventoryTab;

impl CharacterTabImpl for InventoryTab {
    fn ui(&self, ui: &mut egui::Ui, ctx: super::CharacterCtx) {
        let all_items = ctx.character.items(&ctx.state.data).collect_vec();

        TableBuilder::new(ui)
            .column(Column::auto())
            .columns(Column::auto(), 4)
            .auto_shrink(false)
            .resizable(true)
            .striped(true)
            .header(20.0, |mut row| {
                row.col(|ui| ui.strong("Weapons").no());
                row.col(|ui| ui.strong("Weight").no());
                row.col(|ui| ui.strong("Charges").no());
                row.col(|ui| ui.strong("Usage").no());
                row.col(|ui| ui.strong("+ Add").no());
            })
            .body(|body| {
                body.rows(18.0, 100, |mut row| {
                    row.col(|ui| ui.label("Scaboink").no());
                    row.col(|ui| ui.label("20 lbs.").no());
                    row.col(|ui| ui.label("").no());
                    row.col(|ui| ui.label("1 Action").no());
                    row.col(|ui| ui.label("<<actions>>").no());
                })
            });

        ui.label("inventory");
    }

    fn title(&self) -> &str {
        "INVENTORY"
    }
}

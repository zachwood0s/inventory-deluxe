use std::collections::HashSet;

use common::{
    data_store::{self, CharacterStorage, ItemHandle, ItemRef},
    ItemId,
};
use egui::UiBuilder;
use egui_dnd::utils::shift_vec;
use egui_extras::{Column, TableBuilder};
use itertools::Itertools;
use log::info;

use crate::widgets::No as _;

use super::{CharacterCtx, CharacterTabImpl};

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct State {
    sorted_items: Vec<ItemHandle>,
}

impl State {
    fn load(ui: &mut egui::Ui, id: egui::Id, ctx: &CharacterCtx) -> Self {
        let mut state: State = ui.data_mut(|mem| mem.get_persisted(id).unwrap_or_default());

        let all_items: HashSet<_> = ctx
            .character
            .items(&ctx.state.data)
            .map(|x| x.handle)
            .collect();

        // Get rid of any items we no longer care about
        state.sorted_items.retain(|x| all_items.contains(x));

        // Add all items onto the end and then "dedup". Dedup should remove any of the existing
        // items which were re-added (keeping the order) and add any new items to the end of the
        // list
        state.sorted_items.extend(all_items);

        state.sorted_items = state.sorted_items.into_iter().unique().collect_vec();

        state
    }

    fn store(self, ui: &mut egui::Ui, id: egui::Id) {
        ui.data_mut(|mem| mem.insert_persisted(id, self))
    }
}

#[derive(Clone)]
pub struct InventoryTab;

impl CharacterTabImpl for InventoryTab {
    fn ui(&self, ui: &mut egui::Ui, ctx: CharacterCtx) {
        let salt = egui::Id::new(ctx.character.name()).with("inventory");
        let id = ui.make_persistent_id(salt);
        let mut state = State::load(ui, id, &ctx);

        let mut filtered = state
            .sorted_items
            .iter_mut()
            .flat_map(|handle| {
                Some(ItemRef {
                    handle: *handle,
                    item: ctx.state.data.get_item(&handle.item)?,
                })
            })
            .enumerate()
            .collect_vec();

        //.into_group_map_by(|(_, f)| f.item.category);

        //let keys = item_map.keys().copied().sorted().collect_vec();
        //for key in keys {
        //let items = item_map.get_mut(&key).unwrap();

        let response =
            egui_dnd::dnd(ui, id).show(filtered.iter_mut(), |ui, (_, item), handle, _dragging| {
                ui.horizontal(|ui| {
                    handle.ui(ui, |ui| {
                        ui.label(&item.item.name);
                    })
                });
            });

        if let Some(update) = response.final_update() {
            info!("{update:?}");
            let (original_index_from, _) = filtered[update.from];
            let (original_index_to, _) = filtered[update.to];

            shift_vec(
                original_index_from,
                original_index_to,
                &mut state.sorted_items,
            );
        }
        /*
        ui.scope_builder(UiBuilder::new().id_salt(key), |ui| {
            TableBuilder::new(ui)
                .id_salt(key)
                .column(Column::remainder())
                .columns(Column::auto(), 3)
                .column(Column::exact(100.0))
                .auto_shrink([false, true])
                .striped(true)
                .header(20.0, |mut row| {
                    row.col(|ui| ui.strong(key.to_string()).no());
                    row.col(|ui| ui.strong("Weight").no());
                    row.col(|ui| ui.strong("Charges").no());
                    row.col(|ui| ui.strong("Usage").no());
                    row.col(|ui| ui.strong("+ Add").no());
                })
                .body(|body| {
                    body.rows(18.0, items.len(), |mut row| {
                        let ItemRef { handle, item } = &items[row.index()];

                        row.col(|ui| ui.label(&item.name).no());
                        row.col(|ui| ui.label("20 lbs.").no());
                        row.col(|ui| ui.label("").no());
                        row.col(|ui| ui.label("").no());
                        row.col(|ui| ui.label("<<actions>>").no());
                    })
                });
        });
        */

        state.store(ui, id);
    }

    fn title(&self) -> &str {
        "INVENTORY"
    }
}

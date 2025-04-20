use std::collections::HashSet;

use common::{
    data_store::{self, CharacterStorage, ItemHandle, ItemRef},
    ItemCategory, ItemId,
};
use egui::{
    vec2, Button, Color32, ComboBox, Frame, Grid, Layout, RichText, Separator, Stroke, UiBuilder,
    Widget,
};
use egui_dnd::utils::shift_vec;
use egui_extras::{Column, Size, Strip, StripBuilder, TableBuilder};
use itertools::Itertools;
use log::info;

use crate::{
    state::character::commands::UpdateItemHandle,
    widgets::{CustomUi, EnumSelect, No},
};

use super::{CharacterCtx, CharacterTabImpl};

#[derive(
    Default,
    Clone,
    Copy,
    Eq,
    PartialEq,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
    strum_macros::EnumIter,
)]
enum EquippedState {
    #[default]
    Any,
    Equipped,
    #[display("Not Equipped")]
    NotEquipped,
}

#[derive(
    Default,
    Clone,
    Copy,
    Eq,
    PartialEq,
    derive_more::Display,
    serde::Serialize,
    serde::Deserialize,
    strum_macros::EnumIter,
)]
enum AttunedState {
    #[default]
    Any,
    Attuned,
    #[display("Not Attuned")]
    NotAttuned,
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct Filters {
    equipped: EquippedState,
    attuned: AttunedState,
    show_all: bool,
    weapons: bool,
    equipment: bool,
    consumables: bool,
    valuables: bool,
    misc: bool,
}

impl Filters {
    fn handle_show_all(&mut self) {
        if self.show_all {
            self.set_all(true);
        }

        // If we've deselected show_all and all are currently selected, disable them all
        if !self.show_all && self.iter().all(|(_, filter)| *filter) {
            self.set_all(false);
        }
    }

    fn set_all(&mut self, value: bool) {
        self.iter().for_each(|(_, filter)| *filter = value);
    }

    fn iter(&mut self) -> impl Iterator<Item = (&str, &mut bool)> {
        [
            ("Weapons", &mut self.weapons),
            ("Equipment", &mut self.equipment),
            ("Consumables", &mut self.consumables),
            ("Valuables", &mut self.valuables),
            ("Misc", &mut self.misc),
        ]
        .into_iter()
    }

    fn matches(&self, item_ref: &ItemRef) -> bool {
        match self.equipped {
            EquippedState::Equipped if !item_ref.handle.equipped => return false,
            EquippedState::NotEquipped if item_ref.handle.equipped => return false,
            _ => {}
        }

        match self.attuned {
            AttunedState::Attuned if !item_ref.handle.attuned => return false,
            AttunedState::NotAttuned if item_ref.handle.attuned => return false,
            _ => {}
        }

        match item_ref.item.category {
            ItemCategory::Weapons => self.weapons,
            ItemCategory::Equipment => self.equipment,
            ItemCategory::Consumables => self.consumables,
            ItemCategory::Valuables => self.valuables,
            ItemCategory::Misc => self.misc,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct State {
    sorted_items: Vec<ItemId>,
    filters: Filters,
}

impl Default for State {
    fn default() -> Self {
        let mut filters = Filters::default();
        filters.set_all(true);
        filters.show_all = true;

        Self {
            sorted_items: Default::default(),
            filters,
        }
    }
}

impl State {
    fn load(ui: &mut egui::Ui, id: egui::Id, ctx: &CharacterCtx) -> Self {
        let mut state: State = ui.data_mut(|mem| mem.get_persisted(id).unwrap_or_default());

        let all_items: HashSet<_> = ctx
            .character
            .items(&ctx.state.data)
            .map(|x| x.handle.item)
            .collect();

        // Get rid of any items we no longer care about
        state.sorted_items.retain(|x| all_items.contains(x));

        // Add all items onto the end and then "dedup". Dedup should remove any of the existing
        // items which were re-added (keeping the order) and add any new items to the end of the
        // list
        state.sorted_items.extend(all_items.iter());

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
        let style = ui.style().clone();
        let salt = egui::Id::new(ctx.character.name()).with("inventory");
        let id = ui.make_persistent_id(salt);
        let mut state = State::load(ui, id, &ctx);

        ui.menu_button("Filter", |ui| {
            egui::Grid::new("dropdowns").num_columns(2).show(ui, |ui| {
                ui.label("Equipped");
                ui.add(EnumSelect::new(&mut state.filters.equipped, ""));
                ui.end_row();
                ui.label("Attuned");
                ui.add(EnumSelect::new(&mut state.filters.attuned, ""));
            });

            ui.separator();

            if ui
                .checkbox(&mut state.filters.show_all, "Show All")
                .clicked()
            {
                state.filters.handle_show_all();
            }

            for (name, state) in state.filters.iter() {
                ui.checkbox(state, name);
            }
        });

        let mut filtered = state
            .sorted_items
            .iter_mut()
            .flat_map(|item_id| {
                Some(ItemRef {
                    handle: *ctx.character.get_item(item_id).unwrap(),
                    item: ctx.state.data.get_item(item_id)?,
                })
            })
            .enumerate()
            .filter(|(_, x)| state.filters.matches(x))
            .collect_vec();

        let item_size = vec2(ui.available_width(), 32.0);

        ui.allocate_ui(item_size, |ui| {
            ItemRow::new()
                .frame(
                    Frame::new()
                        .stroke(style.visuals.window_stroke)
                        .fill(style.visuals.faint_bg_color)
                        .inner_margin(4),
                )
                .layout(Layout::left_to_right(egui::Align::Center))
                .show(ui, |strip| {
                    let headers = ["Name", "Weight", "Quantity", "Actions"];

                    for (idx, h) in headers.into_iter().enumerate() {
                        strip.cell(|ui| {
                            if idx != 0 {
                                ui.add(Separator::default().vertical());
                            }
                            ui.centered_and_justified(|ui| ui.label(h).no());
                        });
                    }
                });
        });

        let response =
            egui_dnd::dnd(ui, id).show(filtered.iter_mut(), |ui, (_, item), handle, _dragging| {
                let mut new_handle = item.handle;

                ui.horizontal(|ui| {
                    handle.ui_sized(ui, item_size, |ui| {
                        ItemRow::new()
                            .frame(
                                Frame::new()
                                    .stroke(style.visuals.window_stroke)
                                    .inner_margin(4),
                            )
                            .show(ui, |strip| {
                                strip.cell(|ui| {
                                    ui.label(&item.item.name);

                                    if item.item.quest_item {
                                        ui.attribute("QUEST ITEM", Color32::YELLOW);
                                    }

                                    if item.handle.equipped {
                                        ui.attribute("EQUIPPED", Color32::GREEN);
                                    }

                                    if item.handle.attuned {
                                        ui.attribute("ATTUNED", Color32::ORANGE);
                                    }
                                });
                                strip.cell(|ui| {
                                    if let Some(weight) = item.item.weight {
                                        ui.add(Separator::default().vertical());
                                        ui.centered_and_justified(|ui| {
                                            ui.label(format!("{:.2} lbs.", weight));
                                        });
                                    }
                                });
                                strip.cell(|ui| {
                                    ui.add(Separator::default().vertical());
                                    ui.centered_and_justified(|ui| {
                                        ui.label(item.handle.count.to_string());
                                    });
                                });
                                strip.cell(|ui| {
                                    use egui_phosphor::regular::*;

                                    ui.add(Separator::default().vertical());

                                    ui.add_enabled(
                                        item.item.equippable,
                                        ToggleIcon::new(
                                            &mut new_handle.equipped,
                                            SHIELD_CHECK,
                                            SHIELD,
                                            SHIELD_SLASH,
                                        )
                                        .hover("Equip?"),
                                    );

                                    ui.add_enabled(
                                        item.item.requires_attunement,
                                        ToggleIcon::new(
                                            &mut new_handle.attuned,
                                            EYE,
                                            EYE_CLOSED,
                                            EYE_SLASH,
                                        )
                                        .hover("Attune?"),
                                    );

                                    if ui.button(TRASH).on_hover_text("Delete").clicked() {
                                        info!("Delte!");
                                    }

                                    ui.add(Separator::default().vertical());

                                    if ui.button(PENCIL_LINE).on_hover_text("Edit").clicked() {
                                        info!("Show edit!");
                                    }

                                    if ui.button(INFO).on_hover_text("Info").clicked() {
                                        info!("Show info!");
                                    }
                                });
                            });
                    })
                });

                if new_handle != item.handle {
                    ctx.commands
                        .add(UpdateItemHandle::new(ctx.state.owned_user(), new_handle));
                }
            });

        if let Some(update) = response.final_update() {
            let (original_index_from, _) = filtered[update.from];
            let original_index_to = filtered
                .get(update.to)
                .map(|(o, _)| *o)
                .unwrap_or_else(|| state.sorted_items.len());

            shift_vec(
                original_index_from,
                original_index_to,
                &mut state.sorted_items,
            );
        }

        state.store(ui, id);
    }

    fn title(&self) -> &str {
        "INVENTORY"
    }
}

struct ItemRow {
    frame: Option<Frame>,
    layout: Option<Layout>,
}

impl ItemRow {
    fn new() -> Self {
        Self {
            frame: None,
            layout: None,
        }
    }

    fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    fn layout(mut self, layout: Layout) -> Self {
        self.layout = Some(layout);
        self
    }

    fn show(self, ui: &mut egui::Ui, add_contents: impl FnOnce(&mut Strip)) {
        self.frame.unwrap_or_default().show(ui, |ui| {
            let layout = self.layout.unwrap_or_else(|| *ui.layout());
            StripBuilder::new(ui)
                .size(Size::remainder().at_least(200.0))
                .sizes(Size::exact(100.0), 2)
                .size(Size::exact(170.0))
                .cell_layout(layout)
                .horizontal(|mut strip| add_contents(&mut strip));
        });
    }
}

struct ToggleIcon<'a> {
    toggle_value: &'a mut bool,
    on_icon: &'a str,
    off_icon: &'a str,
    disable_icon: &'a str,
    hover_tooltip: Option<&'static str>,
}

impl<'a> ToggleIcon<'a> {
    pub fn new(
        toggle_value: &'a mut bool,
        on_icon: &'a str,
        off_icon: &'a str,
        disable_icon: &'a str,
    ) -> Self {
        Self {
            toggle_value,
            on_icon,
            off_icon,
            disable_icon,
            hover_tooltip: None,
        }
    }

    pub fn hover(mut self, hover_tooltip: &'static str) -> Self {
        self.hover_tooltip = Some(hover_tooltip);
        self
    }
}

impl Widget for ToggleIcon<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let icon = if !ui.is_enabled() {
            self.disable_icon
        } else if *self.toggle_value {
            self.on_icon
        } else {
            self.off_icon
        };

        let mut resp = ui.add(Button::new(icon));

        if let Some(hover_tooltip) = self.hover_tooltip {
            resp = resp.on_hover_text(hover_tooltip);
        }

        if resp.clicked() {
            *self.toggle_value = !*self.toggle_value;
        }

        resp
    }
}

use std::collections::HashSet;

use common::{data_store::AbilityRef, AbilityId, AbilitySource};
use egui::{vec2, Color32, Frame, Layout, Separator};
use egui_dnd::utils::shift_vec;
use egui_extras::{Size, Strip, StripBuilder};
use fuzzy_matcher::FuzzyMatcher;
use itertools::Itertools;
use log::info;

use crate::{
    state::commands::EditAbility,
    widgets::{CustomUi, No, ToggleIcon},
};

use super::{CharacterCtx, CharacterTabImpl};

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct Filters {
    show_all: bool,
    passive: bool,
    bonus_action: bool,
    reaction: bool,
    action: bool,
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
            ("Action", &mut self.action),
            ("Bonus Action", &mut self.bonus_action),
            ("Passive", &mut self.passive),
            ("Reaction", &mut self.reaction),
        ]
        .into_iter()
    }

    fn matches(&self, ability: &AbilityRef) -> bool {
        match ability.ability.ability_type.as_str() {
            "Passive" => self.passive,
            "Bonus Action" => self.bonus_action,
            "Reaction" => self.reaction,
            "Action" => self.action,
            _ => true,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct State {
    sorted_items: Vec<AbilityId>,
    filters: Filters,
    search_text: String,
}

impl Default for State {
    fn default() -> Self {
        let mut filters = Filters::default();
        filters.set_all(true);
        filters.show_all = true;

        Self {
            sorted_items: Default::default(),
            filters,
            search_text: Default::default(),
        }
    }
}

impl State {
    fn load(ui: &mut egui::Ui, id: egui::Id, ctx: &CharacterCtx) -> Self {
        let mut state: State = ui.data_mut(|mem| mem.get_persisted(id).unwrap_or_default());

        let all_items: HashSet<_> = ctx
            .character
            .abilities(&ctx.state.data)
            .map(|x| x.handle.ability_name.clone())
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
pub struct AbilitiesTab;

impl CharacterTabImpl for AbilitiesTab {
    fn ui(&self, ui: &mut egui::Ui, ctx: super::CharacterCtx) {
        let style = ui.style().clone();
        let salt = egui::Id::new(ctx.character.name()).with("inventory");
        let id = ui.make_persistent_id(salt);
        let mut state = State::load(ui, id, &ctx);

        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

        Frame::default().outer_margin(4).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut state.search_text);

                ui.menu_button(egui_phosphor::regular::SLIDERS_HORIZONTAL, |ui| {
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
            });
        });

        let mut filtered = state
            .sorted_items
            .iter_mut()
            .flat_map(|ability_id| {
                Some(AbilityRef {
                    handle: ctx.character.get_ability(ability_id).unwrap(),
                    ability: ctx.state.data.get_ability(ability_id)?,
                })
            })
            .enumerate()
            .filter(|(_, x)| state.filters.matches(x))
            .filter(|(_, x)| {
                matcher
                    .fuzzy_match(&x.ability.name, &state.search_text)
                    .is_some()
            })
            .collect_vec();

        let item_size = vec2(ui.available_width(), 32.0);

        ui.allocate_ui(item_size, |ui| {
            AbilityRow::new(true)
                .frame(
                    Frame::new()
                        .stroke(style.visuals.window_stroke)
                        .fill(style.visuals.faint_bg_color)
                        .inner_margin(4),
                )
                .layout(Layout::left_to_right(egui::Align::Center))
                .show(ui, |strip| {
                    let headers = ["Name", "Type", "Resource", "To-Hit", "Damage", "Actions"];

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

        let response = egui_dnd::dnd(ui, id).show(
            filtered.iter_mut(),
            |ui, (_, ability), handle, _dragging| {
                let enabled = ctx
                    .character
                    .is_ability_active(&ability.handle.ability_name);

                ui.horizontal(|ui| {
                    let resp = handle.ui_sized(ui, item_size, |ui| {
                        AbilityRow::new(enabled)
                            .frame(
                                Frame::new()
                                    .stroke(style.visuals.window_stroke)
                                    .inner_margin(4),
                            )
                            .show(ui, |strip| {
                                strip.cell(|ui| {
                                    ui.add_space(10.0);

                                    ui.label(&*ability.ability.name);

                                    match ability.handle.ability_source {
                                        Some(AbilitySource::Item(_)) if enabled => {
                                            ui.attribute("ITEM", Color32::DARK_GRAY);
                                        }
                                        Some(AbilitySource::Item(_)) => {
                                            ui.attribute("ITEM: not equipped", Color32::DARK_GRAY);
                                        }
                                        None => {}
                                    }
                                });
                                strip.cell(|ui| {
                                    ui.add(Separator::default().vertical());
                                    ui.centered_and_justified(|ui| {
                                        ui.label(&ability.ability.ability_type);
                                    });
                                });
                                strip.cell(|ui| {
                                    ui.add(Separator::default().vertical());
                                    ui.centered_and_justified(|ui| {
                                        ui.label(&ability.ability.resource);
                                    });
                                });
                                strip.cell(|ui| {
                                    ui.add(Separator::default().vertical());
                                    ui.centered_and_justified(|ui| {
                                        ui.label(&ability.ability.to_hit);
                                    });
                                });
                                strip.cell(|ui| {
                                    ui.add(Separator::default().vertical());
                                    ui.centered_and_justified(|ui| {
                                        ui.label(&ability.ability.damage);
                                    });
                                });
                                strip.cell(|ui| {
                                    use egui_phosphor::regular::*;

                                    ui.add(Separator::default().vertical());

                                    if ui.button(TRASH).on_hover_text("Delete").clicked() {
                                        info!("Delte!");
                                    }

                                    ui.add(Separator::default().vertical());

                                    if ui.button(PENCIL_LINE).on_hover_text("Edit").clicked() {
                                        ctx.commands.add(EditAbility(ability.ability.name.clone()));
                                    }

                                    if ui.button(INFO).on_hover_text("Info").clicked() {
                                        info!("Show info!");
                                    }
                                });
                            });
                    });

                    resp.on_hover_ui(|ui| {
                        egui_demo_lib::easy_mark::easy_mark(ui, &ability.ability.description);
                    });
                });
            },
        );

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
        "ABILITIES"
    }
}

struct AbilityRow {
    frame: Option<Frame>,
    layout: Option<Layout>,
    enabled: bool,
}

impl AbilityRow {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
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
        ui.add_enabled_ui(self.enabled, |ui| {
            self.frame.unwrap_or_default().show(ui, |ui| {
                let layout = self.layout.unwrap_or_else(|| *ui.layout());
                StripBuilder::new(ui)
                    .size(Size::remainder().at_least(200.0))
                    .sizes(Size::exact(100.0), 3)
                    .size(Size::remainder().at_most(200.0).at_least(100.0))
                    .size(Size::exact(110.0))
                    .cell_layout(layout)
                    .horizontal(|mut strip| add_contents(&mut strip));
            });
        });
    }
}

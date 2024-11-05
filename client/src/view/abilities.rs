use core::f32;
use std::{collections::HashMap, hash::Hash};

use common::Ability;
use egui::{
    collapsing_header, epaint, vec2, Color32, DragValue, NumExt, RadioButton, Resize, RichText,
    ScrollArea, Sense, TextBuffer, Vec2, Widget,
};
use itertools::Itertools;
use log::info;

use crate::{
    listener::CommandQueue,
    state::{
        abilities::commands::{SetAbilityCount, SetPowerSlotCount},
        DndState,
    },
};

use super::DndTabImpl;

#[derive(Default)]
pub struct Abilities;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
enum IndicatorShape {
    Circle,
    Square,
}

impl<'a, T> From<T> for IndicatorShape
where
    T: Into<&'a str>,
{
    fn from(value: T) -> Self {
        match value.into() {
            "PowerSlot" => IndicatorShape::Circle,
            _ => IndicatorShape::Square,
        }
    }
}

struct Indicator {
    pub shape: IndicatorShape,
    pub filled: bool,
}

impl Widget for Indicator {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;

        let mut desired_size = egui::vec2(icon_width, 0.0);

        desired_size = desired_size.at_least(Vec2::splat(spacing.interact_size.y));
        desired_size.y = desired_size.y.max(icon_width);
        let (rect, response) =
            ui.allocate_exact_size(desired_size, Sense::focusable_noninteractive());

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);

            match self.shape {
                IndicatorShape::Circle => {
                    ui.painter().add(epaint::CircleShape::stroke(
                        big_icon_rect.center(),
                        big_icon_rect.width() / 2.0,
                        visuals.bg_stroke,
                    ));

                    if self.filled {
                        ui.painter().add(epaint::CircleShape::filled(
                            small_icon_rect.center(),
                            small_icon_rect.width() / 2.0,
                            visuals.fg_stroke.color,
                        ));
                    }
                }
                IndicatorShape::Square => {
                    ui.painter().add(epaint::RectShape::new(
                        big_icon_rect.expand(visuals.expansion),
                        visuals.rounding,
                        visuals.bg_fill,
                        visuals.bg_stroke,
                    ));

                    if self.filled {
                        ui.painter().add(epaint::RectShape::new(
                            small_icon_rect.expand(visuals.expansion),
                            visuals.rounding,
                            visuals.fg_stroke.color,
                            visuals.fg_stroke,
                        ));
                    }
                }
            }
        };

        response
    }
}

struct AbilityWidget<'a, 'c> {
    ability_idx: usize,
    state: &'a DndState,
    ability: &'a Ability,
    commands: &'a mut CommandQueue<'c>,
}

impl<'a, 'c> Widget for AbilityWidget<'a, 'c> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let ability = self.ability;
        let id = egui::Id::new(&ability.name);
        ui.group(|ui| {
            collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                .show_header(ui, |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        egui::Frame::none().show(ui, |ui| {
                            ui.label(egui::RichText::new(&ability.ability_type).size(10.0));
                        });
                        ui.label(egui::RichText::new(&ability.name).size(14.0));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        match &*self.ability.resource {
                            "UseToken" => {
                                if ui.button("Use").clicked() {
                                    self.commands.add(SetAbilityCount {
                                        ability_idx: self.ability_idx,
                                        count: ability.uses.saturating_sub(1),
                                        broadcast: true,
                                    });
                                }
                                if ui.button("Reset").clicked() {
                                    self.commands.add(SetAbilityCount {
                                        ability_idx: self.ability_idx,
                                        count: ability.max_count,
                                        broadcast: true,
                                    });
                                }

                                ui.style_mut().spacing.item_spacing = egui::vec2(2.0, 0.0);

                                let shape = (&*self.ability.resource).into();

                                for ind in 0..ability.max_count {
                                    Indicator {
                                        shape,
                                        filled: ind < ability.uses,
                                    }
                                    .ui(ui);
                                }
                            }
                            "Counter" => {
                                let mut count = ability.uses;
                                let resp = DragValue::new(&mut count)
                                    .range(i64::MIN..=ability.max_count)
                                    .update_while_editing(false)
                                    .ui(ui);

                                if resp.changed() || resp.drag_stopped() {
                                    // Only broadcast to the server once we've finished the drag process
                                    let broadcast = !resp.dragged();

                                    self.commands.add(SetAbilityCount {
                                        ability_idx: self.ability_idx,
                                        count,
                                        broadcast,
                                    });
                                }
                            }
                            "PowerSlot" => {
                                if ui.button("Use").clicked() {
                                    self.commands.add(SetPowerSlotCount {
                                        count: self
                                            .state
                                            .character
                                            .character
                                            .power_slots
                                            .saturating_sub(1),
                                    });
                                }
                            }
                            _ => {}
                        }
                    });
                })
                .body_unindented(|ui| {
                    egui_demo_lib::easy_mark::easy_mark(ui, &ability.description);

                    if let Some(notes) = ability.notes.as_ref().filter(|x| !x.is_empty()) {
                        ui.scope(|ui| {
                            ui.visuals_mut().override_text_color = Some(Color32::DARK_GRAY);
                            egui_demo_lib::easy_mark::easy_mark(ui, &format!("\n{}", notes));
                        });
                    }

                    if let Some(flavor_text) =
                        ability.flavor_text.as_ref().filter(|x| !x.is_empty())
                    {
                        egui_demo_lib::easy_mark::easy_mark(ui, &format!("\n/{}/", flavor_text));
                    }
                });
        })
        .response
    }
}

impl DndTabImpl for Abilities {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &crate::prelude::DndState,
        commands: &mut crate::listener::CommandQueue,
    ) {
        fn ability_list(
            ui: &mut egui::Ui,
            state: &crate::prelude::DndState,
            commands: &mut CommandQueue,
            abilities: &[Ability],
            ty: &str,
        ) {
            for (ability_idx, ability) in abilities.iter().enumerate() {
                if ability.ability_type != ty {
                    continue;
                }
                AbilityWidget {
                    ability_idx,
                    state,
                    ability,
                    commands,
                }
                .ui(ui);
            }
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ScrollArea::new([false, true]).show(ui, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                    ui.label("Power Slots:");

                    if ui.button("Reset").clicked() {
                        commands.add(SetPowerSlotCount { count: 3 });
                    }

                    ui.style_mut().spacing.item_spacing = egui::vec2(2.0, 0.0);

                    let shape = IndicatorShape::Circle;

                    for ind in 0..3 {
                        Indicator {
                            shape,
                            filled: ind < state.character.character.power_slots,
                        }
                        .ui(ui);
                    }
                });

                ui.heading("Passives");
                ability_list(ui, state, commands, &state.character.abilities, "Passive");

                ui.add_space(8.0);

                ui.heading("Reactions");
                ability_list(ui, state, commands, &state.character.abilities, "Reaction");

                ui.add_space(8.0);

                ui.heading("Actions");
                ability_list(
                    ui,
                    state,
                    commands,
                    &state.character.abilities,
                    "Bonus Action",
                );
                ability_list(ui, state, commands, &state.character.abilities, "Action");
                ability_list(ui, state, commands, &state.character.abilities, "Other");
            });
        });
    }

    fn title(&self) -> String {
        "Abilities".to_owned()
    }
}

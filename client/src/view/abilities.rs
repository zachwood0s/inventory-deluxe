use core::f32;
use std::hash::Hash;

use egui::{collapsing_header, epaint, vec2, NumExt, RadioButton, Resize, Sense, Vec2, Widget};

use crate::state::abilities::commands::SetAbilityCount;

use super::DndTabImpl;

#[derive(Default)]
pub struct Abilities;

enum IndicatorShape {
    Circle,
    Square
}

struct Indicator {
    pub shape: IndicatorShape,
    pub filled: bool,
}

impl Widget for Indicator {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {        
        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;
        let icon_spacing = spacing.icon_spacing;

        let mut desired_size = egui::vec2(icon_width, 0.0);

        desired_size = desired_size.at_least(Vec2::splat(spacing.interact_size.y));
        desired_size.y = desired_size.y.max(icon_width);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::focusable_noninteractive());


        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);
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

        };

        response
    }
}

impl DndTabImpl for Abilities {
    fn ui(&mut self, ui: &mut egui::Ui, state: &crate::prelude::DndState, commands: &mut crate::listener::CommandQueue) {
        egui::CentralPanel::default().show_inside(ui, |ui| {

            ui.heading("Passives");
            for a in state.character.abilities.iter() {
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.set_height(60.0);
                    egui::ScrollArea::vertical().id_source(&a.name).auto_shrink(false).show(ui, |ui| {
                        ui.label(&a.name);
                        ui.label(&a.description);
                    });
                });
            }
            
            //ui.set_width(ui.available_width() / 2.0);
            //ui.set_height(ui.available_height());
            ui.add_space(8.0);

            egui::Frame::none().show(ui, |ui| {
                
                ui.columns(2, |columns| {
                    egui::Frame::none().show(&mut columns[0], |ui| {
                        ui.heading("Actions");
                        for (ability_idx, ability) in state.character.abilities.iter().enumerate() {
                            let id = egui::Id::new(&ability.name);

                            ui.group(|ui| {
                                    collapsing_header::CollapsingState::load_with_default_open(&ui.ctx(), id, false)
                                        .show_header(ui, |ui| {
                                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                                egui::Frame::none().show(ui, |ui| {
                                                    ui.label(egui::RichText::new("Spell").size(10.0));
                                                });
                                                ui.label(egui::RichText::new(&ability.name).size(14.0));
                                            });

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button("Use").clicked() {
                                                    commands.add(SetAbilityCount { ability_idx, count: ability.uses.saturating_sub(1) });
                                                }
                                                if ui.button("Reset").clicked() {
                                                    commands.add(SetAbilityCount { ability_idx, count: ability.max_count });
                                                }

                                                ui.style_mut().spacing.item_spacing = egui::vec2(2.0, 0.0);
                                                
                                                for ind in 0..ability.max_count {
                                                    Indicator {
                                                        shape: IndicatorShape::Circle,
                                                        filled: ind < ability.uses,
                                                    }.ui(ui);
                                                }
                                            });
                                        })
                                        .body_unindented(|ui| {
                                            egui_demo_lib::easy_mark::easy_mark(ui, &ability.description);
                                        });

                                //ui.allocate_space(vec2(ui.available_width(), 1.0));
                            });


                            // ui.group(|ui| {
                            //     ui.set_width(ui.available_width());
                            //     ui.set_height(60.0);
                            //     egui::ScrollArea::vertical().id_source(&a.description).auto_shrink(false).show(ui, |ui| {
                            //         ui.label(&a.name);
                            //         ui.label(&a.description);
                            //     });
                            // });
                        }
                    });

                    egui::Frame::none().show(&mut columns[1], |ui| {
                        ui.heading("Reactions");
                        for a in state.character.items.iter() {
                            egui::ScrollArea::vertical().id_source(&a.name).auto_shrink(false).show(ui, |ui| {
                                ui.label(&a.name);
                                ui.label(&a.description);
                            });
                        };
                    });
                });
            });
        });

    }

    fn title(&self) -> String {
        "Abilities".to_owned()
    }
}
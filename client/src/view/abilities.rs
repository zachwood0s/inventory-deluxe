use core::f32;
use std::hash::Hash;

use egui::{collapsing_header, vec2, Resize, Vec2};

use super::DndTabImpl;

#[derive(Default)]
pub struct Abilities;

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
                        for a in state.character.abilities.iter() {
                            let id = egui::Id::new(&a.name);

                            ui.group(|ui| {
                                    collapsing_header::CollapsingState::load_with_default_open(&ui.ctx(), id, false)
                                        .show_header(ui, |ui| {
                                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                                egui::Frame::none().show(ui, |ui| {
                                                    ui.label(egui::RichText::new("Spell").size(10.0));
                                                });
                                                ui.label(egui::RichText::new(&a.name).size(14.0));
                                            });

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button("Use").clicked() {

                                                }
                                                if ui.button("Reset").clicked() {

                                                }
                                            });
                                        })
                                        .body_unindented(|ui| {
                                            egui_demo_lib::easy_mark::easy_mark(ui, &a.description);
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
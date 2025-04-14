use std::ops::DerefMut;

use common::data_store::CharacterStorage;
use common::{CharStat, Character, CharacterStats};
use egui::{
    CentralPanel, Color32, Frame, Image, Margin, Response, RichText, SidePanel, Stroke,
    TopBottomPanel, Vec2, Widget, Window,
};
use egui_extras::{Size, Strip, StripBuilder};

use crate::listener::CommandQueue;
use crate::state::character::commands::UpdateCharacterStats;
use crate::widgets::{group::Group, stat_tile::StatTile, CustomUi};

use super::SkillsTable;

pub struct CharacterSheetWindow<'a, 'q> {
    pub sheet: CharacterSheet<'a, 'q>,
}

impl CharacterSheetWindow<'_, '_> {
    pub fn ui(self, ui: &mut egui::Ui) {
        Window::new("Character")
            .title_bar(false)
            .default_open(true)
            .show(ui.ctx(), |ui| {
                self.sheet.ui(ui);
            });
    }
}

pub struct CharacterSheet<'a, 'q> {
    character: &'a CharacterStorage,
    commands: &'a mut CommandQueue<'q>,
}

impl<'a, 'q> CharacterSheet<'a, 'q> {
    pub fn new(character: &'a CharacterStorage, commands: &'a mut CommandQueue<'q>) -> Self {
        Self {
            character,
            commands,
        }
    }

    pub fn ui(self, ui: &mut egui::Ui) {
        let top_bar_height = 100.0;
        let stat_bar_height = 100.0;

        let mut new_stats = *self.character.stats();

        StripBuilder::new(ui)
            .size(Size::exact(top_bar_height))
            .size(Size::exact(stat_bar_height))
            .size(Size::remainder())
            .vertical(|mut strip| {
                strip.strip(|builder| {
                    TopBar::new(self.character, &mut new_stats, top_bar_height)
                        .show_in_strip(builder)
                });
                strip.strip(|builder| StatBar::new(&mut new_stats).show_in_strip(builder));
                strip.cell(|ui| {
                    ui.separator();

                    SidePanel::left("character_left")
                        .min_width(300.0)
                        .resizable(false)
                        .show_inside(ui, |ui| {
                            SkillsTable::new(self.character, self.commands).show(ui);
                        });

                    CentralPanel::default().show_inside(ui, |ui| ui.label("Scoingo"));
                });
            });

        if &new_stats != self.character.stats() {
            self.commands.add(UpdateCharacterStats::new(
                self.character.name().clone(),
                new_stats,
            ));
        }
    }
}

struct TopBar<'a> {
    character: &'a CharacterStorage,
    new_stats: &'a mut CharacterStats,
    height: f32,
    hp_width: f32,
    min_name_width: f32,
}

impl<'a> TopBar<'a> {
    pub fn new(
        character: &'a CharacterStorage,
        new_stats: &'a mut CharacterStats,
        height: f32,
    ) -> Self {
        Self {
            character,
            new_stats,
            height,
            hp_width: 250.0,
            min_name_width: 300.0,
        }
    }

    pub fn show_in_strip(self, builder: egui_extras::StripBuilder) {
        builder
            .size(Size::exact(self.height))
            .size(Size::remainder().at_least(self.min_name_width))
            .size(Size::exact(self.height))
            .size(Size::exact(self.hp_width))
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    ProfilePic::new("https://cdn.discordapp.com/attachments/1295543267231928321/1295557362551230514/th.png?ex=67f52311&is=67f3d191&hm=663ceb5f04136e4456ee988b8c97879afa1d40c98b85b7bb2d5075b418ec9420&").ui(ui);
                });
                strip.cell(|ui| {
                    ui.horizontal_centered(|ui| {
                        Group::new("character_info").show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(self.character.info().name.clone()).font(egui::FontId::proportional(30.0)));
                                ui.label(&self.character.info().tagline);
                            });
                        });
                    });
                });
                strip.cell(|ui| {
                    let frame = Frame::canvas(ui.style()).inner_margin(Margin::symmetric(0, 6)).outer_margin(5);
                    let stat = self.new_stats.get_stat_mut(CharStat::Ac);
                    StatTile::new("ARMOR", "CLASS", stat.deref_mut()).frame(frame).ui(ui);
                });
                strip.cell(|ui| {
                    ui.label("Health");
                });
            });
    }
}

struct StatBar<'a> {
    new_stats: &'a mut CharacterStats,
}

impl<'a> StatBar<'a> {
    pub fn new(new_stats: &'a mut CharacterStats) -> Self {
        Self { new_stats }
    }

    pub fn show_in_strip(self, builder: egui_extras::StripBuilder) {
        fn stat(ui: &mut egui::Ui, stats: &mut CharacterStats, stat: CharStat) {
            let frame = Frame::canvas(ui.style()).inner_margin(Margin::symmetric(0, 10));

            let stat_val = stats.get_stat_mut(stat);
            let mod_score = if stat.has_modifier() {
                &stat_val.mod_string()
            } else {
                ""
            };

            let label = stat.full_name();

            StatTile::new(label, mod_score, stat_val.deref_mut())
                .label_size(10.0)
                .frame(frame)
                .ui(ui);
        }

        builder
            .sizes(Size::remainder().at_least(100.0), 7)
            .horizontal(|mut strip| {
                for stat_type in CharStat::ALL.into_iter().filter(|x| *x != CharStat::Ac) {
                    strip.cell(|ui| {
                        stat(ui, self.new_stats, stat_type);
                    });
                }
            });
    }
}

pub struct ProfilePic<'a> {
    uri: &'a str,
}

impl<'a> ProfilePic<'a> {
    pub fn new(uri: &'a str) -> Self {
        Self { uri }
    }
}

impl Widget for ProfilePic<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        Frame::dark_canvas(ui.style())
            .outer_margin(5)
            .show(ui, |ui| Image::from_uri(self.uri).shrink_to_fit().ui(ui))
            .inner
    }
}

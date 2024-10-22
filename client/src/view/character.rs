use std::fmt::Display;

use crate::{
    prelude::*,
    state::character::commands::{RefreshCharacter, ToggleSkill},
};
use egui::{
    collapsing_header, popup_below_widget, text::LayoutJob, tooltip_id, Align, Button,
    CentralPanel, CollapsingHeader, Color32, DragValue, Frame, Label, Margin, Resize, RichText,
    TopBottomPanel, Vec2, Widget,
};
use egui_extras::{Column, TableBuilder};
use serde::de::IntoDeserializer;

use crate::{
    listener::CommandQueue,
    state::{character::commands::UseItem, DndState},
};

use super::DndTabImpl;

#[derive(Clone, Copy)]
enum CharStat {
    Cha,
    Str,
    Wis,
    Int,
    Dex,
    Con,
}

struct Skill<'a> {
    stat: CharStat,
    name: &'a str,
}

impl Display for CharStat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CharStat::Cha => write!(f, "CHA"),
            CharStat::Str => write!(f, "STR"),
            CharStat::Wis => write!(f, "WIS"),
            CharStat::Int => write!(f, "INT"),
            CharStat::Dex => write!(f, "DEX"),
            CharStat::Con => write!(f, "CON"),
        }
    }
}

trait ModScore {
    fn mod_score(&self) -> i16;
}

impl ModScore for i16 {
    fn mod_score(&self) -> i16 {
        (self / 2) - 5
    }
}

impl CharStat {
    fn get_mod_score(self, char: &common::Character) -> i64 {
        let stat = match self {
            CharStat::Cha => char.cha,
            CharStat::Str => char.str,
            CharStat::Wis => char.wis,
            CharStat::Int => char.int,
            CharStat::Dex => char.dex,
            CharStat::Con => char.con,
        };

        stat.mod_score().into()
    }
}

const SKILL_LIST: [Skill<'static>; 18] = [
    Skill {
        stat: CharStat::Dex,
        name: "Acrobatics",
    },
    Skill {
        stat: CharStat::Wis,
        name: "Animal Handling",
    },
    Skill {
        stat: CharStat::Int,
        name: "Arcana",
    },
    Skill {
        stat: CharStat::Str,
        name: "Athletics",
    },
    Skill {
        stat: CharStat::Cha,
        name: "Deception",
    },
    Skill {
        stat: CharStat::Int,
        name: "History",
    },
    Skill {
        stat: CharStat::Wis,
        name: "Insight",
    },
    Skill {
        stat: CharStat::Cha,
        name: "Intimidation",
    },
    Skill {
        stat: CharStat::Int,
        name: "Investigation",
    },
    Skill {
        stat: CharStat::Wis,
        name: "Medicine",
    },
    Skill {
        stat: CharStat::Int,
        name: "Nature",
    },
    Skill {
        stat: CharStat::Wis,
        name: "Perception",
    },
    Skill {
        stat: CharStat::Cha,
        name: "Performance",
    },
    Skill {
        stat: CharStat::Cha,
        name: "Persuasion",
    },
    Skill {
        stat: CharStat::Int,
        name: "Religion",
    },
    Skill {
        stat: CharStat::Dex,
        name: "Sleight of Hand",
    },
    Skill {
        stat: CharStat::Dex,
        name: "Stealth",
    },
    Skill {
        stat: CharStat::Wis,
        name: "Survival",
    },
];

pub struct StatWidget {
    name: String,
    value: i16,
}

impl StatWidget {
    pub fn new(name: impl ToString, value: i16) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }

    fn mod_score(&self) -> i16 {
        (self.value / 2) - 5
    }
}

impl egui::Widget for StatWidget {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        Frame::none()
            .stroke(egui::Stroke {
                width: 1.0,
                color: Color32::LIGHT_GRAY,
            })
            .inner_margin(Margin::same(5.0))
            .show(ui, |ui| {
                Resize::default()
                    .resizable(false)
                    .default_size([40.0, 40.0])
                    .show(ui, |ui| {
                        ui.vertical_centered_justified(|ui| {
                            ui.label(RichText::new(&self.name).monospace());
                            let prefix = if self.mod_score() > 0 { "+" } else { "" };
                            ui.heading(format!("{}{}", prefix, self.mod_score()));
                            ui.small(self.value.to_string());
                        });
                    });
            })
            .response
    }
}

#[derive(Default)]
pub struct Character;

impl DndTabImpl for Character {
    fn ui(&mut self, ui: &mut egui::Ui, state: &DndState, commands: &mut CommandQueue) {
        let char = &state.character.character;

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(&char.name);
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Refresh").clicked() {
                        commands.add(RefreshCharacter);
                    }
                })
            });

            ui.add_space(4.0);

            ui.label(RichText::new(format!("\"{}\"", char.tagline)).italics());
            ui.separator();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                StatWidget::new("CHA", char.cha).ui(ui);
                StatWidget::new("STR", char.str).ui(ui);
                StatWidget::new("WIS", char.wis).ui(ui);
                StatWidget::new("INT", char.int).ui(ui);
                StatWidget::new("DEX", char.dex).ui(ui);
                StatWidget::new("CON", char.con).ui(ui);
            });
            ui.add_space(6.0);
            ui.separator();

            ui.label("Skills");

            let table = TableBuilder::new(ui)
                .striped(false)
                .resizable(false)
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::exact(120.0))
                .column(Column::exact(16.0))
                .column(Column::exact(6.0))
                .cell_layout(egui::Layout::left_to_right(Align::Center));

            table.body(|body| {
                let row_height = 18.0;
                let num_rows = SKILL_LIST.len();

                body.rows(row_height, num_rows, |mut row| {
                    let index = row.index();

                    let skill = &SKILL_LIST[index];

                    let selected = char.skills.contains(&(skill.name).to_string());

                    row.col(|ui| {
                        if ui.radio(selected, "").clicked() {
                            commands.add(ToggleSkill::new(skill.name.to_string()));
                        }
                    });

                    row.col(|ui| {
                        ui.label(RichText::new(format!("{}", skill.stat)).monospace());
                    });

                    row.col(|ui| {
                        ui.label(skill.name);
                    });

                    row.col(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mut bonus = skill.stat.get_mod_score(char);

                            if selected {
                                bonus += 2;
                            }

                            let prefix = if bonus > 0 { "+" } else { "" };

                            ui.label(format!("{}{}", prefix, bonus));
                        });
                    });

                    row.col(|_| {});
                });
            });
        });
    }

    fn title(&self) -> String {
        "Character".to_owned()
    }
}

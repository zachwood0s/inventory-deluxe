use std::{fmt::Display, ops::Deref};

use crate::state::character::commands::{RefreshCharacter, ToggleSkill, UpdateCharacterStats};
use egui::{Align, Color32, Frame, Margin, Resize, RichText, Widget};
use egui_extras::{Column, TableBuilder};

use crate::{listener::CommandQueue, state::DndState};

use super::DndTabImpl;

#[derive(Clone, Copy)]
#[allow(dead_code)]
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
        Frame::NONE
            .stroke(egui::Stroke {
                width: 1.0,
                color: Color32::LIGHT_GRAY,
            })
            .inner_margin(Margin::same(5))
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
pub struct Character {
    temp_curr_hp: Option<i16>,
    temp_max_hp: Option<i16>,
}

impl DndTabImpl for Character {
    fn ui(&mut self, ui: &mut egui::Ui, state: &DndState, commands: &mut CommandQueue) {
        let Some(char) = &state.character.characters.get(&state.owned_user()) else {
            return;
        };

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(char.info.name.deref());
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Refresh").clicked() {
                        commands.add(RefreshCharacter);
                    }
                })
            });

            ui.add_space(4.0);

            ui.label(RichText::new(format!("\"{}\"", char.info.tagline)).italics());

            // health
            ui.horizontal(|ui| {
                if self.temp_curr_hp.is_none() {
                    self.temp_curr_hp = Some(char.curr_hp);
                }
                if self.temp_max_hp.is_none() {
                    self.temp_max_hp = Some(char.max_hp);
                }
                let max_hp = self.temp_max_hp.as_mut().unwrap();
                let curr_hp = self.temp_curr_hp.as_mut().unwrap();

                let curr_hp_resp = egui::DragValue::new(curr_hp).range(0..=char.max_hp).ui(ui);
                let max_hp_resp = egui::DragValue::new(max_hp).range(0..=u16::MAX).ui(ui);

                let curr_focus_lost =
                    curr_hp_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let max_focus_lost =
                    max_hp_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                if curr_focus_lost
                    || curr_hp_resp.drag_stopped()
                    || max_focus_lost
                    || max_hp_resp.drag_stopped()
                {
                    commands.add(UpdateCharacterStats::new(
                        state.owned_user(),
                        char.with_max_hp(*max_hp).with_curr_hp(*curr_hp),
                    ))
                }
            });

            // stats
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

                    let selected = char.info.skills.contains(&(skill.name).to_string());

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

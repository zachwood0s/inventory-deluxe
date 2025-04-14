use common::{data_store::CharacterStorage, CharStat};
use derive_more::Display;
use egui::{text::LayoutJob, Align, Frame, RichText, Widget};
use egui_extras::{Column, TableBuilder};

use crate::{listener::CommandQueue, state::character::commands::ToggleSkill};

struct Skill<'a> {
    stat: CharStat,
    name: &'a str,
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

pub struct SkillsTable<'a, 'q> {
    character: &'a CharacterStorage,
    commands: &'a mut CommandQueue<'q>,
}

impl<'a, 'q> SkillsTable<'a, 'q> {
    pub fn new(character: &'a CharacterStorage, commands: &'a mut CommandQueue<'q>) -> Self {
        Self {
            character,
            commands,
        }
    }

    pub fn show(self, ui: &mut egui::Ui) {
        ui.vertical_centered_justified(|ui| {
            let frame = Frame::group(ui.style()).inner_margin(5).outer_margin(5);

            frame.show(ui, |ui| {
                let table = TableBuilder::new(ui)
                    .id_salt("PROFICIENCY")
                    .resizable(false)
                    .column(Column::remainder())
                    .column(Column::exact(16.0))
                    .column(Column::exact(6.0))
                    .cell_layout(egui::Layout::left_to_right(Align::Center));

                table.body(|body| {
                    let row_height = 18.0;
                    body.rows(row_height, 1, |mut row| {
                        row.col(|ui| {
                            ui.label("PROFICIENCY");
                        });

                        row.col(|ui| {
                            ui.label("+2");
                        });

                        row.col(|ui| {});
                    });
                });
            });

            frame.show(ui, |ui| {
                let table = TableBuilder::new(ui)
                    .id_salt("SKILLS")
                    .striped(true)
                    .resizable(false)
                    .column(Column::auto())
                    .column(Column::auto())
                    .column(Column::remainder())
                    .column(Column::exact(16.0))
                    .column(Column::exact(6.0))
                    .cell_layout(egui::Layout::left_to_right(Align::Center));

                table.body(|body| {
                    let row_height = 18.0;
                    let num_rows = SKILL_LIST.len();

                    let info = self.character.info();
                    let stats = self.character.stats();

                    body.rows(row_height, num_rows, |mut row| {
                        let index = row.index();

                        let skill = &SKILL_LIST[index];

                        let selected = info.skills.contains(&(skill.name).to_string());

                        row.col(|ui| {
                            if ui.radio(selected, "").clicked() {
                                log::info!("Selected");
                                self.commands.add(ToggleSkill::new(skill.name.to_string()));
                            }
                        });

                        row.col(|ui| {
                            ui.label(RichText::new(format!("{}", skill.stat)).monospace());
                        });

                        row.col(|ui| {
                            ui.label(skill.name);
                        });

                        row.col(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let mut bonus = stats.get_stat(skill.stat).mod_score();

                                    if selected {
                                        bonus += 2;
                                    }

                                    let prefix = if bonus > 0 { "+" } else { "" };

                                    ui.label(format!("{}{}", prefix, bonus));
                                },
                            );
                        });

                        row.col(|_| {});
                    });
                });
            });
        });
    }
}

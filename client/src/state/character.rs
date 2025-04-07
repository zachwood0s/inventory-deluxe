use std::collections::HashMap;

use common::{message::DndMessage, Ability, Item};

#[derive(Default)]
pub struct CharacterState {
    pub characters: HashMap<common::User, common::Character>,
    pub items: Vec<Item>,
    pub abilities: Vec<Ability>,
}

impl CharacterState {
    pub fn process(&mut self, message: &DndMessage) {
        #[allow(clippy::single_match)]
        match message {
            DndMessage::ItemList(items) => {
                self.items = items.clone();
            }
            DndMessage::CharacterData(character) => {
                self.characters
                    .insert(character.info.name.clone(), character.clone());
            }
            DndMessage::AbilityList(abilities) => {
                self.abilities = abilities.clone();
            }
            DndMessage::UpdateCharacterStats(msg) => {
                if let Some(character) = self.characters.get_mut(&msg.user) {
                    character.stats = msg.new_stats;
                }
            }
            _ => {}
        }
    }
}

pub mod commands {
    use common::CharacterStats;

    use crate::prelude::*;

    pub struct UseItem {
        pub item_idx: usize,
        pub count: u32,
    }

    impl UseItem {
        pub fn new(item_idx: usize, count: u32) -> Self {
            Self { item_idx, count }
        }
    }

    impl Command for UseItem {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            let user = state.owned_user();

            let Some(item) = state.character.items.get_mut(self.item_idx) else {
                error!(
                    "Trying to use item which no longer exists. Idx: {}",
                    self.item_idx
                );
                return;
            };

            item.count = item.count.saturating_sub(self.count);

            // Update item count in DB
            tx.send(
                DndMessage::UpdateItemCount(UpdateItemCount {
                    user: user.clone(),
                    item_id: item.id,
                    new_count: item.count,
                })
                .into(),
            );

            // Send Log Message
            tx.send(
                DndMessage::Log(Log {
                    user,
                    payload: LogMessage::UseItem(item.name.clone(), self.count),
                })
                .into(),
            );

            // Remove immediately from display if no more count.
            // (DB will also do this)
            if item.count == 0 {
                state.character.items.remove(self.item_idx);
            }
        }
    }

    pub struct RefreshCharacter;

    impl Command for RefreshCharacter {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            tx.send(
                DndMessage::RetrieveCharacterData(RetrieveCharacterData {
                    user: state.owned_user(),
                })
                .into(),
            )
        }
    }

    pub struct ToggleSkill {
        pub skill_name: String,
    }

    impl ToggleSkill {
        pub fn new(skill_name: String) -> Self {
            ToggleSkill { skill_name }
        }
    }

    impl Command for ToggleSkill {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            let user = state.owned_user();

            let Some(character) = state.character.characters.get_mut(&user) else {
                return;
            };

            let skills = &mut character.info.skills;

            if skills.contains(&self.skill_name) {
                skills.retain(|x| x != &self.skill_name);
            } else {
                skills.push(self.skill_name);
            }

            tx.send(
                DndMessage::UpdateSkills(UpdateSkills {
                    user,
                    skills: skills.clone(),
                })
                .into(),
            );
        }
    }

    pub struct UpdateCharacterStats {
        user: User,
        new_stats: CharacterStats,
    }

    impl UpdateCharacterStats {
        pub fn new(user: User, new_stats: CharacterStats) -> Self {
            Self { user, new_stats }
        }
    }

    impl Command for UpdateCharacterStats {
        fn execute(self: Box<Self>, _: &mut DndState, tx: &EventSender<Signal>) {
            let Self { user, new_stats } = *self;

            tx.send(
                DndMessage::UpdateCharacterStats(common::message::UpdateCharacterStats {
                    user,
                    new_stats,
                })
                .into(),
            )
        }
    }
}

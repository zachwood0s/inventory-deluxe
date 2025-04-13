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
            _ => {}
        }
    }
}

pub mod commands {
    use common::{
        data_store::{self, DataMessage, UpdateSkills},
        CharacterStats,
    };

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

            let Ok(character) = state.data.get_character_mut(&user) else {
                error!("Can't find charactero: {}", user);
                return;
            };

            //let Some(item) = character.get_item_mut(self.)

            //item.count = item.count.saturating_sub(self.count);

            //let data_message = data_store::UpdateItemCount {
            //    user,
            //    item_id: item.id,
            //    new_count: item.count,
            //};

            //// Update item count in DB
            ////tx.send(
            ////    DndMessage::UpdateItemCount(UpdateItemCount {
            ////        user: user.clone(),
            ////        item_id: item.id,
            ////        new_count: item.count,
            ////    })
            ////    .into(),
            ////);

            //// Send Log Message
            //tx.send(
            //    DndMessage::Log(Log {
            //        user,
            //        payload: LogMessage::UseItem(item.name.clone(), self.count),
            //    })
            //    .into(),
            //);

            // Remove immediately from display if no more count.
            // (DB will also do this)
            //if item.count == 0 {
            //    state.character.items.remove(self.item_idx);
            //}
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

            let skills = character.info.skills.clone();
            let message = UpdateSkills { user, skills };

            tx.send(DndMessage::DataMessage(message.into()).into());
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

            let data: data_store::DataMessage =
                data_store::UpdateCharacterStats { user, new_stats }.into();

            tx.send(DndMessage::DataMessage(data).into())
        }
    }
}

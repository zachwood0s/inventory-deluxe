use common::{message::DndMessage, Ability, Item};

#[derive(Default)]
pub struct CharacterState {
    pub character: common::Character,
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
                self.character = character.clone();
            }
            DndMessage::AbilityList(abilities) => {
                self.abilities = abilities.clone();
            }
            _ => {}
        }
    }
}

pub mod commands {
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
            tx.send(DndMessage::UpdateItemCount(user.clone(), item.id, item.count).into());

            // Send Log Message
            tx.send(
                DndMessage::Log(user, LogMessage::UseItem(item.name.clone(), self.count)).into(),
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
            tx.send(DndMessage::RetrieveCharacterData(state.owned_user()).into())
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

            let skills = &mut state.character.character.skills;


            if skills.contains(&self.skill_name) {
                skills.retain(|x| x != &self.skill_name);
            }
            else {
                skills.push(self.skill_name);
            }

            tx.send(DndMessage::UpdateSkills(user.clone(), skills.clone()).into());
        }
    }
}

pub mod commands {
    use crate::prelude::*;

    pub struct SetAbilityCount {
        pub ability_idx: usize,
        pub count: i64,
        pub broadcast: bool,
    }

    impl SetAbilityCount {
        pub fn new(ability_idx: usize, count: i64, broadcast: bool) -> Self {
            Self {
                ability_idx,
                count,
                broadcast,
            }
        }
    }

    impl Command for SetAbilityCount {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            let user = state.owned_user();

            let Some(ability) = state.character.abilities.get_mut(self.ability_idx) else {
                error!(
                    "Trying to use ability that doesn't exist on the character. Idx: {}",
                    self.ability_idx
                );
                return;
            };

            ability.uses = self.count;

            if self.broadcast {
                // Update item count in DB
                tx.send(
                    DndMessage::UpdateAbilityCount(
                        user.clone(),
                        ability.name.clone(),
                        ability.uses,
                    )
                    .into(),
                );

                // Send Log Message
                tx.send(
                    DndMessage::Log(
                        user,
                        LogMessage::SetAbilityCount(ability.name.clone(), self.count),
                    )
                    .into(),
                );
            }
        }
    }

    pub struct SetPowerSlotCount {
        pub count: i16,
    }

    impl SetPowerSlotCount {
        pub fn new(count: i16) -> Self {
            Self { count }
        }
    }

    impl Command for SetPowerSlotCount {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            let user = state.owned_user();

            let power_slots = &mut state.character.character.power_slots;

            *power_slots = self.count;

                // Update item count in DB
                tx.send(
                    DndMessage::UpdatePowerSlotCount(
                        user.clone(),
                        *power_slots,
                    )
                    .into(),
                );

                /*
                // Send Log Message
                tx.send(
                    DndMessage::Log(
                        user,
                        LogMessage::SetAbilityCount(ability.name.clone(), self.count),
                    )
                    .into(),
                );
                */
        }
    }
    pub struct RefreshCharacter;

    impl Command for RefreshCharacter {
        fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
            tx.send(DndMessage::RetrieveCharacterData(state.owned_user()).into())
        }
    }
}

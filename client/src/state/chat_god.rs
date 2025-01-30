/*
let parts = self.text.split(" ").collect_vec();

match Parser::parse(&parts) {
    Some(command) => tx.send(command.to_dnd(state).into()),
    None => {
        tx.send(DndMessage::Log(state.owned_user(), LogMessage::Chat(self.text)).into())
    }
}
*/

mod comand_parser {
    use common::message::{DndMessage, LogMessage};
    use egui::TextBuffer;
    use itertools::Itertools;
    use rand::Rng;

    use super::DndState;

    pub trait ToDndMessge {
        fn to_dnd(&self, state: &DndState) -> DndMessage;
    }

    impl ToDndMessge for DndMessage {
        fn to_dnd(&self, _state: &DndState) -> DndMessage {
            self.clone()
        }
    }

    impl<T: ToDndMessge + ?Sized> ToDndMessge for Box<T> {
        fn to_dnd(&self, state: &DndState) -> DndMessage {
            (**self).to_dnd(state)
        }
    }

    pub trait ChatCommand {
        type Output: ToDndMessge + 'static;

        fn parse(parts: &[&str]) -> Option<Self::Output> {
            let first = parts.first()?;

            Self::prefix()
                .contains(&first.as_str())
                .then(|| Self::parse_parts(parts))
                .flatten()
        }

        fn prefix() -> Vec<&'static str>;
        fn parse_parts(parts: &[&str]) -> Option<Self::Output>;
    }

    pub struct Roll {
        die: u32,
        val: u32,
    }

    impl ToDndMessge for Roll {
        fn to_dnd(&self, state: &DndState) -> DndMessage {
            DndMessage::Log(state.owned_user(), LogMessage::Roll(self.die, self.val))
        }
    }

    impl ChatCommand for Roll {
        type Output = Self;

        fn prefix() -> Vec<&'static str> {
            vec!["/roll", "/r"]
        }

        fn parse_parts(parts: &[&str]) -> Option<Self::Output> {
            if parts.len() != 2 {
                return None;
            }

            let die = parts[1].parse().ok()?;
            let mut rng = rand::rng();

            Some(Roll {
                die,
                val: rng.random_range(0..die),
            })
        }
    }

    impl ToDndMessge for String {
        fn to_dnd(&self, state: &DndState) -> DndMessage {
            DndMessage::Log(state.owned_user(), LogMessage::Chat(self.clone()))
        }
    }

    pub struct Whisper;
    impl ChatCommand for Whisper {
        type Output = String;

        fn prefix() -> Vec<&'static str> {
            vec!["/whisper"]
        }

        fn parse_parts(parts: &[&str]) -> Option<Self::Output> {
            Some(format!(
                "I whisper to you: {}",
                parts.iter().skip(1).join(" ")
            ))
        }
    }

    macro_rules! impl_command_tuple {
        (
            $($type: ident),*
        ) => {
            impl<$($type: ChatCommand, )*> ChatCommand for ($($type, )*) {
                type Output = Box<dyn ToDndMessge>;

                fn prefix() -> Vec<&'static str> {
                    [
                        $($type::prefix(),)*
                    ].into_iter().flatten().collect()
                }

                fn parse_parts(parts: &[&str]) -> Option<Self::Output> {
                    $(
                        if let Some(res) = $type::parse(parts) {
                            return Some(Box::new(res));
                        }
                    )*

                    None
                }
            }
        }
    }

    impl_command_tuple!(T1, T2);
    impl_command_tuple!(T1, T2, T3);
    impl_command_tuple!(T1, T2, T3, T4);
    impl_command_tuple!(T1, T2, T3, T4, T5);
    impl_command_tuple!(T1, T2, T3, T4, T5, T6);
    impl_command_tuple!(T1, T2, T3, T4, T5, T6, T7);
}

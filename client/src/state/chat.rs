use crate::{prelude::*, widgets::frames::Emphasized};
use comand_parser::{ChatCommandParser, ToDndMessge};
use egui::{text::LayoutJob, Align, Color32, FontSelection, Margin, RichText, Rounding, Style};
use itertools::Itertools;

pub struct ClientLogMessage {
    pub user: User,
    pub message: LogMessage,
}

impl ClientLogMessage {
    pub fn new(user: User, message: LogMessage) -> Self {
        Self { user, message }
    }

    pub fn ui(&self, ui: &mut egui::Ui, display_name: bool) {
        let hide_name = matches!(self.message, LogMessage::Joined(_))
            || matches!(self.message, LogMessage::Disconnected(_));

        if display_name {
            ui.separator();
            if !hide_name {
                ui.label(
                    RichText::new(format!("{}: ", self.user.name))
                        .strong()
                        .color(Color32::LIGHT_BLUE),
                );
            }
        }

        match &self.message {
            LogMessage::Chat(c) => {
                ui.label(c);
            }
            LogMessage::UseItem(item, count) => {
                let style = Style::default();
                let mut layout_job = LayoutJob::default();
                RichText::new(format!("Used {} ", count))
                    .italics()
                    .append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

                RichText::new(item).color(Color32::LIGHT_GREEN).append_to(
                    &mut layout_job,
                    &style,
                    FontSelection::Default,
                    Align::LEFT,
                );

                ui.label(layout_job);
            }
            LogMessage::Joined(joined_user) => {
                ui.colored_label(Color32::DARK_GRAY, format!("{} joined", joined_user));
            }
            LogMessage::Disconnected(discon_user) => {
                ui.colored_label(Color32::DARK_GRAY, format!("{} disconnected", discon_user));
            }
            LogMessage::Server(msg) => {
                ui.colored_label(Color32::DARK_GRAY, msg);
            }
            LogMessage::SetAbilityCount(ability, count) => {
                let style = Style::default();
                let mut layout_job = LayoutJob::default();
                RichText::new("Used  ").italics().append_to(
                    &mut layout_job,
                    &style,
                    FontSelection::Default,
                    Align::LEFT,
                );

                RichText::new(ability).color(Color32::LIGHT_RED).append_to(
                    &mut layout_job,
                    &style,
                    FontSelection::Default,
                    Align::LEFT,
                );

                RichText::new(format!(", they have {} uses left", count))
                    .italics()
                    .append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

                ui.label(layout_job);
            }
            LogMessage::Roll(die_roll) => {
                let style = Style::default();
                let mut layout_job = LayoutJob::default();

                let mut rolls = die_roll.rolls.iter().peekable();
                while let Some(roll) = rolls.next() {
                    let color = if roll.taken {
                        Color32::GREEN
                    } else {
                        Color32::GRAY
                    };

                    RichText::new(format!("{}", roll.value))
                        .color(color)
                        .append_to(&mut layout_job, &style, FontSelection::Default, Align::LEFT);

                    if rolls.peek().is_some() {
                        RichText::new(" + ").color(Color32::DARK_GRAY).append_to(
                            &mut layout_job,
                            &style,
                            FontSelection::Default,
                            Align::LEFT,
                        );
                    }
                }

                let mut result_layout = LayoutJob::default();

                RichText::new(format!("{} = ", die_roll.roll_str))
                    .color(Color32::DARK_GRAY)
                    .append_to(
                        &mut result_layout,
                        &style,
                        FontSelection::Default,
                        Align::RIGHT,
                    );

                RichText::new(format!("{}", die_roll.total))
                    .color(Color32::GREEN)
                    .italics()
                    .append_to(
                        &mut result_layout,
                        &style,
                        FontSelection::Default,
                        Align::RIGHT,
                    );

                Emphasized.show(ui, |ui| {
                    ui.label(layout_job);
                    ui.separator();
                    ui.label(result_layout);
                });
            }
        };
    }
}

#[derive(Default)]
pub struct ChatState {
    pub log_messages: Vec<ClientLogMessage>,
}

impl ChatState {
    pub fn process(&mut self, message: &DndMessage) {
        #[allow(clippy::single_match)]
        match message {
            DndMessage::Log(Log { user, payload }) => self
                .log_messages
                .push(ClientLogMessage::new(user.clone(), payload.clone())),
            DndMessage::ItemList(list) => {
                println!("Recieved item list {list:?}");
            }
            _ => {}
        }
    }
}

pub struct ChatCommand {
    text: String,
}

impl ChatCommand {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

impl Command for ChatCommand {
    fn execute(self: Box<Self>, state: &mut DndState, tx: &EventSender<Signal>) {
        let parts = self.text.split(" ").collect_vec();

        type Parser = (commands::Roll, commands::SaveBoard, commands::LoadBoard);
        match Parser::parse(&parts) {
            Some(command) => tx.send(command.to_dnd(state).into()),
            None => {
                tx.send(
                    DndMessage::Log(Log {
                        user: state.owned_user(),
                        payload: LogMessage::Chat(self.text),
                    })
                    .into(),
                );
            }
        }
    }
}

mod comand_parser {
    use common::message::DndMessage;
    use egui::TextBuffer;

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

    pub trait ChatCommandParser {
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

    macro_rules! impl_command_tuple {
        (
            $($type: ident),*
        ) => {
            impl<$($type: ChatCommandParser, )*> ChatCommandParser for ($($type, )*) {
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

pub mod commands {
    use common::message::{DieRoll, DndMessage, Log, LogMessage, SingleDieRoll};
    use itertools::Itertools;
    use log::info;
    use once_cell::sync::Lazy;
    use rand::Rng;
    use regex::Regex;

    use super::{
        comand_parser::{ChatCommandParser, ToDndMessge},
        DndState,
    };

    pub struct Roll {
        roll_str: String,
        die_count: u32,
        die: u32,
        modifier: Option<Modifier>,
    }

    #[derive(PartialEq, Eq, Clone, Copy)]
    pub enum ValueType {
        Highest,
        Lowest,
    }

    pub enum ModifierType {
        Keep,
        Drop,
    }

    pub struct Modifier {
        modi: ModifierType,
        val: ValueType,
        count: u32,
    }

    impl Modifier {
        fn keep(val: ValueType, count: u32) -> Self {
            Self {
                modi: ModifierType::Keep,
                val,
                count,
            }
        }

        fn drop(val: ValueType, count: u32) -> Self {
            Self {
                modi: ModifierType::Drop,
                val,
                count,
            }
        }

        pub fn parse(parse_str: &str, count: u32) -> Option<Self> {
            match parse_str.to_lowercase().as_str() {
                "kh" => Some(Self::keep(ValueType::Highest, count)),
                "kl" => Some(Self::keep(ValueType::Lowest, count)),
                "dh" => Some(Self::drop(ValueType::Highest, count)),
                "dl" => Some(Self::drop(ValueType::Lowest, count)),
                _ => None,
            }
        }

        pub fn apply(&self, rolls: &mut [SingleDieRoll]) {
            let mut ordered = rolls
                .iter_mut()
                .sorted_by_key(|roll| roll.value)
                .collect_vec();

            if self.val == ValueType::Highest {
                ordered.reverse();
            }

            // - For "keeping" values, we mark all as not taken, then mark
            // the selected values as taken
            // - For "dropping" values, we keep all as taken, then mark
            // the selected values as NOT taken
            let mark_top_value = match self.modi {
                ModifierType::Keep => {
                    ordered.iter_mut().for_each(|x| x.taken = false);
                    true
                }
                ModifierType::Drop => false,
            };

            // Go through each in the ordered list and mark the them as taken or dropped.
            ordered
                .iter_mut()
                .take(self.count as usize)
                .for_each(|x| x.taken = mark_top_value);
        }
    }

    impl ToDndMessge for Roll {
        fn to_dnd(&self, state: &DndState) -> DndMessage {
            let mut rng = rand::rng();

            let mut rolls = Vec::new();
            for _ in 0..self.die_count {
                let value = rng.random_range(1..=self.die);

                // All rolls start off as taken, we'll elminate later
                rolls.push(SingleDieRoll { value, taken: true })
            }

            if let Some(modifier) = &self.modifier {
                modifier.apply(&mut rolls);
            }

            let total = rolls.iter().flat_map(|x| x.taken.then_some(x.value)).sum();

            DndMessage::Log(Log {
                user: state.owned_user(),
                payload: LogMessage::Roll(DieRoll {
                    roll_str: self.roll_str.clone(),
                    total,
                    rolls,
                }),
            })
        }
    }

    impl ChatCommandParser for Roll {
        type Output = Self;

        fn prefix() -> Vec<&'static str> {
            vec!["/roll", "/r"]
        }

        fn parse_parts(parts: &[&str]) -> Option<Self::Output> {
            static RE: Lazy<Regex> = Lazy::new(|| {
                // Parses the form of [num]d[num](mod[num])?
                // i.e. 2d20kh1
                Regex::new(
                    r"(?P<amount>[0-9]+)?d(?P<die>[0-9]+)((?P<adv>[a-zA-Z]+)(?P<adv_value>[0-9]+))?",
                )
                .unwrap()
            });

            if parts.len() != 2 {
                return None;
            }

            let captures = RE.captures(parts[1])?;

            const MAX_DIE_ROLL: u32 = 1_000;
            const MAX_DIE_COUNT: u32 = 1_000;

            let get_capture_int = |name, default, maximum| -> Option<u32> {
                if let Some(val) = captures.name(name) {
                    let parsed = val.as_str().parse().ok()?;

                    (parsed < maximum).then_some(parsed)
                } else {
                    Some(default)
                }
            };

            let die_count = get_capture_int("amount", 1, MAX_DIE_COUNT)?;
            let die = get_capture_int("die", 1, MAX_DIE_ROLL)?;
            let adv_value = get_capture_int("adv_value", 0, MAX_DIE_ROLL);
            let modifier = captures
                .name("adv")
                .and_then(|x| Modifier::parse(x.as_str(), adv_value.unwrap()));

            Some(Roll {
                roll_str: parts[1].into(),
                die,
                die_count,
                modifier,
            })
        }
    }

    pub struct SaveBoard {
        tag: Option<String>,
    }

    impl ToDndMessge for SaveBoard {
        fn to_dnd(&self, _: &DndState) -> DndMessage {
            DndMessage::SaveBoard(common::message::SaveBoard {
                tag: self.tag.clone(),
            })
        }
    }

    impl ChatCommandParser for SaveBoard {
        type Output = Self;

        fn prefix() -> Vec<&'static str> {
            vec!["/save"]
        }

        fn parse_parts(parts: &[&str]) -> Option<Self::Output> {
            if parts.len() > 2 {
                return None;
            }

            let tag = parts.get(1).map(|x| String::from(*x));

            Some(Self { tag })
        }
    }

    pub struct LoadBoard {
        tag: String,
    }

    impl ToDndMessge for LoadBoard {
        fn to_dnd(&self, _: &DndState) -> DndMessage {
            DndMessage::LoadBoard(common::message::LoadBoard {
                tag: self.tag.clone(),
            })
        }
    }

    impl ChatCommandParser for LoadBoard {
        type Output = Self;

        fn prefix() -> Vec<&'static str> {
            vec!["/load"]
        }

        fn parse_parts(parts: &[&str]) -> Option<Self::Output> {
            if parts.len() != 2 {
                return None;
            }

            let tag = parts.get(1).map(|x| String::from(*x))?;

            Some(Self { tag })
        }
    }

    impl ToDndMessge for String {
        fn to_dnd(&self, state: &DndState) -> DndMessage {
            DndMessage::Log(Log {
                user: state.owned_user(),
                payload: LogMessage::Chat(self.clone()),
            })
        }
    }
}

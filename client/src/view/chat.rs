use std::sync::mpsc::Receiver;

use common::{message::DndMessage, User};
use egui::{Color32, ScrollArea, TextEdit, Widget};
use message_io::events::EventSender;

use crate::{listener::Signal, state::DndState};

use super::DndTabImpl;

#[derive(Default)]
pub struct Chat {
    text: String,
}

impl DndTabImpl for Chat {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &DndState,
        tx: &EventSender<Signal>,
        rx: &Receiver<DndMessage>,
    ) {
        egui::TopBottomPanel::bottom("chat_box")
            .resizable(false)
            .min_height(30.0)
            .show_inside(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let submitted = TextEdit::singleline(&mut self.text)
                        .desired_width(f32::INFINITY)
                        .ui(ui);

                    if submitted.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        submitted.request_focus();

                        tx.send(
                            DndMessage::Chat(state.user.clone().unwrap(), self.text.clone()).into(),
                        );

                        self.text.clear();
                    }
                })
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ScrollArea::new([false, true])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for msg in state.chat.log_messages.iter() {
                        msg.ui(ui);
                    }
                });
        });
    }

    fn title(&self) -> String {
        "Chat".to_owned()
    }
}

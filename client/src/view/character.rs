use common::Item;
use egui::{
    collapsing_header, popup_below_widget, text::LayoutJob, tooltip_id, CentralPanel,
    CollapsingHeader, Color32, DragValue, Frame, RichText, Widget,
};

use super::DndTabImpl;

#[derive(Default)]
pub struct Character {
    use_num: u32,
}

impl Character {
    fn draw_item(&mut self, ui: &mut egui::Ui, item: &Item) {
        let mut item_text =
            LayoutJob::single_section(item.name.clone(), egui::TextFormat::default());
        item_text.append(
            &format!("x{}", item.count),
            10.0,
            egui::TextFormat {
                color: Color32::LIGHT_GREEN,
                italics: true,
                ..Default::default()
            },
        );
        let id = egui::Id::new(&item.name);
        let popup_id = egui::Id::new(format!("{}.popup", item.name));

        collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&item.name));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let button = ui.button("Use");
                        if button.clicked() {
                            ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                        }
                        popup_below_widget(
                            ui,
                            popup_id,
                            &button,
                            egui::PopupCloseBehavior::CloseOnClickOutside,
                            |ui| {
                                ui.set_min_width(70.0);
                                ui.horizontal(|ui| {
                                    DragValue::new(&mut self.use_num).ui(ui);
                                    if ui.button("Done").clicked() {
                                        println!("Used item");
                                    }
                                })
                            },
                        );

                        ui.label(
                            RichText::new(format!("x{}", item.count))
                                .color(Color32::LIGHT_GREEN)
                                .italics(),
                        );
                    })
                })
            })
            .body(|ui| {
                ui.label(&item.description);

                let flavor = RichText::new(format!("\"{}\"", &item.flavor_text)).italics();

                ui.label(flavor);
            });
    }
}

impl DndTabImpl for Character {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &crate::state::DndState,
        tx: &message_io::events::EventSender<crate::listener::Signal>,
        rx: &std::sync::mpsc::Receiver<common::message::DndMessage>,
    ) {
        ui.label("Character Sheet");

        CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Items");
            for item in state.character.items.iter() {
                self.draw_item(ui, item);
                ui.separator();
            }
        });
    }

    fn title(&self) -> String {
        "Character".to_owned()
    }
}

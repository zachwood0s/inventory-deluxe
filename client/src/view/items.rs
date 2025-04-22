use egui::{collapsing_header, popup_below_widget, DragValue};

use crate::{listener::CommandQueue, prelude::*, state::character::commands::UseItem};

use super::DndTabImpl;

pub struct ItemWidget<'a, 'b, 'c> {
    idx: usize,
    item: Item,
    use_num: &'a mut u32,
    commands: &'b mut CommandQueue<'c>,
}

impl<'a, 'b, 'c> ItemWidget<'a, 'b, 'c> {
    fn new(
        idx: usize,
        item: Item,
        use_num: &'a mut u32,
        commands: &'b mut CommandQueue<'c>,
    ) -> Self {
        Self {
            idx,
            item,
            use_num,
            commands,
        }
    }
}

impl Widget for ItemWidget<'_, '_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut item_text =
            LayoutJob::single_section(self.item.name.clone(), egui::TextFormat::default());
        item_text.append(
            &format!("x{}", self.item.count),
            10.0,
            egui::TextFormat {
                color: Color32::LIGHT_GREEN,
                italics: true,
                ..Default::default()
            },
        );

        let id = egui::Id::new(&self.item.name);
        let popup_id = egui::Id::new(format!("{}.popup", self.item.name));

        collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut title = RichText::new(&self.item.name);

                    if self.item.quest_item {
                        title = title.color(Color32::YELLOW);
                    }

                    ui.label(title);

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
                                    DragValue::new(self.use_num)
                                        .range(1..=self.item.count)
                                        .ui(ui);

                                    if ui.button("Done").clicked() {
                                        self.commands.add(UseItem::new(self.idx, *self.use_num));

                                        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                                    }
                                })
                            },
                        );

                        ui.label(
                            RichText::new(format!("x{}", self.item.count))
                                .color(Color32::LIGHT_GREEN)
                                .italics(),
                        );
                    })
                })
            })
            .body(|ui| {
                //egui_demo_lib::easy_mark::easy_mark(ui, &self.item.description);

                if let Some(flavor_text) = &self.item.flavor_text {
                    egui_demo_lib::easy_mark::easy_mark(ui, &format!("/\"{}\"/", flavor_text));
                }
            })
            .0
    }
}

#[derive(Default)]
pub struct Items {
    use_num: u32,
}

impl DndTabImpl for Items {
    fn ui(&mut self, ui: &mut Ui, state: &DndState, commands: &mut CommandQueue) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Items");
            for (idx, item) in state.character.items.iter().enumerate() {
                ItemWidget::new(idx, item.clone(), &mut self.use_num, commands).ui(ui);
                ui.separator();
            }
        });
    }

    fn title(&self) -> String {
        "Items".to_string()
    }
}

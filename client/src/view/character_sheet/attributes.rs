use egui::{vec2, Frame, Layout, Margin, RichText, UiBuilder, Vec2, Widget};
use egui_extras::{Size, StripBuilder};
use emath::Numeric;

use crate::widgets::group::Group;

pub struct StatTile<'a, T: Numeric> {
    label: &'a str,
    bottom_label: &'a str,
    value: &'a mut T,
}

impl<'a, T: Numeric> StatTile<'a, T> {
    pub fn new(label: &'a str, bottom_label: &'a str, value: &'a mut T) -> Self {
        Self {
            label,
            value,
            bottom_label,
        }
    }
}

impl<T: Numeric> Widget for StatTile<'_, T> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let frame = Frame::canvas(ui.style());

        let (_, max_rect) = ui.allocate_space(ui.available_size());
        let builder = UiBuilder::new()
            .layout(Layout::left_to_right(egui::Align::Center).with_main_justify(true))
            .max_rect(max_rect);

        let resp = ui.scope_builder(builder, |ui| {
            frame.show(ui, |ui| {
                Group::new("grouped").show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(self.label).monospace());
                        let val_text = RichText::new(self.value.to_f64().to_string())
                            .size(35.0)
                            .monospace();
                        ui.label(val_text);
                        ui.label(RichText::new(self.bottom_label).monospace());
                    });
                });
            });
        });

        resp.response

        //child_ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
        //    frame.show(ui, |ui| {
        //        let prepared = Group::new().begin(ui);

        //        Group::new("grouped_stat").show(ui, |ui| {
        //            ui.vertical_centered_justified(|ui| {
        //                ui.label(self.label);
        //                let val_text = RichText::new(self.value.to_f64().to_string()).size(30.0);
        //                ui.label(val_text);
        //                ui.label(self.bottom_label);
        //            });
        //        });
        //    });
        //});

        //child_ui.response()

        //ui.scope_builder(builder, |ui| {
        //    Group::new("grouped_stat").show(ui, |ui| {
        //        ui.vertical_centered_justified(|ui| {
        //            ui.label(self.label);
        //            let val_text = RichText::new(self.value.to_f64().to_string()).size(30.0);
        //            ui.label(val_text);
        //            ui.label(self.bottom_label);
        //        });
        //    });
        //})
        //.response

        //Frame::canvas(ui.style())
        //    .show(ui, |ui| {
        //        ui.vertical_centered_justified(|ui| {});
        //        ui.allocate_space(ui.available_size());
        //    })
        //    .response
    }
}

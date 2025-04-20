use egui::{
    DragValue, Frame, Layout, RichText, Sense, TextStyle, UiBuilder, UiKind, UiStackInfo, Widget,
};
use emath::Numeric;

use super::CustomUi;

pub struct StatTile<'a, T: Numeric> {
    label: &'a str,
    bottom_label: &'a str,
    value: &'a mut T,
    label_size: f32,
    frame: Option<Frame>,
}

impl<'a, T: Numeric> StatTile<'a, T> {
    pub fn new(label: &'a str, bottom_label: &'a str, value: &'a mut T) -> Self {
        Self {
            label,
            value,
            bottom_label,
            label_size: 12.0,
            frame: None,
        }
    }

    pub fn label_size(mut self, label_size: f32) -> Self {
        self.label_size = label_size;
        self
    }

    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }
}

impl<T: Numeric> Widget for StatTile<'_, T> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let frame = self.frame.unwrap_or_else(|| Frame::canvas(ui.style()));

        let id = ui.next_auto_id();

        let editing = ui
            .data_mut(|data| data.get_temp::<bool>(id))
            .unwrap_or(false);

        let (_, max_rect) = ui.allocate_space(ui.available_size());
        let builder = UiBuilder::new()
            .layout(Layout::left_to_right(egui::Align::Center).with_main_justify(true))
            .ui_stack_info(UiStackInfo::new(UiKind::Frame).with_frame(frame))
            .sense(Sense::click())
            .max_rect(max_rect);

        let mut resp = ui.scope_builder(builder, |ui| {
            ui.style_mut().drag_value_text_style = TextStyle::Name("stat_tile_edit".into());

            frame.show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.no_select_label(RichText::new(self.label).monospace().size(self.label_size));

                    let resp = if editing {
                        let drag = DragValue::new(self.value).range(0..=30);
                        ui.add(drag)
                    } else {
                        let val_text = RichText::new(self.value.to_f64().to_string())
                            .font(TextStyle::Name("stat_tile".into()).resolve(ui.style()))
                            .monospace();

                        ui.no_select_label(val_text)
                    };

                    ui.no_select_label(
                        RichText::new(self.bottom_label)
                            .monospace()
                            .size(self.label_size),
                    );

                    resp
                })
            })
        });

        resp.response = resp
            .response
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        if resp.response.clicked() {
            ui.data_mut(|data| data.insert_temp(id, true));
        } else if resp.response.clicked_elsewhere() {
            ui.data_mut(|data| data.remove_temp::<bool>(id));
        }

        resp.inner.inner.inner
    }
}

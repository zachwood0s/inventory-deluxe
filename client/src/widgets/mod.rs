use std::any::type_name;

use egui::{
    vec2, Color32, ComboBox, Frame, Label, Layout, Response, Rgba, RichText, Stroke, Widget,
    WidgetText,
};
use emath::Numeric;

mod frames;
mod group;
mod stat_tile;
mod toggle_icon;

pub use frames::*;
pub use group::*;
pub use stat_tile::*;
pub use toggle_icon::*;

pub trait No {
    fn no(self);
}

impl No for Response {
    fn no(self) {}
}

pub trait WithAlpha {
    fn with_alpha(self, alpha: f32) -> Self;
}

impl WithAlpha for Rgba {
    fn with_alpha(self, alpha: f32) -> Self {
        Self::from_rgba_premultiplied(self.r(), self.g(), self.b(), alpha)
    }
}

pub trait CustomUi {
    fn stat_tile<Num: Numeric>(
        &mut self,
        top_label: &str,
        bottom_label: &str,
        value: &mut Num,
    ) -> Response;

    fn no_select_label(&mut self, text: impl Into<WidgetText>) -> Response;

    fn selectable_value_enum<Value: PartialEq + std::fmt::Display + strum::IntoEnumIterator>(
        &mut self,
        current_value: &mut Value,
    );

    fn attribute(&mut self, text: impl std::fmt::Display, color: impl Into<Color32>) -> Response;
}

impl CustomUi for egui::Ui {
    fn stat_tile<Num: Numeric>(
        &mut self,
        top_label: &str,
        bottom_label: &str,
        value: &mut Num,
    ) -> Response {
        StatTile::new(top_label, bottom_label, value).ui(self)
    }

    fn no_select_label(&mut self, text: impl Into<WidgetText>) -> Response {
        Label::new(text).selectable(false).ui(self)
    }

    fn selectable_value_enum<Value: PartialEq + std::fmt::Display + strum::IntoEnumIterator>(
        &mut self,
        current_value: &mut Value,
    ) {
        for variant in Value::iter() {
            let label = variant.to_string();
            self.selectable_value(current_value, variant, label);
        }
    }

    fn attribute(&mut self, text: impl std::fmt::Display, color: impl Into<Color32>) -> Response {
        let color = color.into();
        Frame::new()
            .stroke(Stroke::new(1.0, color))
            .inner_margin(vec2(5.0, -5.0))
            .outer_margin(0)
            .corner_radius(10)
            .show(self, |ui| {
                ui.no_select_label(
                    RichText::new(format!("{}", text))
                        .color(color)
                        .strong()
                        .size(8.0),
                )
            })
            .response
    }
}

#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct EnumSelect<'a, V>
where
    V: PartialEq + std::fmt::Display + strum::IntoEnumIterator,
{
    current_value: &'a mut V,
    label: &'a str,
}

impl<'a, V> EnumSelect<'a, V>
where
    V: PartialEq + std::fmt::Display + strum::IntoEnumIterator,
{
    pub fn new(current_value: &'a mut V, label: &'a str) -> Self {
        Self {
            current_value,
            label,
        }
    }
}

impl<V> Widget for EnumSelect<'_, V>
where
    V: PartialEq + std::fmt::Display + strum::IntoEnumIterator,
{
    fn ui(self, ui: &mut egui::Ui) -> Response {
        ComboBox::new(type_name::<V>(), self.label)
            .selected_text(self.current_value.to_string())
            .show_ui(ui, |ui| {
                for variant in V::iter() {
                    let label = variant.to_string();
                    ui.selectable_value(self.current_value, variant, label);
                }
            })
            .response
    }
}

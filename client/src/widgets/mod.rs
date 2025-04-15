use egui::{Label, Response, Rgba, Widget, WidgetText};
use emath::Numeric;
use stat_tile::StatTile;

pub mod frames;
pub mod group;
pub mod stat_tile;

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
}

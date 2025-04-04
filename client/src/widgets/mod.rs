use egui::Rgba;

pub mod frames;
pub mod group;

pub trait WithAlpha {
    fn with_alpha(self, alpha: f32) -> Self;
}

impl WithAlpha for Rgba {
    fn with_alpha(self, alpha: f32) -> Self {
        Self::from_rgba_premultiplied(self.r(), self.g(), self.b(), alpha)
    }
}

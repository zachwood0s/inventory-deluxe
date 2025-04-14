use super::CharacterTabImpl;

#[derive(Clone)]
pub struct AttributesTab;

impl CharacterTabImpl for AttributesTab {
    fn ui(&self, ui: &mut egui::Ui, ctx: super::CharacterCtx) {
        ui.label("attrs");
    }

    fn title(&self) -> &str {
        "ATTRIBUTES"
    }
}

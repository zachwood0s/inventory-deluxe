use super::CharacterTabImpl;

#[derive(Clone)]
pub struct InventoryTab;

impl CharacterTabImpl for InventoryTab {
    fn ui(&self, ui: &mut egui::Ui, ctx: super::CharacterCtx) {
        ui.label("inventory");
    }

    fn title(&self) -> &str {
        "INVENTORY"
    }
}

use common::{Item, ItemId};
use egui::Grid;
use log::{info, warn};

use crate::{listener::CommandQueue, state::DndState};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct EditState {
    item: Item,
    id: egui::Id,
}

impl EditState {
    fn load(ui: &mut egui::Ui, item_id: &ItemId, state: &DndState) -> Option<Self> {
        let id = ui.make_persistent_id("edit_item").with(item_id);

        // TODO: This copies the whole item every frame, might not be ideal
        // May want to arc/mutex this so that the clone is cheap
        ui.data_mut(|mem| {
            mem.get_persisted(id).or_else(|| {
                let state = EditState {
                    item: state.data.get_item(item_id)?.clone(),
                    id,
                };

                Some(state)
            })
        })
    }

    fn store(self, ui: &mut egui::Ui) {
        ui.data_mut(|mem| mem.insert_persisted(self.id, self))
    }
}

pub struct ItemEdit<'a, 'q> {
    item_id: &'a ItemId,
    state: &'a DndState,
    commands: &'a mut CommandQueue<'q>,
}

impl<'a, 'q> ItemEdit<'a, 'q> {
    pub fn new(
        item_id: &'a ItemId,
        state: &'a DndState,
        commands: &'a mut CommandQueue<'q>,
    ) -> Self {
        Self {
            item_id,
            state,
            commands,
        }
    }

    pub fn show(self, ui: &mut egui::Ui) {
        let Some(mut state) = EditState::load(ui, self.item_id, self.state) else {
            warn!("Failed to load edit state for item: {}", self.item_id);
            return;
        };

        Grid::new("item_grid").num_columns(2).show(ui, |ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut state.item.name);
            ui.end_row();

            ui.label("Description");
            // TODO: You won't be able to edit if no description was there to begin with
            if let Some(description) = &mut state.item.description {
                ui.text_edit_multiline(description);
            }
            ui.end_row();

            ui.label("Flavor Text");
            // TODO: You won't be able to edit if no description was there to begin with
            if let Some(flavor) = &mut state.item.flavor_text {
                ui.text_edit_singleline(flavor);
            }
            ui.end_row();

            ui.label("Quest Item");
            ui.checkbox(&mut state.item.quest_item, "");
            ui.end_row();

            ui.label("Equippable");
            ui.checkbox(&mut state.item.equippable, "");
            ui.end_row();

            ui.label("Item Category");
            ui.end_row();
        });

        if ui.button("Save").clicked() {
            info!("Save item");
            //self.commands.add(UpdateItem::new(state.item.clone()));
        }

        state.store(ui);
    }
}

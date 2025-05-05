#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::{
    sync::mpsc::{channel, Receiver},
    thread,
};

use common::{message::DndMessage, AbilityId, User};
use eframe::egui;
use egui::{CentralPanel, Window};
use egui_dock::{DockArea, DockState, NodeIndex, SurfaceIndex};
use listener::{CommandQueue, DndListener, Signal};
use message_io::events::EventSender;
use state::DndState;
use view::{edit::AbilityEdit, DndTab};

use clap::Parser;

pub mod listener;
pub mod prelude;
pub mod state;
pub mod view;
pub mod widgets;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    ip: Option<String>,
    name: Option<String>,
}

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let args = Args::parse();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1920.0, 1080.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            cc.egui_ctx.set_pixels_per_point(1.2);

            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

            cc.egui_ctx.set_fonts(fonts);

            cc.egui_ctx.style_mut(|style| {
                style.text_styles.insert(
                    egui::TextStyle::Name("stat_tile".into()),
                    egui::FontId::proportional(35.0),
                );
                style.text_styles.insert(
                    egui::TextStyle::Name("stat_tile_edit".into()),
                    egui::FontId::proportional(33.0),
                );
            });

            Ok(Box::new(MyApp::new(args)))
        }),
    )
}

struct MyApp {
    tree: DockState<DndTab>,
    counter: usize,
    state: DndState,

    server_ip: String,
    user_string: String,

    tx: Option<EventSender<Signal>>,
    rx: Option<Receiver<DndMessage>>,
}

impl MyApp {
    pub fn new(args: Args) -> Self {
        let tree = DockState::new(vec![
            DndTab::from_tab(view::Chat::default(), SurfaceIndex::main(), NodeIndex(1)),
            DndTab::from_tab(
                view::UiBoardState::default(),
                SurfaceIndex::main(),
                NodeIndex(2),
            ),
        ]);

        Self {
            tree,
            counter: 3,
            tx: None,
            rx: None,
            state: Default::default(),
            server_ip: args.ip.unwrap_or_default(),
            user_string: args.name.unwrap_or_default(),
        }
    }

    fn show_login(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |_| {
            Window::new("Login").collapsible(false).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Server: ");
                    ui.text_edit_singleline(&mut self.server_ip);
                });
                ui.horizontal(|ui| {
                    ui.label("Name: ");
                    let input = ui.text_edit_singleline(&mut self.user_string);
                    if input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let user = User {
                            name: self.user_string.clone(),
                        };

                        self.state.user = Some(user.clone());

                        // Create the server listener with the user that we've selected
                        let (tx_listener, rx_main) = channel();

                        let listener =
                            DndListener::new(tx_listener, user, &self.server_ip).unwrap();

                        self.tx = Some(listener.event_sender());
                        self.rx = Some(rx_main);

                        thread::spawn(move || listener.run());
                    }
                })
            });
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.state.user.is_none() {
            self.show_login(ctx, _frame);
        } else {
            let mut added_nodes = Vec::new();

            let mut command_queue = Vec::new();

            {
                let mut tab_viewer = view::TabViewer {
                    added_nodes: &mut added_nodes,
                    state: &self.state,
                    network: CommandQueue {
                        command_queue: &mut command_queue,
                    },
                };

                DockArea::new(&mut self.tree)
                    .style(egui_dock::Style::from_egui(ctx.style().as_ref()))
                    .show_add_buttons(true)
                    .show_add_popup(true)
                    .show_leaf_close_all_buttons(false)
                    .show_leaf_collapse_buttons(false)
                    .show(ctx, &mut tab_viewer);
            }

            {
                let mut ability_edit = self.state.ability_edit.is_some();
                let id = egui::Id::new("edit_ability");
                let layer_id = egui::LayerId::new(egui::Order::Middle, id);

                let resp = Window::new("Edit Ability")
                    .id(id)
                    .open(&mut ability_edit)
                    .show(ctx, |ui| {
                        let Some(ability_id) = self.state.ability_edit.as_ref() else {
                            ui.label("Select an ability to edit");
                            return;
                        };

                        AbilityEdit::new(
                            ability_id,
                            &self.state,
                            &mut CommandQueue {
                                command_queue: &mut command_queue,
                            },
                        )
                        .show(ui);
                    });

                // Handle window closed
                if !ability_edit {
                    self.state.ability_edit = None;
                }

                // While window is show, move it to the top
                if resp.is_some() {
                    ctx.move_to_top(layer_id);
                }
            }

            for msg in self.rx.as_ref().unwrap().try_iter() {
                self.state.process(msg);
            }

            for command in command_queue.drain(..) {
                command.execute(&mut self.state, self.tx.as_ref().unwrap());
            }

            added_nodes.drain(..).for_each(|node| {
                self.tree
                    .set_focused_node_and_surface((node.surface, node.node));
                self.tree.push_to_focused_leaf(DndTab {
                    kind: node.kind,
                    surface: node.surface,
                    node: NodeIndex(self.counter),
                });
                self.counter += 1;
            });
        }

        ctx.request_repaint();
    }
}

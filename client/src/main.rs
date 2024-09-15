#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::{
    sync::mpsc::{channel, Receiver},
    thread,
};

use common::message::DndMessage;
use eframe::egui;
use egui_dock::{DockArea, DockState, NodeIndex, SurfaceIndex};
use listener::{DndListener, Signal};
use message_io::events::EventSender;
use view::DndTab;

mod listener;
mod view;

fn main() -> eframe::Result {
    let (tx_listener, rx_main) = channel();

    let listener =
        DndListener::new(tx_listener).map_err(|x| eframe::Error::AppCreation(x.into()))?;

    let tx_main = listener.event_sender();

    thread::spawn(move || listener.run());

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(MyApp::new(tx_main, rx_main)))
        }),
    )
}

struct MyApp {
    tree: DockState<DndTab>,
    counter: usize,

    tx: EventSender<Signal>,
    rx: Receiver<DndMessage>,
}

impl MyApp {
    pub fn new(tx: EventSender<Signal>, rx: Receiver<DndMessage>) -> Self {
        let tree = DockState::new(vec![
            DndTab::from_tab(view::Chat::default(), SurfaceIndex::main(), NodeIndex(1)),
            DndTab::from_tab(view::Board, SurfaceIndex::main(), NodeIndex(2)),
        ]);

        Self {
            tree,
            counter: 3,
            tx,
            rx,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut added_nodes = Vec::new();

        DockArea::new(&mut self.tree)
            .style(egui_dock::Style::from_egui(ctx.style().as_ref()))
            .show_add_buttons(true)
            .show_add_popup(true)
            .show(
                ctx,
                &mut view::TabViewer {
                    added_nodes: &mut added_nodes,
                    tx: &self.tx,
                    rx: &self.rx,
                },
            );

        added_nodes.drain(..).for_each(|node| {
            self.tree
                .set_focused_node_and_surface((node.surface, node.node));
            self.tree.push_to_focused_leaf(DndTab {
                kind: node.kind,
                surface: node.surface,
                node: NodeIndex(self.counter),
            });
            self.counter += 1;
        })
    }
}

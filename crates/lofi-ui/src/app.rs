use eframe::egui;
use lofi_sim::NodeSnapshot;

use crate::engine::{Command, Engine, Snapshot};
use crate::lcd::draw_lcd;

const PANEL_WIDTH: f32 = 300.0;
const LCD_SCALE: f32 = 2.0;

/// Editable mirror of one device's controls. The simulation is authoritative;
/// these hold slider state between frames and only push on change.
struct UiNode {
    pan: f32,
    gain: f32,
    drift_ppb: i32,
    offset_us: i64,
    mute: bool,
    solo: bool,
}

impl UiNode {
    fn from_snapshot(node: &NodeSnapshot) -> Self {
        Self {
            pan: node.mix.pan,
            gain: node.mix.gain,
            drift_ppb: node.drift_ppb,
            offset_us: node.local_offset_us,
            mute: node.mix.mute,
            solo: node.mix.solo,
        }
    }
}

pub struct LofiApp {
    engine: Engine,
    nodes: Vec<UiNode>,
    sync_enabled: bool,
}

impl LofiApp {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine,
            nodes: Vec::new(),
            sync_enabled: true,
        }
    }

    fn sync_mirror(&mut self, snap: &Snapshot) {
        if self.nodes.len() != snap.nodes.len() {
            self.nodes = snap.nodes.iter().map(UiNode::from_snapshot).collect();
        }
        self.sync_enabled = snap.sync_enabled;
    }
}

impl eframe::App for LofiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let snap = self.engine.snapshot();
        self.sync_mirror(&snap);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("➕ Add device").clicked() {
                    self.engine.send(Command::AddNode);
                }
                if ui.button("▶ Start all").clicked() {
                    self.engine.send(Command::SetAllPlay(true));
                }
                if ui.button("⏸ Stop all").clicked() {
                    self.engine.send(Command::SetAllPlay(false));
                }
                let mut sync = self.sync_enabled;
                if ui.checkbox(&mut sync, "Mesh sync").changed() {
                    self.sync_enabled = sync;
                    self.engine.send(Command::SetSync(sync));
                }
                ui.separator();
                ui.label(format!("phase spread {} us", snap.spread_us));
                ui.label(format!("t {} ms", snap.global_us / 1000));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let mut remove = None;
                    for ix in 0..snap.nodes.len() {
                        if self.device_panel(ui, ix, &snap.nodes[ix]) {
                            remove = Some(ix);
                        }
                    }
                    if let Some(ix) = remove {
                        self.engine.send(Command::RemoveNode(ix));
                    }
                });
            });
        });

        ctx.request_repaint();
    }
}

impl LofiApp {
    /// Returns true if the user asked to remove this device.
    fn device_panel(&mut self, ui: &mut egui::Ui, ix: usize, snap: &NodeSnapshot) -> bool {
        let mut remove = false;
        ui.group(|ui| {
            ui.set_width(PANEL_WIDTH);
            ui.vertical(|ui| {
                draw_lcd(ui, &snap.display, LCD_SCALE);

                ui.horizontal(|ui| {
                    let label = if snap.display.playing {
                        "⏸ Stop"
                    } else {
                        "▶ Start"
                    };
                    if ui.button(label).clicked() {
                        self.engine.send(Command::TogglePlay(ix));
                    }
                    let node = &mut self.nodes[ix];
                    if ui.toggle_value(&mut node.mute, "Mute").changed() {
                        self.engine.send(Command::SetMute(ix, node.mute));
                    }
                    if ui.toggle_value(&mut node.solo, "Solo").changed() {
                        self.engine.send(Command::SetSolo(ix, node.solo));
                    }
                    if ui.button("✕").clicked() {
                        remove = true;
                    }
                });

                let node = &mut self.nodes[ix];
                if ui
                    .add(egui::Slider::new(&mut node.pan, -1.0..=1.0).text("Pan"))
                    .changed()
                {
                    self.engine.send(Command::SetPan(ix, node.pan));
                }
                if ui
                    .add(egui::Slider::new(&mut node.gain, 0.0..=1.5).text("Vol"))
                    .changed()
                {
                    self.engine.send(Command::SetGain(ix, node.gain));
                }
                if ui
                    .add(
                        egui::Slider::new(&mut node.drift_ppb, -200_000..=200_000)
                            .text("Drift ppb"),
                    )
                    .changed()
                {
                    self.engine.send(Command::SetDrift(ix, node.drift_ppb));
                }
                if ui
                    .add(
                        egui::Slider::new(&mut node.offset_us, -500_000..=500_000)
                            .text("Offset us"),
                    )
                    .changed()
                {
                    self.engine.send(Command::SetOffset(ix, node.offset_us));
                }
            });
        });
        remove
    }
}

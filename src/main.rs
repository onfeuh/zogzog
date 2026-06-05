use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use chrono::Local;
use eframe::egui;
use tokio::sync::mpsc;

use crate::w3gs::{GameType, W3gsPacket};

mod client;
mod host;
mod w3gs;

const CARGO_CRATE_NAME: &str = env!("CARGO_CRATE_NAME");
const BUILD_VERSION: &str = env!("BUILD_VERSION");

const WC3_GAME_PORT: u16 = 6112;

#[derive(Debug)]
pub enum NetworkEvent {
    PacketReceived { addr: SocketAddr, data: Vec<u8> },
    PacketSent { addr: SocketAddr, data: Vec<u8> },
    ServerStarted,
    Connecting,
    Connected,
    Disconnected,
    Error(String),
    PeerCount(usize),
}

#[derive(Debug, PartialEq)]
pub enum ProxyState {
    Idle,
    Hosting,
    Connecting,
    Connected,
}

fn main() -> eframe::Result {
    // tokio runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .name("my-runtime")
        .build()
        .unwrap();

    // eframe
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([650.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Zogzog",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<MyApp>::new(MyApp::new(rt)))
        }),
    )
}

struct MyApp {
    host_ip_string: String,
    port_string: String,
    proxy_ip_string: String,
    runtime: tokio::runtime::Runtime,
    event_rx: mpsc::Receiver<NetworkEvent>,
    event_tx: mpsc::Sender<NetworkEvent>,
    state: ProxyState,
    error: Option<String>,
    log: Vec<String>,
    game_type: GameType,
    version: u32,
    peer_count: u32,
}

impl MyApp {
    fn new(runtime: tokio::runtime::Runtime) -> Self {
        let host_default_ip = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
        let proxy_default_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2));
        let (event_tx, event_rx) = mpsc::channel(32);
        Self {
            host_ip_string: host_default_ip.to_string(),
            port_string: 7000.to_string(),
            proxy_ip_string: proxy_default_ip.to_string(),
            log: vec![],
            event_rx,
            event_tx,
            state: ProxyState::Idle,
            error: None,
            game_type: GameType::FrozenThrone,
            version: 29,
            peer_count: 0,
            runtime,
        }
    }
}

impl eframe::App for MyApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                NetworkEvent::PacketReceived { addr, data } => {
                    let time = Local::now().format("%H:%M:%S");
                    let p = W3gsPacket::from_repr(data[1]).unwrap_or(W3gsPacket::Unknown);
                    self.log
                        .push(format!("{}: RECV from {}: [{}]", time, addr, p,));
                }
                NetworkEvent::ServerStarted => {
                    self.error = None;
                    self.state = ProxyState::Hosting
                }
                NetworkEvent::PacketSent { addr, data } => {
                    let time = Local::now().format("%H:%M:%S");
                    let p = W3gsPacket::from_repr(data[1]).unwrap_or(W3gsPacket::Unknown);
                    self.log
                        .push(format!("{}: RECV from {}: [{}]", time, addr, p,));
                }
                NetworkEvent::Connected => self.state = ProxyState::Connected,
                NetworkEvent::Connecting => {
                    self.error = None;
                    self.state = ProxyState::Connecting
                }
                NetworkEvent::Disconnected => self.state = ProxyState::Idle,
                NetworkEvent::Error(e) => {
                    self.error = Some(e);
                    self.state = ProxyState::Idle
                }
                NetworkEvent::PeerCount(u) => self.peer_count = u as u32,
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::bottom("footer").show_inside(ui, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{}: build {}", CARGO_CRATE_NAME, BUILD_VERSION))
                        .weak()
                        .small(),
                );
            });
            ui.add_space(2.0);
        });

        egui::Panel::top("header").show_inside(ui, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::Image::new(egui::include_image!("../icon.png"))
                        .fit_to_exact_size(egui::vec2(64.0, 64.0)),
                );
                ui.vertical(|ui| {
                    ui.heading("Zogzog");
                    ui.label(egui::RichText::new("2003 gaming with friends - the easy way").weak());
                    ui.label(egui::RichText::new("LAN bridge utility made by ~onfeuh").weak());
                });
            });
            ui.add_space(2.0);
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add_space(8.0);

                match self.state {
                    ProxyState::Idle => {
                        self.join_panel(ui);
                    }
                    ProxyState::Hosting => {
                        status_box(
                            ui,
                            egui::Color32::from_rgb(20, 20, 20),
                            format!(
                                "Listening on {}:{} — {} peer(s)",
                                self.host_ip_string, self.port_string, self.peer_count
                            ),
                        );
                        ui.add_space(6.0);

                        egui::Frame::new()
                            .fill(ui.visuals().extreme_bg_color)
                            .inner_margin(8.0)
                            .corner_radius(4.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.set_height(ui.available_height());
                                egui::ScrollArea::vertical()
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        for line in &self.log {
                                            ui.monospace(line);
                                        }
                                    });
                            });
                    }
                    ProxyState::Connecting => {
                        ui.label(format!(
                            "Connecting to {}:{}...",
                            self.host_ip_string, self.port_string
                        ));
                    }
                    ProxyState::Connected => {
                        status_box(
                            ui,
                            egui::Color32::from_rgb(20, 20, 20),
                            format!("Connected to {}:{}", self.host_ip_string, self.port_string),
                        );
                        ui.add_space(6.0);

                        egui::Frame::new()
                            .fill(ui.visuals().extreme_bg_color)
                            .inner_margin(8.0)
                            .corner_radius(4.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.set_height(ui.available_height());
                                egui::ScrollArea::vertical()
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        for line in &self.log {
                                            ui.monospace(line);
                                        }
                                    });
                            });
                    }
                }
            });
        });
    }
}

impl MyApp {
    fn join_panel(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("join_form")
            .num_columns(2)
            .spacing([4.0, 3.0])
            .show(ui, |ui| {
                let prev = ui.visuals().extreme_bg_color;

                // Game row
                ui.label("Game");
                egui::ComboBox::from_id_salt("game_type")
                    .width(150.0)
                    .selected_text(self.game_type.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.game_type,
                            GameType::ReignOfChaos,
                            "Reign of Chaos",
                        );
                        ui.selectable_value(
                            &mut self.game_type,
                            GameType::FrozenThrone,
                            "Frozen Throne",
                        );
                    });
                ui.end_row();

                // Version row
                ui.label("Version");
                egui::ComboBox::from_id_salt("version")
                    .width(150.0)
                    .selected_text(format!("1.{}", self.version))
                    .show_ui(ui, |ui| {
                        for v in [27u32, 28, 29] {
                            ui.selectable_value(&mut self.version, v, format!("1.{v}"));
                        }
                    });

                ui.end_row();

                // IP row
                ui.label("Host IP Address");
                if self.host_ip_string.parse::<std::net::IpAddr>().is_err() {
                    ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(80, 20, 20);
                }
                ui.text_edit_singleline(&mut self.host_ip_string);
                ui.visuals_mut().extreme_bg_color = prev;
                ui.end_row();

                // PORT row
                ui.label("Bridge Port");
                if self.port_string.parse::<u32>().is_err() {
                    ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(80, 20, 20);
                }
                ui.text_edit_singleline(&mut self.port_string);
                ui.visuals_mut().extreme_bg_color = prev;
                ui.end_row();

                // IP proxy row
                /*
                ui.label("Proxy IP Address*");
                if self.proxy_ip_string.parse::<std::net::IpAddr>().is_err() {
                    ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(80, 20, 20);
                }
                ui.text_edit_singleline(&mut self.proxy_ip_string);
                ui.visuals_mut().extreme_bg_color = prev;
                ui.end_row();
                 */
            });

        ui.add_space(12.0);

        ui.scope(|ui| {
            ui.set_max_width(330.0);
            ui.label(egui::RichText::new("Host").strong());
            ui.label("Forward the bridge's port (TCP and UDP), then share your public IP.");
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Client").strong());
            ui.label("Enter the host's public IP and bridge port, then connect.");
        });
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            let port = self.port_string.parse::<u16>().ok();
            let host_ip = self.host_ip_string.parse::<std::net::IpAddr>().ok();
            let proxy_ip = self.proxy_ip_string.parse::<std::net::IpAddr>().ok();
            let valid = port.is_some() && host_ip.is_some() && proxy_ip.is_some();

            if ui
                .add_enabled(
                    valid,
                    egui::Button::new("Host").min_size(egui::vec2(80.0, 32.0)),
                )
                .clicked()
            {
                let tx = self.event_tx.clone();
                let bridge_addr = SocketAddr::new(host_ip.unwrap(), port.unwrap());
                let game_type = self.game_type;
                let version = self.version;
                let tx_err = tx.clone();
                self.runtime.spawn(async move {
                    if let Err(e) = host::main(bridge_addr, game_type, version, tx).await {
                        tx_err.send(NetworkEvent::Error(e.to_string())).await.ok();
                    }
                });
            }

            if ui
                .add_enabled(
                    valid,
                    egui::Button::new("Connect to...").min_size(egui::vec2(120.0, 32.0)),
                )
                .clicked()
            {
                let tx = self.event_tx.clone();
                let bridge_addr = SocketAddr::new(host_ip.unwrap(), port.unwrap());
                let proxy_addr = SocketAddr::new(proxy_ip.unwrap(), port.unwrap());
                let tx_err = tx.clone();
                self.runtime.spawn(async move {
                    if let Err(e) = client::main(bridge_addr, proxy_addr, tx).await {
                        println!("{:?}", e);
                        tx_err.send(NetworkEvent::Error(e.to_string())).await.ok();
                    }
                });
            }
        });

        if let Some(err) = self.error.clone() {
            ui.add_space(8.0);
            status_box(ui, egui::Color32::from_rgb(80, 20, 20), err);
        }
    }
}

fn status_box(ui: &mut egui::Ui, color: egui::Color32, text: impl Into<String>) {
    egui::Frame::new()
        .fill(color)
        .inner_margin(8.0)
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::WHITE, text.into());
            });
        });
}

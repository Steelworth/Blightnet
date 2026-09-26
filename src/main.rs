mod app;
mod audio;
mod catalog;
mod chars;
mod chess;
mod crypt;
mod daemon;
mod dice;
mod images;
mod maps;
mod mesh;
mod names;
mod net;
mod nethook;
mod recon;
mod rotn;
mod stations;
mod netspace;
mod sys;
mod term;
mod theme;
mod video;

use app::Blightnet;
use eframe::egui;
use eframe::egui_wgpu::WgpuConfiguration;
use eframe::wgpu;
use std::path::PathBuf;

fn root_dir() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let mut p = dir.to_path_buf();
            for _ in 0..6 {
                candidates.push(p.clone());
                if !p.pop() {
                    break;
                }
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd);
    }
    if let Ok(image) = std::env::var("APPIMAGE") {
        if let Some(dir) = PathBuf::from(image).parent() {
            candidates.push(dir.to_path_buf());
        }
    }
    for dir in candidates {
        if dir.join("data").join("mixer-catalog.json").is_file() {
            return dir;
        }
    }
    PathBuf::from(".")
}

fn root_from_args() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2).find_map(|w| {
        if w[0] == "--root" {
            Some(PathBuf::from(&w[1]))
        } else {
            None
        }
    })
}

fn main() -> eframe::Result<()> {
    let root = root_from_args().unwrap_or_else(root_dir);
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
    match cmd {
        "daemon" => {
            std::process::exit(crate::daemon::run(root));
        }
        "daemon-status" | "node-status" => {
            std::process::exit(crate::daemon::print_status(&root));
        }
        "daemon-stop" | "node-stop" => {
            std::process::exit(crate::daemon::stop(&root));
        }
        "help" | "-h" | "--help" => {
            eprintln!(
                "blightnet                 UI (starts the node if needed)\n\
                 blightnet daemon          background node — table, invite, reconnect\n\
                 blightnet daemon-status   is the node up\n\
                 blightnet daemon-stop     stop the node"
            );
            std::process::exit(0);
        }
        _ => {}
    }
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 640.0])
            .with_fullscreen(false)
            .with_decorations(false)
            .with_maximized(true)
            .with_resizable(true)
            .with_title("Blightnet")
            .with_app_id("blightnet"),
        centered: false,
        vsync: true,
        hardware_acceleration: eframe::HardwareAcceleration::Preferred,
        wgpu_options: WgpuConfiguration {
            // AutoVsync stays on a blocking present (FifoRelaxed/Fifo) without
            // the Wayland stall Fifo+latency-1 can hit at startup.
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: Some(2),
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "Blightnet",
        native,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            crate::theme::install_fonts(&cc.egui_ctx);
            match Blightnet::new(root.clone()) {
                Ok(app) => Ok(Box::new(app)),
                Err(e) => {
                    eprintln!("blightnet: {e}");
                    Err(e.into())
                }
            }
        }),
    )
}

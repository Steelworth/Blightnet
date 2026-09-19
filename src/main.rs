mod app;
mod audio;
mod catalog;
mod chars;
mod dice;
mod images;
mod maps;
mod names;
mod net;
mod nethook;
mod netspace;
mod sys;
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
    for dir in candidates {
        if dir.join("data").join("mixer-catalog.json").is_file() {
            return dir;
        }
    }
    PathBuf::from(".")
}

fn main() -> eframe::Result<()> {
    let root = root_dir();
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

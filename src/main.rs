use anyhow::Result;
use eframe::{self};
use app::DownloaderApp;

mod profile;
mod downloader;
mod uploader;
mod app;
mod utils;
mod config;

fn main() -> Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([800.0, 1000.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Менеджер контента Wildberries",
        native_options,
        Box::new(|_cc| Ok(Box::new(DownloaderApp::default()))),
    ).map_err(|e| anyhow::anyhow!("Ошибка GUI: {}", e))?;
    Ok(())
}
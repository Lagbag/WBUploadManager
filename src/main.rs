mod app;
mod config;
mod downloader;
mod profile;
mod uploader;
mod utils;

use anyhow::Result;
use app::DownloaderApp;
use eframe::{self};

fn main() -> Result<()> {
    env_logger::init(); // Инициализация логгера
    log::info!("Приложение запущено");

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([800.0, 1000.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Менеджер контента Wildberries",
        native_options,
        Box::new(|_cc| Ok(Box::new(DownloaderApp::default()))),
    )
    .map_err(|e| anyhow::anyhow!("Ошибка GUI: {}", e))?;
    Ok(())
}

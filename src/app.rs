use std::sync::{Arc, Mutex};
use std::path::Path;
use std::fs;
use eframe::egui;
use eframe::App;
use rfd::FileDialog;
use arboard::Clipboard;
use crate::profile::{Profile, ProfileManager};
use crate::downloader::{Downloader, FileInfo};
use crate::uploader::WbUploader;
use crate::config::Config;

pub struct DownloaderApp {
    urls: String,
    download_path: String,
    file_names: String,
    profile_manager: ProfileManager,
    new_profile_name: String,
    logs: Arc<Mutex<Vec<String>>>,
    is_processing: Arc<Mutex<bool>>,
    total_files: Arc<Mutex<Option<usize>>>,
    processed_files: Arc<Mutex<usize>>,
    show_logs: bool,
    use_local_path: bool,
    local_source_path: String,
    failed_vendor_codes: Arc<Mutex<Vec<String>>>,
}

impl Default for DownloaderApp {
    fn default() -> Self {
        Self {
            urls: String::new(),
            download_path: String::new(),
            file_names: String::new(),
            profile_manager: ProfileManager::new().unwrap_or_else(|_| ProfileManager {
                profiles: vec![Profile {
                    name: "Default".to_string(),
                    api_key: String::new(),
                }],
                selected_index: 0,
                config: Config::new().unwrap(),
            }),
            new_profile_name: String::new(),
            logs: Arc::new(Mutex::new(Vec::new())),
            is_processing: Arc::new(Mutex::new(false)),
            total_files: Arc::new(Mutex::new(None)),
            processed_files: Arc::new(Mutex::new(0)),
            show_logs: false,
            use_local_path: false,
            local_source_path: String::new(),
            failed_vendor_codes: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl App for DownloaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let visuals = if ctx.style().visuals.dark_mode {
            let mut visuals = egui::Visuals::dark();
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 30, 30);
            visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(50, 50, 50);
            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(80, 80, 80);
            visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
            visuals.override_text_color = Some(egui::Color32::WHITE);
            visuals
        } else {
            let mut visuals = egui::Visuals::light();
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(245, 245, 245);
            visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
            visuals.widgets.inactive.bg_fill = egui::Color32::WHITE;
            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(200, 200, 200);
            visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
            visuals.override_text_color = Some(egui::Color32::BLACK);
            visuals.selection.bg_fill = egui::Color32::from_rgb(180, 200, 255);
            visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
            visuals
        };
        ctx.set_visuals(visuals);

        if self.show_logs {
            egui::SidePanel::right("logs_panel")
                .resizable(true)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.heading(egui::RichText::new("📜 Логи").strong().size(22.0));
                    ui.add_space(10.0);
                    let logs = self.logs.lock().unwrap();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for log in logs.iter() {
                            ui.label(log);
                            if ui.button("📋").on_hover_text("Скопировать").clicked() {
                                let mut clipboard = Clipboard::new().unwrap();
                                clipboard.set_text(log).unwrap();
                            }
                            ui.add_space(5.0);
                        }
                    });
                    ui.add_space(10.0);
                    if ui.button("🗑 Очистить логи").clicked() {
                        let mut logs = self.logs.lock().unwrap();
                        logs.clear();
                    }
                    if ui.button("❌ Закрыть").clicked() {
                        self.show_logs = false;
                    }
                });
        }

        egui::CentralPanel::default().frame(egui::Frame {
            inner_margin: egui::Margin::same(20.0),
            fill: if ctx.style().visuals.dark_mode {
                egui::Color32::from_rgb(60, 80, 180)
            } else {
                egui::Color32::from_rgb(180, 200, 255)
            },
            rounding: egui::Rounding::same(8.0),
            ..Default::default()
        }).show(ctx, |ui| {
            ui.add_space(20.0);
            ui.heading(egui::RichText::new("🔥 Менеджер контента Wildberries").strong().size(32.0));
            ui.add_space(30.0);

            ui.group(|ui| {
                ui.visuals_mut().widgets.noninteractive.rounding = egui::Rounding::same(8.0);
                ui.visuals_mut().widgets.noninteractive.bg_fill = if ctx.style().visuals.dark_mode {
                    egui::Color32::from_rgb(70, 70, 70)
                } else {
                    egui::Color32::from_rgb(220, 220, 220)
                };
                ui.label(egui::RichText::new("👤 Управление профилями").strong().size(22.0));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    egui::ComboBox::from_label("Профиль")
                        .selected_text(&self.profile_manager.current_profile().name)
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            for (i, profile) in self.profile_manager.profiles.iter().enumerate() {
                                if ui.selectable_label(self.profile_manager.selected_index == i, &profile.name).clicked() {
                                    self.profile_manager.selected_index = i;
                                }
                            }
                        });
                    ui.add(egui::TextEdit::singleline(&mut self.new_profile_name).hint_text("Новый профиль").desired_width(150.0));
                    if ui.button("➕ Добавить").clicked() && !self.new_profile_name.is_empty() {
                        self.profile_manager.add_profile(self.new_profile_name.clone());
                        self.new_profile_name.clear();
                        if let Err(e) = self.profile_manager.save() {
                            let mut logs = self.logs.lock().unwrap();
                            logs.push(format!("Ошибка сохранения профилей: {}", e));
                        }
                    }
                });
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("🔑 WB API ключ:").strong());
                    ui.add_space(5.0);
                    ui.add(egui::TextEdit::multiline(&mut self.profile_manager.current_profile_mut().api_key)
                        .desired_width(400.0)
                        .desired_rows(3));
                    if ui.button("💾 Сохранить").clicked() {
                        let api_key = self.profile_manager.current_profile().api_key.trim();
                        if api_key.is_empty() {
                            let mut logs = self.logs.lock().unwrap();
                            logs.push("Ошибка: API ключ не может быть пустым".to_string());
                        } else {
                            match self.profile_manager.save() {
                                Ok(()) => {
                                    let mut logs = self.logs.lock().unwrap();
                                    logs.push("API ключ успешно сохранен".to_string());
                                }
                                Err(e) => {
                                    let mut logs = self.logs.lock().unwrap();
                                    logs.push(format!("Ошибка сохранения API ключа: {}", e));
                                }
                            }
                        }
                        ctx.request_repaint();
                    }
                });
            });

            ui.add_space(30.0);
            ui.group(|ui| {
                ui.visuals_mut().widgets.noninteractive.rounding = egui::Rounding::same(8.0);
                ui.visuals_mut().widgets.noninteractive.bg_fill = if ctx.style().visuals.dark_mode {
                    egui::Color32::from_rgb(70, 70, 70)
                } else {
                    egui::Color32::from_rgb(220, 220, 220)
                };
                ui.label(egui::RichText::new("📥 Источник файлов").strong().size(22.0));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.use_local_path, "Использовать локальную папку");
                });
                ui.add_space(10.0);
                if !self.use_local_path {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🔗 Ссылки на Яндекс.Диск (через запятую):").strong());
                        text_edit_with_context_menu(ui, &mut self.urls, 400.0, "https://disk.yandex.ru/d/xxx/09.06,https://disk.yandex.ru/d/xxx/10.06");
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("📂 Локальная папка:").strong());
                        ui.add(egui::TextEdit::singleline(&mut self.local_source_path).desired_width(300.0));
                        if ui.button("📁 Выбрать").clicked() {
                            if let Some(path) = FileDialog::new().pick_folder() {
                                self.local_source_path = path.to_string_lossy().to_string();
                            }
                        }
                    });
                }
            });

            ui.add_space(30.0);
            ui.group(|ui| {
                ui.visuals_mut().widgets.noninteractive.rounding = egui::Rounding::same(8.0);
                ui.visuals_mut().widgets.noninteractive.bg_fill = if ctx.style().visuals.dark_mode {
                    egui::Color32::from_rgb(70, 70, 70)
                } else {
                    egui::Color32::from_rgb(220, 220, 220)
                };
                ui.label(egui::RichText::new("📁 Путь сохранения").strong().size(22.0));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("📂 Путь:").strong());
                    ui.add(egui::TextEdit::singleline(&mut self.download_path).desired_width(300.0));
                    if ui.button("📁 Выбрать").clicked() {
                        if let Some(path) = FileDialog::new().pick_folder() {
                            self.download_path = path.to_string_lossy().to_string();
                        }
                    }
                });
            });

            ui.add_space(30.0);
            ui.group(|ui| {
                ui.visuals_mut().widgets.noninteractive.rounding = egui::Rounding::same(8.0);
                ui.visuals_mut().widgets.noninteractive.bg_fill = if ctx.style().visuals.dark_mode {
                    egui::Color32::from_rgb(70, 70, 70)
                } else {
                    egui::Color32::from_rgb(220, 220, 220)
                };
                ui.label(egui::RichText::new("📋 Vendor Codes").strong().size(22.0));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🔢 Список vendor codes (по одному на строке):").strong());
                    ui.vertical(|ui| {
                        egui::ScrollArea::vertical()
                            .max_height(100.0)
                            .show(ui, |ui| {
                                text_edit_with_context_menu(ui, &mut self.file_names, 400.0, "PRINT-NNS-YVG-001\nPRINT-NNS-YVG-002");
                            });
                    });
                });
            });

            ui.add_space(30.0);
            let is_processing = *self.is_processing.lock().unwrap();
            ui.add_enabled_ui(!is_processing, |ui| {
                let button = ui.add(egui::Button::new("🚀 Запуск").rounding(8.0));
                if button.clicked() {
                    let urls = self.urls.clone();
                    let download_path = self.download_path.clone();
                    let local_source_path = self.local_source_path.clone();
                    let vendor_codes: Vec<String> = self.file_names.trim()
                        .lines()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let api_key = self.profile_manager.current_profile().api_key.clone();
                    if vendor_codes.is_empty() || download_path.is_empty() || (self.use_local_path && local_source_path.is_empty()) || (!self.use_local_path && urls.is_empty()) {
                        let mut logs = self.logs.lock().unwrap();
                        logs.push("Ошибка: заполните все поля".to_string());
                        return;
                    }
                    if !self.use_local_path && !urls.split(',').all(|s| s.trim().contains("disk.yandex.ru/d/")) {
                        let mut logs = self.logs.lock().unwrap();
                        logs.push("Ошибка: все ссылки должны быть на Яндекс.Диск".to_string());
                        return;
                    }
                    if !Path::new(&download_path).is_absolute() {
                        let mut logs = self.logs.lock().unwrap();
                        logs.push("Ошибка: путь для сохранения должен быть абсолютным".to_string());
                        return;
                    }
                    if self.use_local_path && !Path::new(&local_source_path).is_absolute() {
                        let mut logs = self.logs.lock().unwrap();
                        logs.push("Ошибка: локальный путь должен быть абсолютным".to_string());
                        return;
                    }
                    if api_key.is_empty() {
                        let mut logs = self.logs.lock().unwrap();
                        logs.push("Ошибка: API ключ не указан".to_string());
                        return;
                    }

                    let logs = Arc::clone(&self.logs);
                    let is_processing = Arc::clone(&self.is_processing);
                    let total_files = Arc::clone(&self.total_files);
                    let processed_files = Arc::clone(&self.processed_files);
                    let use_local_path = self.use_local_path;
                    let failed_vendor_codes = Arc::clone(&self.failed_vendor_codes);
                    let file_names = Arc::new(Mutex::new(self.file_names.clone()));
                    let public_keys: Vec<String> = urls
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    {
                        let mut logs = logs.lock().unwrap();
                        logs.push("Начало обработки...".to_string());
                    }
                    *is_processing.lock().unwrap() = true;
                    *processed_files.lock().unwrap() = 0;
                    *total_files.lock().unwrap() = Some(vendor_codes.len());
                    failed_vendor_codes.lock().unwrap().clear();

                    let public_keys_for_thread = public_keys.clone();
                    std::thread::spawn(move || {
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push("Запущен фоновый поток".to_string());
                        }

                        let files = if use_local_path {
                            {
                                let mut logs = logs.lock().unwrap();
                                logs.push("Инициализация Downloader для локального режима".to_string());
                            }
                            let downloader = match Downloader::new(Vec::new(), download_path.clone(), vendor_codes.clone()) {
                                Ok(d) => d,
                                Err(e) => {
                                    {
                                        let mut logs = logs.lock().unwrap();
                                        logs.push(format!("Ошибка инициализации: {}", e));
                                    }
                                    *is_processing.lock().unwrap() = false;
                                    return;
                                }
                            };
                            {
                                let mut logs = logs.lock().unwrap();
                                logs.push(format!("Начало сканирования локальной папки: {}", local_source_path));
                            }
                            match downloader.find_local_files(&local_source_path, &logs) {
                                Ok(files) => {
                                    {
                                        let mut logs = logs.lock().unwrap();
                                        logs.push(format!("Найдено файлов: {}", files.len()));
                                    }
                                    files
                                }
                                Err(e) => {
                                    {
                                        let mut logs = logs.lock().unwrap();
                                        logs.push(format!("Ошибка сканирования локальной папки: {}", e));
                                    }
                                    *is_processing.lock().unwrap() = false;
                                    return;
                                }
                            }
                        } else {
                            {
                                let mut logs = logs.lock().unwrap();
                                logs.push("Инициализация Downloader для Яндекс.Диска".to_string());
                            }
                            let downloader = match Downloader::new(public_keys_for_thread.clone(), download_path.clone(), vendor_codes.clone()) {
                                Ok(d) => d,
                                Err(e) => {
                                    {
                                        let mut logs = logs.lock().unwrap();
                                        logs.push(format!("Ошибка инициализации: {}", e));
                                    }
                                    *is_processing.lock().unwrap() = false;
                                    return;
                                }
                            };
                            {
                                let mut logs = logs.lock().unwrap();
                                logs.push(format!("Начало загрузки с URL: {:?}", public_keys_for_thread));
                            }
                            if let Err(e) = fs::create_dir_all(&download_path) {
                                {
                                    let mut logs = logs.lock().unwrap();
                                    logs.push(format!("Ошибка создания папки: {}", e));
                                }
                                *is_processing.lock().unwrap() = false;
                                return;
                            }
                            match downloader.download_all(&logs, &total_files, &processed_files) {
                                Ok(files) => {
                                    {
                                        let mut logs = logs.lock().unwrap();
                                        logs.push(format!("Скачано файлов: {}", files.len()));
                                    }
                                    files
                                }
                                Err(e) => {
                                    {
                                        let mut logs = logs.lock().unwrap();
                                        logs.push(format!("Ошибка загрузки: {}", e));
                                    }
                                    *is_processing.lock().unwrap() = false;
                                    return;
                                }
                            }
                        };

                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push("Инициализация WbUploader".to_string());
                        }
                        let uploader = match WbUploader::new(api_key) {
                            Ok(u) => u,
                            Err(e) => {
                                {
                                    let mut logs = logs.lock().unwrap();
                                    logs.push(format!("Ошибка инициализации WB: {}", e));
                                }
                                *is_processing.lock().unwrap() = false;
                                return;
                            }
                        };

                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push("Начало обработки vendor codes".to_string());
                        }
                        for vendor_code in vendor_codes {
                            {
                                let mut logs = logs.lock().unwrap();
                                logs.push(format!("Обработка vendorCode: {}", vendor_code));
                            }
                            match uploader.get_nm_id_by_vendor_code(&vendor_code, &logs) {
                                Ok(nm_id) => {
                                    let relevant_files: Vec<FileInfo> = files.iter()
                                        .filter(|f| f.articul == vendor_code)
                                        .cloned()
                                        .collect();
                                    if relevant_files.is_empty() {
                                        {
                                            let mut logs = logs.lock().unwrap();
                                            logs.push(format!("Ошибка: не найдено файлов для vendorCode: {}", vendor_code));
                                        }
                                        let mut failed_vendor_codes = failed_vendor_codes.lock().unwrap();
                                        failed_vendor_codes.push(vendor_code.clone());
                                        continue;
                                    }
                                    let downloader = match Downloader::new(public_keys_for_thread.clone(), download_path.clone(), vec![vendor_code.clone()]) {
                                        Ok(d) => d,
                                        Err(e) => {
                                            {
                                                let mut logs = logs.lock().unwrap();
                                                logs.push(format!("Ошибка инициализации Downloader для публикации: {}", e));
                                            }
                                            let mut failed_vendor_codes = failed_vendor_codes.lock().unwrap();
                                            failed_vendor_codes.push(vendor_code.clone());
                                            continue;
                                        }
                                    };
                                    match downloader.generate_media_json(nm_id, &relevant_files, &logs, None) {
                                        Ok(media) => {
                                            let json_output = serde_json::to_string_pretty(&media).unwrap_or_else(|e| format!("Ошибка сериализации JSON: {}", e));
                                            {
                                                let mut logs = logs.lock().unwrap();
                                                logs.push(format!("JSON Output для nmId {}:\n{}", nm_id, json_output));
                                            }
                                            if let Err(e) = uploader.upload_links(nm_id, &media.data, &logs, &processed_files) {
                                                {
                                                    let mut logs = logs.lock().unwrap();
                                                    logs.push(format!("Ошибка загрузки ссылок на WB для nmId {}: {}", nm_id, e));
                                                }
                                                let mut failed_vendor_codes = failed_vendor_codes.lock().unwrap();
                                                failed_vendor_codes.push(vendor_code.clone());
                                            } else {
                                                {
                                                    let mut logs = logs.lock().unwrap();
                                                    logs.push(format!("Ссылки для nmId {} загружены успешно", nm_id));
                                                }
                                                if use_local_path {
                                                    for file in relevant_files.iter() {
                                                        let disk_path = format!("/wb_upload/{}", file.name);
                                                        {
                                                            let mut logs = logs.lock().unwrap();
                                                            logs.push(format!("Очистка файла {} с Яндекс.Диска", disk_path));
                                                        }
                                                        if let Err(e) = downloader.cleanup_yandex_disk(&disk_path, &logs) {
                                                            {
                                                                let mut logs = logs.lock().unwrap();
                                                                logs.push(format!("Ошибка удаления {} с Яндекс.Диска: {}", disk_path, e));
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    for file in relevant_files.iter() {
                                                        if let Err(e) = downloader.cleanup_file(&file.local_path, &logs) {
                                                            {
                                                                let mut logs = logs.lock().unwrap();
                                                                logs.push(format!("Ошибка удаления файла {}: {}", file.local_path, e));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            {
                                                let mut logs = logs.lock().unwrap();
                                                logs.push(format!("Ошибка генерации JSON для nmId {}: {}", nm_id, e));
                                            }
                                            let mut failed_vendor_codes = failed_vendor_codes.lock().unwrap();
                                            failed_vendor_codes.push(vendor_code.clone());
                                        }
                                    }
                                }
                                Err(e) => {
                                    {
                                        let mut logs = logs.lock().unwrap();
                                        logs.push(format!("Ошибка получения nmId для vendorCode {}: {}", vendor_code, e));
                                    }
                                    let mut failed_vendor_codes = failed_vendor_codes.lock().unwrap();
                                    failed_vendor_codes.push(vendor_code.clone());
                                }
                            }
                            {
                                let mut processed = processed_files.lock().unwrap();
                                *processed += 1;
                            }
                        }

                        {
                            let mut logs = logs.lock().unwrap();
                            let failed = failed_vendor_codes.lock().unwrap();
                            if !failed.is_empty() {
                                logs.push(format!("Ошибочные vendor codes для повторного запуска: {}", failed.join(", ")));
                                let mut file_names = file_names.lock().unwrap();
                                *file_names = failed.join("\n");
                            } else {
                                logs.push("Все vendor codes обработаны успешно.".to_string());
                                let mut file_names = file_names.lock().unwrap();
                                *file_names = String::new();
                            }
                        }

                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push("Процесс завершен.".to_string());
                        }
                        *is_processing.lock().unwrap() = false;
                    });
                }
            });

            if is_processing {
                let processed = *self.processed_files.lock().unwrap();
                let total = *self.total_files.lock().unwrap();
                if let Some(total) = total {
                    let progress = processed as f32 / total as f32;
                    ui.add_space(20.0);
                    ui.add(egui::ProgressBar::new(progress)
                        .text(format!("Обработано {} из {}", processed, total))
                        .desired_width(300.0));
                }
            }

            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if ui.button("📜 Показать логи").clicked() {
                    self.show_logs = true;
                }
                let failed = self.failed_vendor_codes.lock().unwrap();
                ui.add_enabled_ui(!failed.is_empty() && !is_processing, |ui| {
                    if ui.button("🔄 Повторить для ошибочных").clicked() {
                        self.file_names = failed.join("\n");
                        {
                            let mut logs = self.logs.lock().unwrap();
                            logs.push(format!("Повторная обработка vendor codes: {}", failed.join(", ")));
                        }
                    }
                });
            });

            ctx.request_repaint();
        });
    }
}

fn text_edit_with_context_menu(ui: &mut egui::Ui, text: &mut String, width: f32, hint_text: &str) {
    let text_edit = egui::TextEdit::multiline(text)
        .desired_width(width)
        .hint_text(hint_text);
    let response = ui.add(text_edit);
    response.context_menu(|ui| {
        if ui.button("📋 Вставить").clicked() {
            if let Ok(mut clipboard) = Clipboard::new() {
                if let Ok(clipboard_text) = clipboard.get_text() {
                    *text = clipboard_text;
                }
            }
            ui.close_menu();
        }
        if ui.button("🗑 Очистить").clicked() {
            text.clear();
            ui.close_menu();
        }
    });
}
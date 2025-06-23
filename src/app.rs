use crate::config::Config;
use crate::downloader::{Downloader, FileInfo};
use crate::profile::{Profile, ProfileManager};
use crate::uploader::WbUploader;
use arboard::Clipboard;
use eframe::App;
use eframe::egui;
use rfd::FileDialog;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct DownloaderApp {
    urls: String,
    file_names: String,
    profile_manager: ProfileManager,
    new_profile_name: String,
    is_processing: Arc<Mutex<bool>>,
    total_files: Arc<Mutex<Option<usize>>>,
    processed_files: Arc<Mutex<usize>>,
    use_local_path: bool,
    local_source_path: String,
    failed_vendor_codes: Arc<Mutex<Vec<String>>>,
}

impl Default for DownloaderApp {
    fn default() -> Self {
        log::info!("Создание default DownloaderApp");
        Self {
            urls: String::new(),
            file_names: String::new(),
            profile_manager: ProfileManager::new().unwrap_or_else(|e| {
                log::error!("Ошибка создания ProfileManager: {}", e);
                ProfileManager {
                    profiles: vec![Profile {
                        name: "Добавить".to_string(),
                        api_key: String::new(),
                    }],
                    selected_index: 0,
                    config: Config::new().unwrap(),
                }
            }),
            new_profile_name: String::new(),
            is_processing: Arc::new(Mutex::new(false)),
            total_files: Arc::new(Mutex::new(None)),
            processed_files: Arc::new(Mutex::new(0)),
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
                            log::error!("Ошибка сохранения профилей: {}", e);
                        }
                    }
                    if ui.button("🗑 Удалить").clicked() && self.profile_manager.profiles.len() > 1 {
                        self.profile_manager.delete_profile(self.profile_manager.selected_index);
                        if let Err(e) = self.profile_manager.save() {
                            log::error!("Ошибка сохранения профилей после удаления: {}", e);
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
                            log::error!("API ключ не может быть пустым");
                        } else {
                            match self.profile_manager.save() {
                                Ok(()) => log::info!("API ключ успешно сохранен"),
                                Err(e) => log::error!("Ошибка сохранения API ключа: {}", e),
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
                        text_edit_with_context_menu(ui, &mut self.urls, 400.0, "https://disk.yandex.ru/d/link1,https://disk.yandex.ru/d/link2,etc");
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
                ui.label(egui::RichText::new("📋 Vendor Codes").strong().size(22.0));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🔢 Список vendor codes (по одному на строке):").strong());
                    ui.vertical(|ui| {
                        egui::ScrollArea::vertical()
                            .max_height(100.0)
                            .show(ui, |ui| {
                                text_edit_with_context_menu(ui, &mut self.file_names, 400.0, "VendorCode001\nVendorCode001\nEtc");
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
                    let local_source_path = self.local_source_path.clone();
                    let vendor_codes: Vec<String> = self.file_names.trim()
                        .lines()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let api_key = self.profile_manager.current_profile().api_key.clone();
                    if vendor_codes.is_empty() || (self.use_local_path && local_source_path.is_empty()) || (!self.use_local_path && urls.is_empty()) {
                        log::error!("Заполните все поля");
                        return;
                    }
                    if !self.use_local_path && !urls.split(',').all(|s| s.trim().contains("disk.yandex.ru/d/")) {
                        log::error!("Все ссылки должны быть на Яндекс.Диск");
                        return;
                    }
                    if self.use_local_path && !Path::new(&local_source_path).is_absolute() {
                        log::error!("Локальный путь должен быть абсолютным");
                        return;
                    }
                    if api_key.is_empty() {
                        log::error!("API ключ не указан");
                        return;
                    }

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

                    log::info!("Начало обработки...");
                    *is_processing.lock().unwrap() = true;
                    *processed_files.lock().unwrap() = 0;
                    *total_files.lock().unwrap() = Some(vendor_codes.len());
                    failed_vendor_codes.lock().unwrap().clear();

                    let public_keys_for_thread = public_keys.clone();
                    std::thread::spawn(move || {
                        log::info!("Запущен фоновый поток");

                        let files = if use_local_path {
                            log::info!("Инициализация Downloader для локального режима");
                            let downloader = match Downloader::new(Vec::new(), vendor_codes.clone()) {
                                Ok(d) => d,
                                Err(e) => {
                                    log::error!("Ошибка инициализации: {}", e);
                                    *is_processing.lock().unwrap() = false;
                                    return;
                                }
                            };
                            log::info!("Начало сканирования локальной папки: {}", local_source_path);
                            match downloader.find_local_files(&local_source_path) {
                                Ok(files) => {
                                    log::info!("Найдено файлов: {}", files.len());
                                    files
                                }
                                Err(e) => {
                                    log::error!("Ошибка сканирования локальной папки: {}", e);
                                    *is_processing.lock().unwrap() = false;
                                    return;
                                }
                            }
                        } else {
                            log::info!("Инициализация Downloader для Яндекс.Диска");
                            let downloader = match Downloader::new(public_keys_for_thread.clone(), vendor_codes.clone()) {
                                Ok(d) => d,
                                Err(e) => {
                                    log::error!("Ошибка инициализации: {}", e);
                                    *is_processing.lock().unwrap() = false;
                                    return;
                                }
                            };
                            log::info!("Начало поиска файлов с URL: {:?}", public_keys_for_thread);
                            match downloader.find_files("/") {
                                Ok(files) => {
                                    log::info!("Найдено файлов: {}", files.len());
                                    files
                                }
                                Err(e) => {
                                    log::error!("Ошибка поиска файлов: {}", e);
                                    *is_processing.lock().unwrap() = false;
                                    return;
                                }
                            }
                        };

                        log::info!("Инициализация WbUploader");
                        let uploader = match WbUploader::new(api_key) {
                            Ok(u) => u,
                            Err(e) => {
                                log::error!("Ошибка инициализации WB: {}", e);
                                *is_processing.lock().unwrap() = false;
                                return;
                            }
                        };

                        log::info!("Начало обработки vendor codes");
                        for vendor_code in vendor_codes {
                            log::info!("Обработка vendorCode: {}", vendor_code);
                            match uploader.get_nm_id_by_vendor_code(&vendor_code) {
                                Ok(nm_id) => {
                                    let relevant_files: Vec<FileInfo> = files.iter()
                                        .filter(|f| f.articul == vendor_code)
                                        .cloned()
                                        .collect();
                                    if relevant_files.is_empty() {
                                        log::error!("Не найдено файлов для vendorCode: {}", vendor_code);
                                        let mut failed_vendor_codes = failed_vendor_codes.lock().unwrap();
                                        failed_vendor_codes.push(vendor_code.clone());
                                        continue;
                                    }
                                    let downloader = match Downloader::new(public_keys_for_thread.clone(), vec![vendor_code.clone()]) {
                                        Ok(d) => d,
                                        Err(e) => {
                                            log::error!("Ошибка инициализации Downloader для публикации: {}", e);
                                            let mut failed_vendor_codes = failed_vendor_codes.lock().unwrap();
                                            failed_vendor_codes.push(vendor_code.clone());
                                            continue;
                                        }
                                    };
                                    match downloader.generate_media_json(nm_id, &relevant_files, None) {
                                        Ok(media) => {
                                            let json_output = serde_json::to_string_pretty(&media).unwrap_or_else(|e| format!("Ошибка сериализации JSON: {}", e));
                                            log::info!("JSON Output для nmId {}:\n{}", nm_id, json_output);
                                            if let Err(e) = uploader.upload_links(nm_id, &media.data, &processed_files) {
                                                log::error!("Ошибка загрузки ссылок на WB для nmId {}: {}", nm_id, e);
                                                let mut failed_vendor_codes = failed_vendor_codes.lock().unwrap();
                                                failed_vendor_codes.push(vendor_code.clone());
                                            } else {
                                                log::info!("Ссылки для nmId {} загружены успешно", nm_id);
                                            }
                                        }
                                        Err(e) => {
                                            log::error!("Ошибка генерации JSON для nmId {}: {}", nm_id, e);
                                            let mut failed_vendor_codes = failed_vendor_codes.lock().unwrap();
                                            failed_vendor_codes.push(vendor_code.clone());
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("Ошибка получения nmId для vendorCode {}: {}", vendor_code, e);
                                    let mut failed_vendor_codes = failed_vendor_codes.lock().unwrap();
                                    failed_vendor_codes.push(vendor_code.clone());
                                }
                            }
                            {
                                let mut processed = processed_files.lock().unwrap();
                                *processed += 1;
                            }
                        }

                        let failed = failed_vendor_codes.lock().unwrap();
                        if !failed.is_empty() {
                            log::warn!("Ошибочные vendor codes для повторного запуска: {}", failed.join(", "));
                            let mut file_names = file_names.lock().unwrap();
                            *file_names = failed.join("\n");
                        } else {
                            log::info!("Все vendor codes обработаны успешно.");
                            let mut file_names = file_names.lock().unwrap();
                            *file_names = String::new();
                        }

                        log::info!("Процесс завершен.");
                        *is_processing.lock().unwrap() = false;
                    });
                }
            });

            ui.add_space(20.0);
            ui.horizontal(|ui| {
                let failed = self.failed_vendor_codes.lock().unwrap();
                ui.add_enabled_ui(!failed.is_empty() && !is_processing, |ui| {
                    if ui.button("🔄 Повторить для ошибочных").clicked() {
                        self.file_names = failed.join("\n");
                        log::info!("Повторная обработка vendor codes: {}", failed.join(", "));
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

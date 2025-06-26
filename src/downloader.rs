use crate::utils::is_media_file;
use anyhow::{Context, Result};
use regex::Regex;
use reqwest::blocking::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;
use urlencoding::encode;
use walkdir::WalkDir;

#[derive(Deserialize)]
struct ResourceList {
    _embedded: Embedded,
}

#[derive(Deserialize)]
struct Embedded {
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    name: String,
    #[serde(rename = "type")]
    item_type: String,
}

#[derive(Deserialize)]
struct DownloadLink {
    href: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub articul: String,
    pub photo_number: u32,
}

#[derive(Serialize, Deserialize)]
pub struct MediaOutput {
    pub nm_id: i64,
    pub data: Vec<String>,
}

pub struct Downloader {
    client: Client,
    public_keys: Vec<String>,
    pub(crate) prefixes: Vec<String>,
}

impl Downloader {
    pub fn new(public_keys: Vec<String>, prefixes: Vec<String>) -> Result<Self> {
        log::info!(
            "Инициализация Downloader с {} ключами и префиксами {:?}",
            public_keys.len(),
            prefixes
        );
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(5))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    "User-Agent",
                    reqwest::header::HeaderValue::from_static("Mozilla/5.0"),
                );
                headers.insert("Accept", reqwest::header::HeaderValue::from_static("*/*"));
                headers
            })
            .build()
            .map_err(|e| anyhow::anyhow!("Не удалось создать HTTP-клиент: {}", e))?;
        Ok(Self {
            client,
            public_keys,
            prefixes,
        })
    }

    pub fn find_files(&self, path: &str) -> Result<Vec<FileInfo>> {
        let mut files: Vec<FileInfo> = Vec::new();
        let mut found_prefixes: HashSet<String> = HashSet::new();
        let target_prefixes: HashSet<String> = self.prefixes.iter().cloned().collect();

        for public_key in &self.public_keys {
            log::info!(
                "Сканирование директории на Яндекс.Диске: {} для URL: {}",
                path,
                public_key
            );
            let result =
                self.find_files_for_url(public_key, path, &mut found_prefixes, &target_prefixes)?;
            files.extend(result);

            if target_prefixes.is_subset(&found_prefixes) {
                log::info!("Все указанные vendorCode найдены: {:?}", target_prefixes);
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }

        if files.is_empty() {
            log::warn!(
                "Не найдено файлов с префиксами в {}: {:?}",
                path,
                self.prefixes
            );
        } else {
            log::info!("Найдено {} файлов в {}", files.len(), path);
        }
        Ok(files)
    }

    fn find_files_for_url(
        &self,
        public_key: &str,
        path: &str,
        found_prefixes: &mut HashSet<String>,
        target_prefixes: &HashSet<String>,
    ) -> Result<Vec<FileInfo>> {
        let mut files: Vec<FileInfo> = Vec::new();
        let mut subdirs: Vec<String> = Vec::new();
        let mut offset = 0;
        let limit = 100;

        loop {
            let url = format!(
                "https://cloud-api.yandex.net/v1/disk/public/resources?public_key={}&path={}&fields=_embedded.items,name,type&limit={}&offset={}",
                encode(public_key),
                encode(path),
                limit,
                offset
            );
            log::debug!("HTTP Request: GET {}", url);

            let mut attempts = 0;
            let max_attempts = 3;
            let response = loop {
                log::debug!(
                    "Отправка HTTP-запроса к Яндекс.Диске (попытка {}/{}, offset={})",
                    attempts + 1,
                    max_attempts,
                    offset
                );
                match self.client.get(&url).send() {
                    Ok(response) => break response,
                    Err(e) => {
                        log::error!(
                            "Ошибка HTTP запроса для {} (offset={}): {}",
                            path,
                            offset,
                            e
                        );
                        attempts += 1;
                        if attempts >= max_attempts {
                            log::error!(
                                "Не удалось получить ответ для {} (offset={}) после {} попыток",
                                path,
                                offset,
                                max_attempts
                            );
                            return Err(anyhow::anyhow!(
                                "Не удалось получить ответ для {} после {} попыток",
                                path,
                                max_attempts
                            ));
                        }
                        std::thread::sleep(Duration::from_secs(5));
                    }
                }
            };

            log::debug!(
                "Ответ от API Яндекс.Диска получен для {} (offset={})",
                path,
                offset
            );
            let status = response.status();
            let body = response
                .text()
                .map_err(|e| anyhow::anyhow!("Не удалось прочитать ответ для {}: {}", path, e))?;
            log::trace!(
                "HTTP Response: Status: {}, Body (preview): {}",
                status,
                body.chars().take(200).collect::<String>()
            );

            if !status.is_success() {
                log::error!(
                    "Ошибка API Яндекс.Диска для {} (offset={}): Статус {}, Тело: {}",
                    path,
                    offset,
                    status,
                    body
                );
                return Err(anyhow::anyhow!(
                    "Ошибка API Яндекс.Диска: Статус {}, Тело: {}",
                    status,
                    body
                ));
            }

            log::debug!("Парсинг JSON-ответа для {} (offset={})", path, offset);
            let resource_list: ResourceList = serde_json::from_str(&body).context(format!(
                "Ошибка парсинга ответа Яндекс.Диска для {} (offset={})",
                path, offset
            ))?;
            log::debug!(
                "JSON-ответ успешно распарсен для {} (offset={})",
                path,
                offset
            );

            let items = resource_list._embedded.items;
            if items.is_empty() {
                log::debug!("Нет элементов для {} на offset={}", path, offset);
                break;
            }

            for item in &items {
                let item_path = if path == "/" {
                    format!("/{}", item.name)
                } else {
                    format!("{}/{}", path, item.name)
                };
                if item.item_type == "file" && is_media_file(&item.name) {
                    let base_name = item.name.to_lowercase();
                    let matched_prefix = self
                        .prefixes
                        .iter()
                        .filter(|p| base_name.starts_with(&p.to_lowercase()))
                        .max_by_key(|p| p.len());
                    if let Some(prefix) = matched_prefix {
                        let articul = prefix.to_string();
                        found_prefixes.insert(articul.clone());
                        let remaining = &base_name[prefix.len()..];
                        let photo_number = if let Some(caps) =
                            Regex::new(r"^[_-](\d+)\.\w+$")?.captures(remaining)
                        {
                            caps.get(1).unwrap().as_str().parse::<u32>().unwrap_or(1)
                        } else if remaining.starts_with('.') {
                            1
                        } else {
                            log::warn!(
                                "Файл {} содержит vendorCode {}, но не соответствует шаблону",
                                item.name,
                                prefix
                            );
                            continue;
                        };
                        files.push(FileInfo {
                            name: item.name.clone(),
                            path: item_path,
                            articul: articul.clone(),
                            photo_number,
                        });
                        log::info!(
                            "Найден файл: {} (vendorCode: {}, фото: {})",
                            item.name,
                            articul,
                            photo_number
                        );
                    } else {
                        log::debug!(
                            "Файл {} не начинается ни с одного vendorCode: {:?}",
                            item.name,
                            self.prefixes
                        );
                    }
                } else if item.item_type == "dir" {
                    subdirs.push(item_path);
                }
            }

            offset += limit;
            log::debug!(
                "Обработано {} элементов для {}, переходим к следующей странице (offset={})",
                items.len(),
                path,
                offset
            );
            std::thread::sleep(Duration::from_millis(500));

            if target_prefixes.is_subset(found_prefixes) {
                log::info!(
                    "Все указанные vendorCode найдены в {}: {:?}",
                    path,
                    found_prefixes
                );
                break;
            }
        }

        for subdir in subdirs {
            log::info!("Переход к поддиректории: {}", subdir);
            match self.find_files_for_url(public_key, &subdir, found_prefixes, target_prefixes) {
                Ok(new_files) => {
                    files.extend(new_files);
                    log::info!("Завершено сканирование поддиректории: {}", subdir);
                }
                Err(e) => {
                    log::error!("Ошибка сканирования поддиректории {}: {}", subdir, e);
                }
            }
            if target_prefixes.is_subset(found_prefixes) {
                log::info!("Все указанные vendorCode найдены: {:?}", found_prefixes);
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }

        Ok(files)
    }

    pub fn find_local_files(&self, source_path: &str) -> Result<Vec<FileInfo>> {
        log::info!("Поиск локальных файлов в: {}", source_path);
        let mut files = Vec::new();
        let source_path = Path::new(source_path);

        if !source_path.is_dir() {
            log::error!("Ошибка: {} не является директорией", source_path.display());
            return Err(anyhow::anyhow!(
                "Папка {} не является директорией",
                source_path.display()
            ));
        }

        for entry in WalkDir::new(source_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if path.is_file() && is_media_file(&name) {
                let base_name = name.to_lowercase();
                if let Some(prefix) = self
                    .prefixes
                    .iter()
                    .find(|p| base_name.starts_with(&p.to_lowercase()))
                {
                    let articul = prefix.to_string();
                    let remaining = &base_name[prefix.len()..];
                    let photo_number =
                        if let Some(caps) = Regex::new(r"^[_-](\d+)\.\w+$")?.captures(remaining) {
                            caps[1].parse::<u32>().unwrap_or(1)
                        } else if remaining.starts_with('.') {
                            1
                        } else {
                            log::warn!(
                                "Файл {} содержит vendorCode {}, но не соответствует шаблону",
                                name,
                                prefix
                            );
                            continue;
                        };
                    files.push(FileInfo {
                        name: name.clone(),
                        path: path.to_string_lossy().to_string(),
                        articul: articul.clone(),
                        photo_number,
                    });
                    log::info!(
                        "Найден локальный файл: {} (vendorCode: {}, фото: {})",
                        name,
                        articul,
                        photo_number
                    );
                } else {
                    log::debug!(
                        "Файл {} не начинается ни с одного vendorCode: {:?}",
                        name,
                        self.prefixes
                    );
                }
            }
        }
        log::info!("Найдено {} локальных файлов", files.len());
        Ok(files)
    }

    pub fn get_download_url(&self, file_path: &str) -> Result<String> {
        for public_key in &self.public_keys {
            log::info!("Получение ссылки для: {} с URL: {}", file_path, public_key);
            let url = format!(
                "https://cloud-api.yandex.net/v1/disk/public/resources/download?public_key={}&path={}",
                encode(public_key),
                encode(file_path)
            );
            log::debug!("HTTP Request: GET {}", url);

            let mut attempts = 0;
            let max_attempts = 3;
            loop {
                match self.client.get(&url).send() {
                    Ok(response) => {
                        let status = response.status();
                        let body = response.text().map_err(|e| {
                            anyhow::anyhow!("Не удалось прочитать ответ для {}: {}", file_path, e)
                        })?;
                        log::debug!("HTTP Response: Status: {}, Body: {}", status, body);
                        if status.is_success() {
                            let download_link: DownloadLink =
                                serde_json::from_str(&body).map_err(|e| {
                                    anyhow::anyhow!(
                                        "Ошибка парсинга ссылки для {}: {}",
                                        file_path,
                                        e
                                    )
                                })?;
                            return Ok(download_link.href);
                        } else {
                            log::warn!("Ошибка получения ссылки для {}: {}", file_path, body);
                            if status.as_u16() == 401 {
                                log::info!("Пропуск URL {} из-за ошибки 401", public_key);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Ошибка HTTP запроса для {}: {}", file_path, e);
                    }
                }
                attempts += 1;
                if attempts >= max_attempts {
                    log::error!(
                        "Не удалось получить ссылку для {} после {} попыток",
                        file_path,
                        max_attempts
                    );
                    break;
                }
                log::debug!(
                    "Повторная попытка через 5 секунд ({}/{})",
                    attempts,
                    max_attempts
                );
                std::thread::sleep(Duration::from_secs(5));
            }
        }
        Err(anyhow::anyhow!(
            "Не удалось получить ссылку для {} ни с одного URL",
            file_path
        ))
    }

    #[allow(dead_code)]
    pub fn download_all(&self) -> Result<Vec<FileInfo>> {
        log::info!("Начало поиска всех файлов");
        let files = self.find_files("/")?;
        if files.is_empty() {
            log::warn!("Не найдено файлов с префиксами: {:?}", self.prefixes);
        }
        Ok(files)
    }

    pub fn generate_media_json(
        &self,
        nm_id: i64,
        files: &[FileInfo],
        _server_port: Option<u16>,
    ) -> Result<MediaOutput> {
        log::info!("Генерация JSON для nmId: {}", nm_id);
        let mut urls = vec![];
        for file in files {
            log::debug!("Обработка файла {} для nmId {}", file.name, nm_id);
            if !self.public_keys.is_empty() {
                match self.get_download_url(&file.path) {
                    Ok(download_url) => {
                        urls.push(download_url.clone());
                        log::info!("Добавлена URL диска для {}: {}", file.name, download_url);
                    }
                    Err(e) => {
                        log::error!("Ошибка получения ссылки для {}: {}", file.name, e);
                        return Err(e);
                    }
                }
            } else {
                urls.push(format!("file://{}", file.path));
                log::info!(
                    "Добавлен локальный путь для {}: file://{}",
                    file.name,
                    file.path
                );
            }
        }
        if urls.is_empty() {
            log::error!("Не найдено файлов для nmId {}", nm_id);
            return Err(anyhow::anyhow!("Не найдено файлов для nmId: {}", nm_id));
        }
        log::info!(
            "Сгенерировано {} URLs для nmId {}: {:?}",
            urls.len(),
            nm_id,
            urls
        );
        Ok(MediaOutput { nm_id, data: urls })
    }

    #[allow(dead_code)]
    pub fn cleanup_file(&self, file_path: &str) -> Result<()> {
        if file_path.starts_with("file://") {
            let local_path = file_path.strip_prefix("file://").unwrap_or(file_path);
            log::info!("Удаление локального файла: {}", local_path);
            // Uncomment the following lines if local file deletion is desired
            // std::fs::remove_file(local_path).map_err(|e| {
            //     anyhow::anyhow!("Не удалось удалить файл {}: {}", local_path, e)
            // })?;
            log::info!("Удаление локального файла {} пока не реализовано", local_path);
        } else {
            log::info!("Файлы, полученные по URL ({}), не удаляются", file_path);
        }
        Ok(())
    }
}

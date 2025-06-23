use anyhow::{Result, Context};
use reqwest::blocking::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use urlencoding::encode;
use regex::Regex;
use std::collections::HashSet;
use crate::utils::is_media_file;
use crate::config::Config;

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

#[derive(Deserialize)]
struct UploadLink {
    href: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub local_path: String,
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
    download_path: String,
    prefixes: Vec<String>,
    config: Config,
}

impl Downloader {
    pub fn new(public_keys: Vec<String>, download_path: String, prefixes: Vec<String>) -> Result<Self> {
        let config = Config::new()?;
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(5))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert("User-Agent", reqwest::header::HeaderValue::from_static("Mozilla/5.0"));
                headers.insert("Accept", reqwest::header::HeaderValue::from_static("*/*"));
                headers
            })
            .build()?;
        Ok(Self {
            client,
            public_keys,
            download_path,
            prefixes,
            config,
        })
    }

    pub fn upload_to_yandex_disk(&self, local_path: &str, _disk_path: &str, logs: &Arc<Mutex<Vec<String>>>) -> Result<String> {
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Ошибка: Загрузка на Яндекс.Диск не поддерживается без OAuth токена для {}", local_path));
        }
        Err(anyhow::anyhow!("Загрузка на Яндекс.Диск не поддерживается без OAuth токена"))
    }

    pub fn find_files(&self, path: &str, logs: &Arc<Mutex<Vec<String>>>) -> Result<Vec<FileInfo>> {
        let mut files: Vec<FileInfo> = Vec::new();
        let mut found_prefixes: HashSet<String> = HashSet::new();
        let target_prefixes: HashSet<String> = self.prefixes.iter().cloned().collect();

        for public_key in &self.public_keys {
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Сканирование директории на Яндекс.Диске: {} для URL: {}", path, public_key));
            }
            let result = self.find_files_for_url(public_key, path, logs, &mut found_prefixes, &target_prefixes)?;
            files.extend(result);

            if target_prefixes.is_subset(&found_prefixes) {
                {
                    let mut logs = logs.lock().unwrap();
                    logs.push(format!("Все указанные vendorCode найдены: {:?}", target_prefixes));
                }
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }

        if files.is_empty() {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Не найдено файлов с префиксами в {}: {:?}", path, self.prefixes));
        } else {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Найдено {} файлов в {}", files.len(), path));
        }
        Ok(files)
    }

    fn find_files_for_url(
        &self,
        public_key: &str,
        path: &str,
        logs: &Arc<Mutex<Vec<String>>>,
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
                encode(public_key), encode(path), limit, offset
            );
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!(
                    "HTTP Request: GET {}\nHeaders: User-Agent: Mozilla/5.0; Accept: */*",
                    url
                ));
            }

            let mut attempts = 0;
            let max_attempts = 3;
            let response = loop {
                {
                    let mut logs = logs.lock().unwrap();
                    logs.push(format!("Отправка HTTP-запроса к Яндекс.Диску (попытка {}/{}, offset={})...", attempts + 1, max_attempts, offset));
                }
                match self.client.get(&url).send() {
                    Ok(response) => break response,
                    Err(e) => {
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!("Ошибка HTTP запроса для {} (offset={}): {}", path, offset, e));
                        }
                        attempts += 1;
                        if attempts >= max_attempts {
                            {
                                let mut logs = logs.lock().unwrap();
                                logs.push(format!("Не удалось получить ответ для {} (offset={}) после {} попыток", path, offset, max_attempts));
                            }
                            return Err(anyhow::anyhow!("Не удалось получить ответ для {} после {} попыток", path, max_attempts));
                        }
                        std::thread::sleep(Duration::from_secs(5));
                    }
                }
            };

            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Ответ от API Яндекс.Диска получен для {} (offset={})", path, offset));
            }

            let status = response.status();
            let body = response.text()?;
            let body_preview = body.chars().take(200).collect::<String>();
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!(
                    "HTTP Response: Status: {}, Body (preview): {}",
                    status, body_preview
                ));
            }

            if !status.is_success() {
                return Err(anyhow::anyhow!("Ошибка API Яндекс.Диска для {} (offset={}): Статус {}, Тело: {}", path, offset, status, body));
            }

            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Парсинг JSON-ответа для {} (offset={})...", path, offset));
            }
            let resource_list: ResourceList = serde_json::from_str(&body)
                .context(format!("Ошибка парсинга ответа Яндекс.Диска для {} (offset={}): {}", path, offset, body))?;
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("JSON-ответ успешно распарсен для {} (offset={})", path, offset));
            }

            let items = resource_list._embedded.items;
            if items.is_empty() {
                {
                    let mut logs = logs.lock().unwrap();
                    logs.push(format!("Нет элементов для {} на offset={}", path, offset));
                }
                break;
            }

            for item in &items {
                let item_path = if path == "/" { format!("/{}", item.name) } else { format!("{}/{}", path, item.name) };
                if item.item_type == "file" && is_media_file(&item.name) {
                    let base_name = item.name.to_lowercase();
                    let matched_prefix = self.prefixes.iter()
                        .filter(|p| base_name.starts_with(&p.to_lowercase()))
                        .max_by_key(|p| p.len());
                    if let Some(prefix) = matched_prefix {
                        let articul = prefix.to_string();
                        found_prefixes.insert(articul.clone());
                        let remaining = &base_name[prefix.len()..];
                        let photo_number = if let Some(caps) = Regex::new(r"^[_-](\d+)\.\w+$")?.captures(remaining) {
                            caps.get(1).unwrap().as_str().parse::<u32>().unwrap_or(1)
                        } else if remaining.starts_with('.') {
                            1
                        } else {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!(
                                "Файл {} содержит vendorCode {}, но не соответствует шаблону",
                                item.name, prefix
                            ));
                            continue;
                        };
                        let local_path = Path::new(&self.download_path).join(&item.name).to_string_lossy().to_string();
                        files.push(FileInfo {
                            name: item.name.clone(),
                            path: item_path,
                            local_path,
                            articul: articul.clone(),
                            photo_number,
                        });
                        let mut logs = logs.lock().unwrap();
                        logs.push(format!(
                            "Найден файл: {} (vendorCode: {}, фото: {})",
                            item.name, articul, photo_number
                        ));
                    } else {
                        let mut logs = logs.lock().unwrap();
                        logs.push(format!(
                            "Файл {} не начинается ни с одного vendorCode: {:?}",
                            item.name, self.prefixes
                        ));
                    }
                } else if item.item_type == "dir" {
                    subdirs.push(item_path);
                }
            }

            offset += limit;
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Обработано {} элементов для {}, переходим к следующей странице (offset={})", items.len(), path, offset));
            }
            std::thread::sleep(Duration::from_millis(500));

            if target_prefixes.is_subset(found_prefixes) {
                {
                    let mut logs = logs.lock().unwrap();
                    logs.push(format!("Все указанные vendorCode найдены в {}: {:?}", path, found_prefixes));
                }
                break;
            }
        }

        for subdir in subdirs {
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Переход к поддиректории: {}", subdir));
            }
            match self.find_files_for_url(public_key, &subdir, logs, found_prefixes, target_prefixes) {
                Ok(new_files) => {
                    files.extend(new_files);
                    {
                        let mut logs = logs.lock().unwrap();
                        logs.push(format!("Завершено сканирование поддиректории: {}", subdir));
                    }
                }
                Err(e) => {
                    {
                        let mut logs = logs.lock().unwrap();
                        logs.push(format!("Ошибка сканирования поддиректории {}: {}", subdir, e));
                    }
                }
            }
            if target_prefixes.is_subset(found_prefixes) {
                {
                    let mut logs = logs.lock().unwrap();
                    logs.push(format!("Все указанные vendorCode найдены: {:?}", found_prefixes));
                }
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }

        Ok(files)
    }

    pub fn find_local_files(&self, source_path: &str, logs: &Arc<Mutex<Vec<String>>>) -> Result<Vec<FileInfo>> {
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Поиск локальных файлов в: {}", source_path));
            logs.push(format!("Ожидаемые vendorCode: {:?}", self.prefixes));
        }
        let mut files: Vec<FileInfo> = Vec::new();
        let source_path = Path::new(source_path);

        if !source_path.exists() || !source_path.is_dir() {
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Ошибка: Папка {} не существует или не является директорией", source_path.display()));
            }
            return Err(anyhow::anyhow!("Папка {} не существует или не является директорией", source_path.display()));
        }

        fn scan_dir(
            dir: &Path,
            base_path: &Path,
            prefixes: &[String],
            files: &mut Vec<FileInfo>,
            logs: &Arc<Mutex<Vec<String>>>,
            download_path: &str,
        ) -> Result<()> {
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Попытка открыть директорию: {}", dir.display()));
            }
            let dir_entries = match fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(e) => {
                    {
                        let mut logs = logs.lock().unwrap();
                        logs.push(format!("Ошибка чтения директории {}: {}", dir.display(), e));
                    }
                    return Err(anyhow::anyhow!("Ошибка чтения директории {}: {}", dir.display(), e));
                }
            };
            for entry in dir_entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(e) => {
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!("Ошибка обработки записи в {}: {}", dir.display(), e));
                        }
                        continue;
                    }
                };
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                {
                    let mut logs = logs.lock().unwrap();
                    logs.push(format!("Обнаружен файл/папка: {}", path.display()));
                }
                if path.is_file() && is_media_file(&name) {
                    let base_name = name.to_lowercase();
                    let matched_prefix = prefixes.iter()
                        .filter(|p| base_name.starts_with(&p.to_lowercase()))
                        .max_by_key(|p| p.len());
                    if let Some(prefix) = matched_prefix {
                        let articul = prefix.to_string();
                        let remaining = &base_name[prefix.len()..];
                        let photo_number = if let Some(caps) = Regex::new(r"^[_-](\d+)\.\w+$")?.captures(remaining) {
                            caps.get(1).unwrap().as_str().parse::<u32>().unwrap_or(1)
                        } else if remaining.starts_with('.') {
                            1
                        } else {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!(
                                "Файл {} содержит vendorCode {}, но не соответствует шаблону",
                                name, prefix
                            ));
                            continue;
                        };
                        let relative_path = path.strip_prefix(base_path).unwrap_or(&path).to_string_lossy().to_string();
                        let local_path = Path::new(download_path).join(&name).to_string_lossy().to_string();
                        if path != Path::new(&local_path) {
                            fs::copy(&path, &local_path)?;
                            {
                                let mut logs = logs.lock().unwrap();
                                logs.push(format!("Скопирован файл: {} в {}", path.display(), local_path));
                            }
                        }
                        files.push(FileInfo {
                            name: name.clone(),
                            path: relative_path,
                            local_path,
                            articul: articul.clone(),
                            photo_number,
                        });
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!(
                                "Найден локальный файл: {} (vendorCode: {}, фото: {})",
                                name, articul, photo_number
                            ));
                        }
                    } else {
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!(
                                "Файл {} не начинается ни с одного vendorCode: {:?}",
                                name, prefixes
                            ));
                        }
                    }
                } else if path.is_dir() {
                    scan_dir(&path, base_path, prefixes, files, logs, download_path)?;
                }
            }
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Завершено сканирование директории: {}", dir.display()));
            }
            Ok(())
        }

        scan_dir(source_path, source_path, &self.prefixes, &mut files, logs, &self.download_path)?;
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Всего найдено локальных файлов: {}", files.len()));
            if files.is_empty() {
                logs.push(format!("Не найдено файлов с префиксами: {:?}", self.prefixes));
            }
        }
        Ok(files)
    }

    pub fn get_download_link(&self, file_path: &str, logs: &Arc<Mutex<Vec<String>>>) -> Result<String> {
        for public_key in &self.public_keys {
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Получение ссылки для: {} с URL: {}", file_path, public_key));
            }
            let url = format!(
                "https://cloud-api.yandex.net/v1/disk/public/resources/download?public_key={}&path={}",
                encode(public_key), encode(file_path)
            );
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!(
                    "HTTP Request: GET {}\nHeaders: User-Agent: Mozilla/5.0; Accept: */*",
                    url
                ));
            }

            let mut attempts = 0;
            let max_attempts = 3;
            loop {
                let response = self.client.get(&url).send();
                match response {
                    Ok(response) => {
                        let status = response.status();
                        let body = response.text()?;
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!(
                                "HTTP Response: Status: {}, Body: {}",
                                status, body
                            ));
                        }
                        if status.is_success() {
                            let download_link: DownloadLink = serde_json::from_str(&body)?;
                            return Ok(download_link.href);
                        } else {
                            {
                                let mut logs = logs.lock().unwrap();
                                logs.push(format!("Ошибка получения ссылки для {}: {}", file_path, body));
                            }
                            if status.as_u16() == 401 {
                                {
                                    let mut logs = logs.lock().unwrap();
                                    logs.push(format!("Пропуск URL {} из-за ошибки 401", public_key));
                                }
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!("Ошибка HTTP запроса для {}: {}", file_path, e));
                        }
                    }
                }
                attempts += 1;
                if attempts >= max_attempts {
                    {
                        let mut logs = logs.lock().unwrap();
                        logs.push(format!("Не удалось получить ссылку для {} после {} попыток", file_path, max_attempts));
                    }
                    break;
                }
                {
                    let mut logs = logs.lock().unwrap();
                    logs.push(format!("Повторная попытка через 5 секунд ({}/{})", attempts, max_attempts));
                }
                std::thread::sleep(Duration::from_secs(5));
            }
        }
        Err(anyhow::anyhow!("Не удалось получить ссылку для {} ни с одного URL", file_path))
    }

    pub fn download_file(&self, url: &str, file_path: &str, logs: &Arc<Mutex<Vec<String>>>) -> Result<()> {
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Скачивание файла: {}", url));
        }
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!(
                "HTTP Request: GET {}\nHeaders: User-Agent: Mozilla/5.0; Accept: */*",
                url
            ));
        }

        let mut response = self.client.get(url).send()?;
        let status = response.status();
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!(
                "HTTP Response: Status: {}",
                status
            ));
        }

        if !status.is_success() {
            return Err(anyhow::anyhow!("Ошибка скачивания файла: {}", status));
        }
        let mut file = File::create(file_path)?;
        io::copy(&mut response, &mut file)?;
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Файл скачан: {}", file_path));
        }
        Ok(())
    }

    pub fn download_all(&self, logs: &Arc<Mutex<Vec<String>>>, total_files: &Arc<Mutex<Option<usize>>>, processed_files: &Arc<Mutex<usize>>) -> Result<Vec<FileInfo>> {
        {
            let mut logs = logs.lock().unwrap();
            logs.push("Начало скачивания всех файлов".to_string());
        }
        let files = self.find_files("/", logs)?;
        {
            let mut total = total_files.lock().unwrap();
            *total = Some(files.len());
        }
        if files.is_empty() {
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Не найдено файлов с префиксами: {:?}", self.prefixes));
            }
            return Ok(files);
        }
        let mut successful = 0;
        let mut failed = 0;
        for file in &files {
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Обработка файла: {}", file.name));
            }
            match self.get_download_link(&file.path, logs) {
                Ok(download_url) => {
                    if let Err(e) = self.download_file(&download_url, &file.local_path, logs) {
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!("Ошибка скачивания {}: {}", file.name, e));
                        }
                        failed += 1;
                    } else {
                        successful += 1;
                    }
                }
                Err(e) => {
                    {
                        let mut logs = logs.lock().unwrap();
                        logs.push(format!("Ошибка получения ссылки для {}: {}", file.name, e));
                    }
                    failed += 1;
                }
            }
            {
                let mut processed = processed_files.lock().unwrap();
                *processed += 1;
            }
        }
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Итог загрузки: {} успешно, {} провалено", successful, failed));
        }
        Ok(files)
    }

    pub fn generate_media_json(&self, nm_id: i64, files: &[FileInfo], logs: &Arc<Mutex<Vec<String>>>, _server_port: Option<u16>) -> Result<MediaOutput> {
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Генерация JSON для nmId: {}", nm_id));
        }
        let mut urls = vec![];
        for file in files {
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Обработка файла {} для nmId {}", file.name, nm_id));
            }
            if !self.public_keys.is_empty() {
                // Режим Яндекс.Диска: всегда получаем ссылку с Яндекс.Диска
                match self.get_download_link(&file.path, logs) {
                    Ok(download_url) => {
                        urls.push(download_url.clone());
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!("Добавлена ссылка Яндекс.Диска для {}: {}", file.name, download_url));
                        }
                    }
                    Err(e) => {
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!("Ошибка получения ссылки для {}: {}", file.name, e));
                        }
                        return Err(e);
                    }
                }
            } else {
                // Локальный режим: используем file://
                urls.push(format!("file://{}", file.local_path));
                {
                    let mut logs = logs.lock().unwrap();
                    logs.push(format!("Добавлен локальный путь для {}: file://{}", file.name, file.local_path));
                }
            }
        }
        if urls.is_empty() {
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Ошибка: не найдено файлов для nmId {}", nm_id));
            }
            return Err(anyhow::anyhow!("Не найдено файлов для nmId {}", nm_id));
        }
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Сгенерировано {} URL для nmId {}: {:?}", urls.len(), nm_id, urls));
        }
        Ok(MediaOutput { nm_id, data: urls })
    }

    pub fn cleanup_file(&self, file_path: &str, logs: &Arc<Mutex<Vec<String>>>) -> Result<()> {
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Очистка локального файла: {}", file_path));
        }
        if Path::new(file_path).exists() {
            fs::remove_file(file_path)?;
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Удален локальный файл: {}", file_path));
            }
        }
        Ok(())
    }

    pub fn cleanup_yandex_disk(&self, disk_path: &str, logs: &Arc<Mutex<Vec<String>>>) -> Result<()> {
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Ошибка: Очистка Яндекс.Диска не поддерживается без OAuth токена для {}", disk_path));
        }
        Err(anyhow::anyhow!("Очистка Яндекс.Диска не поддерживается без OAuth токена"))
    }
}
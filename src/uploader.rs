use anyhow::Result;
use reqwest::blocking::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct WbUploader {
    client: Client,
    #[allow(dead_code)]
    api_key: String,
}

#[derive(Serialize)]
struct CardRequest {
    settings: CardSettings,
}

#[derive(Serialize)]
struct CardSettings {
    cursor: Cursor,
    filter: Filter,
    sort: Sort,
}

#[derive(Serialize)]
struct Cursor {
    limit: i32,
}

#[derive(Serialize)]
struct Filter {
    #[serde(rename = "withPhoto")]
    with_photo: i32,
    #[serde(rename = "textSearch")]
    text_search: String,
}

#[derive(Serialize)]
struct Sort {
    ascending: bool,
}

#[derive(Deserialize)]
struct CardResponse {
    cards: Vec<Card>,
}

#[derive(Deserialize)]
struct Card {
    #[serde(rename = "nmID")]
    nm_id: i64,
}

impl WbUploader {
    pub fn new(api_key: String) -> Result<Self, anyhow::Error> {
        if api_key.is_empty() {
            log::error!("API ключ пустой");
            return Err(anyhow::anyhow!("API ключ пустой"));
        }
        log::info!(
            "Инициализация WbUploader с API ключом длиной: {}",
            api_key.len()
        );
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    "Authorization",
                    reqwest::header::HeaderValue::from_str(&api_key)?,
                );
                headers.insert(
                    "Accept",
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                headers.insert(
                    "Content-Type",
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
                headers
            })
            .build()
            .map_err(|e| anyhow::anyhow!("Не удалось создать HTTP-клиент: {}", e))?;
        Ok(Self { client, api_key })
    }

    pub fn get_nm_id_by_vendor_code(&self, vendor_code: &str) -> Result<i64, anyhow::Error> {
        log::info!("Запрос nmId для vendorCode: {}", vendor_code);
        let request_body = CardRequest {
            settings: CardSettings {
                cursor: Cursor { limit: 100 },
                filter: Filter {
                    with_photo: -1,
                    text_search: vendor_code.to_string(),
                },
                sort: Sort { ascending: false },
            },
        };
        log::debug!(
            "HTTP Request: POST https://content-api.wildberries.ru/content/v2/get/cards/list\nBody: {}",
            serde_json::to_string_pretty(&request_body)?
        );
        let response = self
            .client
            .post("https://content-api.wildberries.ru/content/v2/get/cards/list")
            .json(&request_body)
            .send()
            .map_err(|e| {
                anyhow::anyhow!(
                    "Не удалось отправить запрос для vendorCode {}: {}",
                    vendor_code,
                    e
                )
            })?;
        let status = response.status();
        let body = response.text().map_err(|e| {
            anyhow::anyhow!(
                "Не удалось прочитать ответ для vendorCode {}: {}",
                vendor_code,
                e
            )
        })?;
        log::debug!("HTTP Response: Status: {}, Body: {}", status, body);

        if !status.is_success() {
            log::error!("Ошибка API Wildberries: Статус {}, Тело: {}", status, body);
            return Err(anyhow::anyhow!(
                "Ошибка API Wildberries: Статус {}, Тело: {}",
                status,
                body
            ));
        }
        let card_response: CardResponse = serde_json::from_str(&body).map_err(|e| {
            anyhow::anyhow!(
                "Ошибка парсинга ответа для vendorCode {}: {}",
                vendor_code,
                e
            )
        })?;
        if let Some(card) = card_response.cards.first() {
            log::info!(
                "Найден nmId: {} для vendorCode: {}",
                card.nm_id,
                vendor_code
            );
            Ok(card.nm_id)
        } else {
            log::error!("nmId не найден для vendorCode: {}", vendor_code);
            Err(anyhow::anyhow!(
                "nmId не найден для vendorCode: {}",
                vendor_code
            ))
        }
    }

    pub fn upload_links(
        &self,
        nm_id: i64,
        urls: &[String],
        processed_files: &Arc<Mutex<usize>>,
    ) -> Result<(), anyhow::Error> {
        log::info!("Начало загрузки ссылок для nmId {}", nm_id);
        for url in urls {
            if !url.starts_with("http://")
                && !url.starts_with("https://")
                && !url.starts_with("file://")
            {
                log::error!("{} не является валидным URL", url);
                return Err(anyhow::anyhow!("Передан невалидный URL: {}", url));
            }
        }

        let mut attempts = 0;
        let max_attempts = 3;
        loop {
            let body = serde_json::json!({
                "nmId": nm_id,
                "data": urls
            });
            log::debug!(
                "HTTP Request: POST https://content-api.wildberries.ru/content/v3/media/save\nBody: {}",
                serde_json::to_string_pretty(&body)?
            );
            let response = self
                .client
                .post("https://content-api.wildberries.ru/content/v3/media/save")
                .json(&body)
                .send();
            match response {
                Ok(response) => {
                    let status = response.status();
                    let response_body = response.text().map_err(|e| {
                        anyhow::anyhow!("Не удалось прочитать ответ для nmId {}: {}", nm_id, e)
                    })?;
                    log::debug!("HTTP Response: Status: {}, Body: {}", status, response_body);
                    if status.is_success() {
                        log::info!("Загружены ссылки на WB для nmId {}: {:?}", nm_id, urls);
                        {
                            let mut processed = processed_files.lock().unwrap();
                            *processed += 1;
                        }
                        return Ok(());
                    } else if status.as_u16() == 429 {
                        log::warn!(
                            "Ошибка 429: Слишком много запросов для nmId {}, повторная попытка через 60 секунд (попытка {}/{})",
                            nm_id,
                            attempts + 1,
                            max_attempts
                        );
                        if attempts >= max_attempts {
                            log::error!(
                                "Не удалось загрузить ссылки для nmId {} после {} попыток",
                                nm_id,
                                max_attempts
                            );
                            return Err(anyhow::anyhow!(
                                "Не удалось загрузить ссылки после {} попыток",
                                max_attempts
                            ));
                        }
                        thread::sleep(Duration::from_secs(60));
                    } else {
                        log::error!(
                            "Ошибка загрузки ссылок на WB для nmId {}: {}",
                            nm_id,
                            response_body
                        );
                        return Err(anyhow::anyhow!(
                            "Ошибка загрузки ссылок: Статус {}, Тело: {}",
                            status,
                            response_body
                        ));
                    }
                }
                Err(e) => {
                    log::error!("Ошибка HTTP запроса для nmId {}: {}", nm_id, e);
                    if attempts >= max_attempts {
                        log::error!(
                            "Не удалось загрузить ссылки для nmId {} после {} попыток",
                            nm_id,
                            max_attempts
                        );
                        return Err(anyhow::anyhow!(
                            "Не удалось загрузить ссылки после {} попыток",
                            max_attempts
                        ));
                    }
                    log::warn!(
                        "Ошибка HTTP запроса, повторная попытка через 60 секунд (попытка {}/{})",
                        attempts + 1,
                        max_attempts
                    );
                    thread::sleep(Duration::from_secs(60));
                }
            }
            attempts += 1;
        }
    }

    pub fn upload_local_file(
        &self,
        nm_id: i64,
        file_path: &str,
        photo_number: u32,
        processed_files: &Arc<Mutex<usize>>,
    ) -> Result<(), anyhow::Error> {
        log::info!(
            "Начало загрузки файла {} для nmId {} с номером фото {}",
            file_path,
            nm_id,
            photo_number
        );

        // Проверка существования файла
        if !Path::new(file_path).exists() {
            log::error!("Файл {} не существует", file_path);
            return Err(anyhow::anyhow!("Файл {} не существует", file_path));
        }

        // Чтение файла в память
        let mut file = File::open(file_path).map_err(|e| {
            anyhow::anyhow!("Не удалось открыть файл {}: {}", file_path, e)
        })?;
        let mut file_content = Vec::new();
        file.read_to_end(&mut file_content).map_err(|e| {
            anyhow::anyhow!("Не удалось прочитать файл {}: {}", file_path, e)
        })?;

        let mut attempts = 0;
        let max_attempts = 3;
        loop {
            // Формирование multipart формы внутри цикла
            let form = reqwest::blocking::multipart::Form::new().part(
                "uploadfile",
                reqwest::blocking::multipart::Part::bytes(file_content.clone())
                    .file_name(Path::new(file_path).file_name().unwrap().to_string_lossy().to_string())
                    .mime_str("application/octet-stream")?,
            );

            log::debug!(
                "HTTP Request: POST https://content-api.wildberries.ru/content/v3/media/file\nX-Nm-Id: {}, X-Photo-Number: {}",
                nm_id,
                photo_number
            );
            let response = self
                .client
                .post("https://content-api.wildberries.ru/content/v3/media/file")
                .header("X-Nm-Id", nm_id.to_string())
                .header("X-Photo-Number", photo_number.to_string())
                .multipart(form)
                .send();

            match response {
                Ok(response) => {
                    let status = response.status();
                    let response_body = response.text().map_err(|e| {
                        anyhow::anyhow!(
                            "Не удалось прочитать ответ для файла {}: {}",
                            file_path,
                            e
                        )
                    })?;
                    log::debug!("HTTP Response: Status: {}, Body: {}", status, response_body);

                    if status.is_success() {
                        log::info!(
                            "Файл {} загружен для nmId {} с номером фото {}",
                            file_path,
                            nm_id,
                            photo_number
                        );
                        {
                            let mut processed = processed_files.lock().unwrap();
                            *processed += 1;
                        }
                        return Ok(());
                    } else if status.as_u16() == 429 {
                        log::warn!(
                            "Ошибка 429: Слишком много запросов для файла {}, повторная попытка через 60 секунд (попытка {}/{})",
                            file_path,
                            attempts + 1,
                            max_attempts
                        );
                        if attempts >= max_attempts {
                            log::error!(
                                "Не удалось загрузить файл {} после {} попыток",
                                file_path,
                                max_attempts
                            );
                            return Err(anyhow::anyhow!(
                                "Не удалось загрузить файл после {} попыток",
                                max_attempts
                            ));
                        }
                        thread::sleep(Duration::from_secs(60));
                    } else {
                        log::error!(
                            "Ошибка загрузки файла {} для nmId {}: Статус {}, Тело: {}",
                            file_path,
                            nm_id,
                            status,
                            response_body
                        );
                        return Err(anyhow::anyhow!(
                            "Ошибка загрузки файла: Статус {}, Тело: {}",
                            status,
                            response_body
                        ));
                    }
                }
                Err(e) => {
                    log::error!("Ошибка HTTP запроса для файла {}: {}", file_path, e);
                    if attempts >= max_attempts {
                        log::error!(
                            "Не удалось загрузить файл {} после {} попыток",
                            file_path,
                            max_attempts
                        );
                        return Err(anyhow::anyhow!(
                            "Не удалось загрузить файл после {} попыток",
                            max_attempts
                        ));
                    }
                    log::warn!(
                        "Ошибка HTTP запроса, повторная попытка через 60 секунд (попытка {}/{})",
                        attempts + 1,
                        max_attempts
                    );
                    thread::sleep(Duration::from_secs(60));
                }
            }
            attempts += 1;
        }
    }
}
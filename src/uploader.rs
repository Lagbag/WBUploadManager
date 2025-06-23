use anyhow::Result;
use reqwest::blocking::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct WbUploader {
    client: Client,
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
    pub fn new(api_key: String) -> Result<Self> {
        if api_key.is_empty() {
            return Err(anyhow::anyhow!("API ключ пустой"));
        }
        println!("Инициализация WbUploader с API ключом длиной: {}", api_key.len());
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert("Authorization", reqwest::header::HeaderValue::from_str(&api_key)?);
                headers.insert("Accept", reqwest::header::HeaderValue::from_static("application/json"));
                headers.insert("Content-Type", reqwest::header::HeaderValue::from_static("application/json"));
                headers
            })
            .build()?;
        Ok(Self { client, api_key })
    }

    pub fn get_nm_id_by_vendor_code(&self, vendor_code: &str, logs: &Arc<Mutex<Vec<String>>>) -> Result<i64> {
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Запрос nmId для vendorCode: {}", vendor_code));
        }
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
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!(
                "HTTP Request: POST https://content-api.wildberries.ru/content/v2/get/cards/list\nBody: {}",
                serde_json::to_string_pretty(&request_body)?
            ));
        }
        let response = self.client
            .post("https://content-api.wildberries.ru/content/v2/get/cards/list")
            .json(&request_body)
            .send()?;
        let status = response.status();
        let body = response.text()?;
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!(
                "HTTP Response: Status: {}, Body: {}",
                status, body
            ));
        }
        if !status.is_success() {
            return Err(anyhow::anyhow!("Ошибка API Wildberries: Статус {}, Тело: {}", status, body));
        }
        let card_response: CardResponse = serde_json::from_str(&body)?;
        if let Some(card) = card_response.cards.first() {
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("Найден nmId: {} для vendorCode: {}", card.nm_id, vendor_code));
            }
            Ok(card.nm_id)
        } else {
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!("nmId не найден для vendorCode: {}", vendor_code));
            }
            Err(anyhow::anyhow!("nmId не найден для vendorCode: {}", vendor_code))
        }
    }

    pub fn upload_links(&self, nm_id: i64, urls: &[String], logs: &Arc<Mutex<Vec<String>>>, processed_files: &Arc<Mutex<usize>>) -> Result<()> {
        {
            let mut logs = logs.lock().unwrap();
            logs.push(format!("Начало загрузки ссылок для nmId {}", nm_id));
        }
        for url in urls {
            if !url.starts_with("http://") && !url.starts_with("https://") && !url.starts_with("file://") {
                {
                    let mut logs = logs.lock().unwrap();
                    logs.push(format!("Ошибка: {} не является валидным URL", url));
                }
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
            {
                let mut logs = logs.lock().unwrap();
                logs.push(format!(
                    "HTTP Request: POST https://content-api.wildberries.ru/content/v3/media/save\nHeaders: Authorization: [REDACTED]\nBody: {}",
                    serde_json::to_string_pretty(&body)?
                ));
            }
            let response = self.client
                .post("https://content-api.wildberries.ru/content/v3/media/save")
                .json(&body)
                .send();
            match response {
                Ok(response) => {
                    let status = response.status();
                    let response_body = response.text()?;
                    {
                        let mut logs = logs.lock().unwrap();
                        logs.push(format!(
                            "HTTP Response: Status: {}, Body: {}",
                            status, response_body
                        ));
                    }
                    if status.is_success() {
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!("Загружены ссылки на WB для nmId {}: {:?}", nm_id, urls));
                        }
                        {
                            let mut processed = processed_files.lock().unwrap();
                            *processed += 1;
                        }
                        return Ok(());
                    } else if status.as_u16() == 429 {
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!("Ошибка 429: Слишком много запросов для nmId {}, повторная попытка через 60 секунд (попытка {}/{})", nm_id, attempts + 1, max_attempts));
                        }
                        if attempts >= max_attempts {
                            {
                                let mut logs = logs.lock().unwrap();
                                logs.push(format!("Не удалось загрузить ссылки для nmId {} после {} попыток", nm_id, max_attempts));
                            }
                            return Err(anyhow::anyhow!("Не удалось загрузить ссылки после {} попыток", max_attempts));
                        }
                        thread::sleep(Duration::from_secs(60));
                    } else {
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!("Ошибка загрузки ссылок на WB для nmId {}: {}", nm_id, response_body));
                        }
                        return Err(anyhow::anyhow!("Ошибка загрузки ссылок: Статус {}, Тело: {}", status, response_body));
                    }
                }
                Err(e) => {
                    {
                        let mut logs = logs.lock().unwrap();
                        logs.push(format!("Ошибка HTTP запроса для nmId {}: {}", nm_id, e));
                    }
                    if attempts >= max_attempts {
                        {
                            let mut logs = logs.lock().unwrap();
                            logs.push(format!("Не удалось загрузить ссылки для nmId {} после {} попыток", nm_id, max_attempts));
                        }
                        return Err(anyhow::anyhow!("Не удалось загрузить ссылки после {} попыток", max_attempts));
                    }
                    {
                        let mut logs = logs.lock().unwrap();
                        logs.push(format!("Ошибка HTTP запроса, повторная попытка через 60 секунд (попытка {}/{})", attempts + 1, max_attempts));
                    }
                    thread::sleep(Duration::from_secs(60));
                }
            }
            attempts += 1;
        }
    }
}
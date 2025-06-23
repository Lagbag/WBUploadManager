use serde::{Deserialize, Serialize};
use crate::config::Config;
use anyhow::Result;

#[derive(Serialize, Deserialize, Clone)]
pub struct Profile {
    pub name: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct ProfileManager {
    pub profiles: Vec<Profile>,
    pub selected_index: usize,
    #[serde(skip)]
    pub config: Config,
}

impl ProfileManager {
    pub fn new() -> Result<Self> {
        log::info!("Инициализация ProfileManager");
        let config = Config::new()?;
        let config_file = config.get_config_file_path();
        let profiles: Vec<Profile> = if config_file.exists() {
            let data = std::fs::read_to_string(&config_file)
                .map_err(|e| anyhow::anyhow!("Не удалось прочитать файл конфигурации {}: {}", config_file.display(), e))?;
            serde_json::from_str(&data).unwrap_or_else(|e| {
                log::warn!("Ошибка парсинга конфигурации, используется профиль по умолчанию: {}", e);
                vec![Profile {
                    name: "Добавить".to_string(),
                    api_key: String::new(),
                }]
            })
        } else {
            log::info!("Конфигурация не найдена, создаётся профиль по умолчанию");
            vec![Profile {
                name: "Добавить".to_string(),
                api_key: String::new(),
            }]
        };
        Ok(ProfileManager {
            profiles,
            selected_index: 0,
            config,
        })
    }

    pub fn add_profile(&mut self, name: String) {
        log::info!("Добавление профиля: {}", name);
        self.profiles.push(Profile {
            name,
            api_key: String::new(),
        });
        self.selected_index = self.profiles.len() - 1;
    }

    pub fn delete_profile(&mut self, index: usize) {
        log::info!("Удаление профиля: {}", self.profiles[index].name);
        self.profiles.remove(index);
        if self.selected_index >= self.profiles.len() {
            self.selected_index = self.profiles.len().saturating_sub(1);
        }
    }

    pub fn current_profile(&self) -> &Profile {
        &self.profiles[self.selected_index]
    }

    pub fn current_profile_mut(&mut self) -> &mut Profile {
        &mut self.profiles[self.selected_index]
    }

    pub fn save(&self) -> Result<()> {
        log::info!("Сохранение профилей");
        let config_file = self.config.get_config_file_path();
        let data = serde_json::to_string_pretty(&self.profiles)
            .map_err(|e| anyhow::anyhow!("Ошибка сериализации профилей: {}", e))?;
        std::fs::write(&config_file, data)
            .map_err(|e| anyhow::anyhow!("Не удалось записать файл конфигурации {}: {}", config_file.display(), e))?;
        log::info!("Профили сохранены в {}", config_file.display());
        Ok(())
    }
}
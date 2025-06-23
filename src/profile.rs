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
        let config = Config::new()?;
        let config_file = config.get_config_file_path();
        let profiles: Vec<Profile> = if config_file.exists() {
            let data = std::fs::read_to_string(&config_file)?;
            serde_json::from_str(&data).unwrap_or_else(|_| vec![Profile {
                name: "Default".to_string(),
                api_key: String::new(),
            }])
        } else {
            vec![Profile {
                name: "Default".to_string(),
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
        self.profiles.push(Profile {
            name,
            api_key: String::new(),
        });
        self.selected_index = self.profiles.len() - 1;
    }

    pub fn current_profile(&self) -> &Profile {
        &self.profiles[self.selected_index]
    }

    pub fn current_profile_mut(&mut self) -> &mut Profile {
        &mut self.profiles[self.selected_index]
    }

    pub fn save(&self) -> Result<()> {
        let config_file = self.config.get_config_file_path();
        let data = serde_json::to_string_pretty(&self.profiles)?;
        std::fs::write(&config_file, data)?;
        Ok(())
    }
}
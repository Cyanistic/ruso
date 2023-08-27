use dioxus::prelude::*;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Props, PartialEq)]
pub struct MapOptions{
    pub approach_rate: f64,
    pub circle_size: f64,
    pub hp_drain: f64,
    pub overall_difficulty: f64,
    pub background: Option<PathBuf>,
    pub map_path: PathBuf,
    pub songs_path: PathBuf,
    pub rate: f64,
}

impl MapOptions{
    pub fn new() -> Self{
        MapOptions { 
            approach_rate: 5.0,
            circle_size: 5.0,
            hp_drain: 5.0,
            overall_difficulty: 5.0,
            background: None,
            map_path: PathBuf::new(), 
            songs_path: PathBuf::new(),
            rate: 1.0
        }
    }
}

impl Default for MapOptions{
    fn default() -> Self{
        Self::new()
    }
}


#[derive(Debug, Props, PartialEq, Serialize, Deserialize)]
pub struct Settings{
    pub theme: Theme,
    pub ar_lock: bool,
    pub cs_lock: bool,
    pub hp_lock: bool,
    pub od_lock: bool,
    pub songs_path: PathBuf,
    pub gosumemory_path: PathBuf,
    pub gosumemory_startup: bool,
    pub websocket_url: String,
}

impl Settings{
    pub fn new() -> Self{
        Settings{
            theme: Theme::Dark,
            ar_lock: false,
            cs_lock: false,
            hp_lock: false,
            od_lock: false,
            songs_path: PathBuf::new(),
            gosumemory_path: PathBuf::new(),
            gosumemory_startup: false,
            websocket_url: "ws://localhost:24050/ws".to_string()
        }
    }

    pub fn new_from_config() -> Self{
        let config_file = dirs::config_dir().unwrap().join("ruso").join("settings.json");
        if config_file.exists(){
            let config_data = std::fs::read_to_string(config_file).unwrap();
            match serde_json::from_str(&config_data){
                Ok(k) => k,
                Err(e) => {
                    eprintln!("Error parsing config file: {}, using default settings", e);
                    Self::new()
                }
            }
        }else {
            Self::new()
        }
    }
}

impl Default for Settings{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct StatusMessage{
    pub text: Option<String>,
    pub status: Status
}

impl StatusMessage{
    pub fn new() -> Self{
        StatusMessage { 
            text: None,
            status: Status::Success
        }
    }
}

impl Default for StatusMessage{
    fn default() -> Self{
        Self::new()
    }
}

#[derive(Debug)]
pub enum Status{
    Success,
    Error
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Theme{
    Light,
    Dark,
    Osu,
    Custom
}

#[derive(Debug, Clone)]
pub enum Tab{
    Auto,
    Manual,
    Settings
}



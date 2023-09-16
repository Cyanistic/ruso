use dioxus::prelude::*;
use std::{path::PathBuf, io::{ErrorKind, Write}, fs::File};
use libosu::data::Mode;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Props, PartialEq)]
pub struct MapOptions{
    pub approach_rate: f64,
    pub artist: Box<str>,
    pub background: Option<PathBuf>,
    pub bpm: usize,
    pub circle_size: f64,
    pub difficulty_name: Box<str>,
    pub hp_drain: f64,
    pub map_path: PathBuf,
    pub mode: Mode,
    pub overall_difficulty: f64,
    pub rate: f64,
    pub stars: f64,
    pub title: Box<str>,
}

impl MapOptions{
    pub fn new() -> Self{
        MapOptions { 
            approach_rate: 5.0,
            artist: "".into(),
            background: None,
            bpm: 100,
            circle_size: 5.0,
            difficulty_name: "".into(),
            hp_drain: 5.0,
            map_path: PathBuf::new(), 
            mode: Mode::Osu,
            overall_difficulty: 5.0,
            rate: 1.0,
            stars: 0.0,
            title: "".into(),
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
    pub ar_lock: bool,
    pub cs_lock: bool,
    pub generate_osz: bool,
    pub gosumemory_path: PathBuf,
    pub gosumemory_startup: bool,
    pub hp_lock: bool,
    pub od_lock: bool,
    pub songs_path: PathBuf,
    pub theme: Theme,
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
            generate_osz: true,
            songs_path: PathBuf::new(),
            gosumemory_path: PathBuf::new(),
            gosumemory_startup: false,
            websocket_url: "ws://localhost:24050/ws".to_string()
        }
    }

    /// Attempt to create a settings struct from an existing 'settings.json' file
    pub fn new_from_config() -> Self{
        let config_file = dirs::config_dir().unwrap().join("ruso").join("settings.json");
        let config_data = match std::fs::read_to_string(&config_file){
            Ok(k) => k,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                // Config file does not exist, create it
                let mut config_file = File::create(&config_file).expect("Could not find or create settings.json");
                let default_config = serde_json::to_string_pretty(&Self::new()).unwrap();

                // Use default config if a configuration could not be created
                if let Err(e) = config_file.write_all(default_config.as_bytes()){
                    eprintln!("Could not create config file: {}", e);
                    return Self::new()
                }
                return Self::new_from_config()
            },
            Err(e) => panic!("Error reading config file: {}", e)
        };
        match serde_json::from_str(&config_data){
            Ok(k) => k,
            Err(e) => {
                eprintln!("Error parsing config file: {}, using default settings", e);
                Self::new()
            }
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

#[derive(Debug, PartialEq, Clone)]
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



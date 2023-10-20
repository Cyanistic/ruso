use anyhow::Result;
use dioxus::prelude::*;
use serde_json::error::Category;
use std::{path::PathBuf, io::{ErrorKind, Write, BufReader}, fs::File};
use libosu::{data::Mode, events::Event::Background};
use rosu_pp::BeatmapExt;
use serde::{Serialize, Deserialize};
use crate::utils::{calculate_bpm, round_dec};

// #[derive(Clone)]
// pub struct AppProps<'a>{
//     pub dropped_file: Rc<Cell<Option<&'a PathBuf>>>
// }

// impl AppProps<'_>{
//     pub fn new() -> Self{
//         AppProps{
//             dropped_file: Rc::new(Cell::new(None))
//         }
//     }
// }

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
    pub original_ar: f64,
    pub original_od: f64,
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
            original_ar: 5.0,
            original_od: 5.0,
            overall_difficulty: 5.0,
            rate: 1.0,
            stars: 0.0,
            title: "".into(),
        }
    }

    /// Reads the data of a .osu map file and modifies the MapOptions struct accordingly
    pub fn read_map_metadata(&mut self, settings: &Settings) -> Result<()>{
        let map = libosu::beatmap::Beatmap::parse(BufReader::new(File::open(settings.songs_path.join(&self.map_path))?))?;
        let stars = rosu_pp::Beatmap::from_path(settings.songs_path.join(&self.map_path))?.stars().calculate().stars();
        self.original_ar = map.difficulty.approach_rate;
        self.original_od = map.difficulty.overall_difficulty;
        if !settings.ar_lock{
            if settings.scale_ar{
                self.scale_ar();
            }else{
                self.approach_rate = map.difficulty.approach_rate;
            }
        }
        if !settings.cs_lock{
            self.circle_size = map.difficulty.circle_size;
        }
        if !settings.hp_lock{
            self.hp_drain = map.difficulty.hp_drain_rate;
        }
        if !settings.od_lock{
            if settings.scale_od{
                self.scale_od();
            }else{
                self.overall_difficulty = map.difficulty.overall_difficulty;
            }
        }
        self.bpm = calculate_bpm(&map.timing_points);
        self.background = {
            let mut bg = None;
            for i in map.events{
                if let Background(b) = i{
                    bg = Some(self.map_path.parent().unwrap().to_path_buf().join(PathBuf::from(b.filename)));
                    break;
                }
            }
            bg
        };
        self.mode = map.mode;
        self.stars = round_dec(stars, 2);
        self.title = map.title.into();
        self.artist = map.artist.into();
        self.difficulty_name = map.difficulty_name.into();
        Ok(())
    }

    // Code logic copied from https://github.com/hwsmm/cosutrainer/blob/9bc998977976116c4cd2e559dc85d46cfeb191cd/src/mapeditor.c#L98
    /// Scales the approach rate with the given rate.
    pub fn scale_ar(&mut self){
        match self.mode {
            Mode::Taiko | Mode::Mania => (),
            Mode::Osu | Mode::Catch => {
                let mut ar_ms = if self.original_ar <= 5.0 {
                     1200.0 + 600.0 * (5.0 - self.original_ar) / 5.0
                }else{
                     1200.0 - 750.0 * (self.original_ar - 5.0) / 5.0
                };
                ar_ms /= self.rate;

                self.approach_rate = round_dec(if ar_ms >= 1200.0 {
                    15.0 - ar_ms / 120.0 
                }else{
                    (1200.0 / 150.0) - (ar_ms / 150.0) + 5.0
                }, 2).min(10.0).max(0.0);
                // Added min and max to keep the ar within a valid range
            }
        }
    }

    // Code logic copied from https://github.com/hwsmm/cosutrainer/blob/9bc998977976116c4cd2e559dc85d46cfeb191cd/src/mapeditor.c#L108
    /// Scales the overall difficulty with the given rate.
    pub fn scale_od(&mut self){
        self.overall_difficulty = round_dec(match self.mode{
            Mode::Osu => (80.0 - (80.0 - 6.0 * self.original_od) / self.rate) / 6.0,
            Mode::Taiko => (80.0 - (80.0 - 6.0 * self.original_od) / self.rate) / 6.0,
            Mode::Catch => self.overall_difficulty,
            Mode::Mania => (64.0 - (64.0 - 3.0 * self.original_od) / self.rate) / 3.0,
        }, 2).min(10.0).max(0.0);
        // Added min and max to keep the od within a valid range
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
    pub change_pitch: bool,
    pub cs_lock: bool,
    pub force_generation: bool,
    pub generate_osz: bool,
    pub gosumemory_path: PathBuf,
    pub gosumemory_startup: bool,
    pub hp_lock: bool,
    pub od_lock: bool,
    pub scale_ar: bool,
    pub scale_od: bool,
    pub songs_path: PathBuf,
    pub theme: Theme,
    pub websocket_url: String,
}

impl Settings{
    pub fn new() -> Self{
        Settings{
            theme: Theme::Dark,
            ar_lock: false,
            change_pitch: true,
            cs_lock: false,
            force_generation: false,
            hp_lock: false,
            od_lock: false,
            generate_osz: true,
            scale_ar: false,
            scale_od: false,
            songs_path: PathBuf::new(),
            gosumemory_path: PathBuf::new(),
            gosumemory_startup: false,
            websocket_url: "ws://127.0.0.1:24050/ws".to_string()
        }
    }

    /// Attempt to create a settings struct from an existing 'settings.json' file
    pub fn new_from_config() -> Self{
        let config_dir = match dirs::config_dir(){
            Some(k) => k,
            None => return Self::new()
        }.join("ruso");
        if !config_dir.exists(){
            if let Err(e) = std::fs::create_dir_all(&config_dir){
                eprintln!("Could not create config directory: {}", e);
                return Self::new()
            }
        }
        let config_file = config_dir.join("settings.json");
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
            Err(e) if e.classify() == Category::Syntax  => {
                eprintln!("Syntax error in config file: {}, using default settings.", e);
                Self::new()
            },
            Err(e) if e.classify() == Category::Data  => {
                eprintln!("The config file is incorrect: {}, using default settings.", e);
                Self::new()
            },
            Err(e) => {
                eprintln!("Error parsing config file: {}, using default settings.", e);
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

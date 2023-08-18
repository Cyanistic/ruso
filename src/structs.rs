use dioxus::prelude::*;
use std::path::PathBuf;
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
            // difficulty: HashMap::from([
            //    ("HP Drain", 5.0),
            //    ("Circle Size", 5.0),
            //    ("Approach Rate", 5.0),
            //    ("Overall Difficulty", 5.0),
            // ]),
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


#[derive(Debug, Props, PartialEq)]
pub struct Settings{
    pub slider_scroll: bool,
    pub theme: Theme,
    pub ar_lock: bool,
    pub cs_lock: bool,
    pub hp_lock: bool,
    pub od_lock: bool
}

impl Settings{
    pub fn new() -> Self{
        Settings{
            slider_scroll: false,
            theme: Theme::Dark,
            ar_lock: false,
            cs_lock: false,
            hp_lock: false,
            od_lock: false
        }
    }
}

impl Default for Settings{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq)]
pub enum Theme{
    Light,
    Dark,
    Osu,
    Custom
}



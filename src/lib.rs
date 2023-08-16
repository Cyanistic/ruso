use std::{path::PathBuf, fs::File, io::Write};
use anyhow::Result;
use dioxus::prelude::Props;
use libosu::prelude::{HitObjectKind};
extern crate gstreamer as gst;
use gst::{prelude::*, MessageType};

#[derive(Debug, Props, PartialEq)]
pub struct MapOptions{
    pub approach_rate: f64,
    pub circle_size: f64,
    pub hp_drain: f64,
    pub map_path: PathBuf,
    pub overall_difficulty: f64,
    pub parent_path: PathBuf,
    pub rate: f64,
}

impl MapOptions {
    pub fn new() -> Self{
        MapOptions { 
            approach_rate: 5.0, 
            circle_size: 5.0, 
            hp_drain: 5.0, 
            map_path: PathBuf::new(), 
            overall_difficulty: 5.0, 
            parent_path: PathBuf::new(),
            rate: 1.0
        }
    }
}

#[derive(Debug, Props, PartialEq)]
pub struct Settings{
    pub slider_scroll: bool,
    pub theme: Theme
}

#[derive(Debug, PartialEq)]
pub enum Theme{
    Light,
    Dark,
    Osu,
    Custom
}

impl Default for MapOptions{
    fn default() -> Self{
        MapOptions { 
            approach_rate: 5.0, 
            circle_size: 5.0, 
            hp_drain: 5.0, 
            map_path: PathBuf::new(), 
            overall_difficulty: 5.0, 
            parent_path: PathBuf::new(),
            rate: 1.0
        }
    }
}

pub async fn generate_map(path: &PathBuf, rate: f64) -> Result<()>{
    let map_file = File::open(path)?;
    let mut map_data = libosu::beatmap::Beatmap::parse(map_file)?;
    let audio_path = path.parent().unwrap().join(&map_data.audio_filename);
    map_data.audio_filename = format!("{}({}).{}", &audio_path.file_stem().unwrap().to_str().unwrap(), rate, &audio_path.extension().unwrap().to_str().unwrap());
    let audio_thread = std::thread::spawn(move || {
        generate_audio(&audio_path, rate)?;
        Ok::<(), anyhow::Error>(())
    });
    for h in &mut map_data.hit_objects{
        h.start_time.0 = (rate / *h.start_time as f64).round() as i32;
        match &mut h.kind {
            HitObjectKind::Hold(k) => {
                k.end_time.0 = (rate / *k.end_time as f64).round() as i32;
            },
            HitObjectKind::Spinner(k) => {
                k.end_time.0 = (rate / *k.end_time as f64).round() as i32;
            },
            _ => {}
        }
    }
    for t in &mut map_data.timing_points{
        t.time.0 = (rate / t.time.0 as f64).round() as i32;
    }
    let new_path = path.parent().unwrap().join(path.file_stem().unwrap());
    write!(File::create(format!("{}({}).osu", new_path.display(), rate))?,"{}", map_data)?;
    if let Err(e) = audio_thread.join(){
        return Err(anyhow::anyhow!("Error generating audio file: {:?}", e))
    }
    Ok(())
}

fn generate_audio(audio_path: &PathBuf, rate: f64) -> Result<()>{
    println!("{:?}", audio_path);
    gst::init()?;
    let pipeline_description = match audio_path.extension().unwrap().to_str().unwrap().to_lowercase().as_str(){
        "mp3" => format!(
           "filesrc location=\"{}\" ! mpegaudioparse ! mpg123audiodec ! decodebin ! audioconvert ! audioresample ! speed speed={} ! audioconvert ! audioresample ! lamemp3enc target=quality quality=0 ! id3v2mux ! filesink location=\"{}({}).{}\"",
           &audio_path.display(),
           &rate,
           &audio_path.parent().unwrap().join(audio_path.file_stem().unwrap()).display(),
           &rate,
           &audio_path.extension().unwrap().to_str().unwrap()
        ),
        "ogg" => format!(
           "filesrc location=\"{}\" ! oggdemux ! vorbisdec ! audioconvert ! speed speed={} ! vorbisenc ! oggmux ! filesink location=\"{}({}).{}\"",
           &audio_path.display(),
           &rate,
           &audio_path.parent().unwrap().join(audio_path.file_stem().unwrap()).display(),
           &rate,
           &audio_path.extension().unwrap().to_str().unwrap()
        ),
        "wav" => format!(
           "filesrc location=\"{}\" ! wavparse ! audioconvert ! audioresample ! speed speed={} ! audioconvert ! wavenc ! filesink location=\"{}({}).{}\"",
           &audio_path.display(),
           &rate,
           &audio_path.parent().unwrap().join(audio_path.file_stem().unwrap()).display(),
           &rate,
           &audio_path.extension().unwrap().to_str().unwrap()
        ),
        e => return Err(anyhow::anyhow!("Unsupported file type: {}", e))
    };
    println!("{}", pipeline_description);
    
    let pipeline = gst::parse_launch(&pipeline_description)?;
    pipeline.set_state(gst::State::Playing)?;
    let bus = pipeline.bus().unwrap();
    bus.add_signal_watch();
        loop {
            if bus.pop().is_some_and(|x| x.as_ref().type_().eq(&MessageType::Eos)){
                break;
            }
        }
    Ok(())
}

pub fn round_dec(x: f64, decimals: u32) -> f64 {
    let y = 10i32.pow(decimals) as f64;
    (x * y).round() / y
}

#[cfg(test)]
mod test{
    use super::*;
    #[tokio::test]
    async fn test1(){
        generate_map(&PathBuf::from("/home/cyan/.local/share/osu-wine/osu!/Songs/991895 Kondo Koji - Slider/Kondo Koji - Slider (NikoSek) [YaHoo!!].osu"), 1.9).await.unwrap();
    }
    #[tokio::test]
    async fn test2(){
        generate_map(&PathBuf::from("/home/cyan/.local/share/osu-wine/osu!/Songs/1869337 Fellowship - Glory Days/Fellowship - Glory Days (EdgyKing) [Selfless Journey].osu"), 3.0).await.unwrap();
    }
}

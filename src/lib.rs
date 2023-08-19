use std::{path::{PathBuf, Path}, fs::{File, OpenOptions}, io::{Write, ErrorKind, Read}, time::Duration, sync::Arc};
use anyhow::Result;
use libosu::{prelude::*, events::Event::Background};
extern crate gstreamer as gst;
use gst::{prelude::*, MessageType};
pub mod structs;
pub use structs::{MapOptions, Settings};
use tokio_tungstenite::{tungstenite::connect, connect_async};
use tokio::{io::AsyncWriteExt, sync::Mutex};
use futures_util::{SinkExt, Stream, StreamExt};
use serde_json::{to_value, json, from_value, from_str};

pub fn read_map_metadata(options: MapOptions, settings: &Settings) -> Result<MapOptions>{
    let map = Beatmap::parse(File::open(options.songs_path.join(&options.map_path))?)?;
    let mut new_options = MapOptions{
        approach_rate: map.difficulty.approach_rate as f64,
        overall_difficulty: map.difficulty.overall_difficulty as f64,
        circle_size: map.difficulty.circle_size as f64,
        hp_drain: map.difficulty.hp_drain_rate as f64,
        background: {
            let mut bg = None;
            for i in map.events{
                if let Background(b) = i{
                    bg = Some(options.map_path.parent().unwrap().to_path_buf().join(PathBuf::from(b.filename)));
                    break;
                }
            }
            bg
        },
        ..options
    };
    if settings.ar_lock{
        new_options.approach_rate = options.approach_rate;
    }
    if settings.cs_lock{
        new_options.circle_size = options.circle_size;
    }
    if settings.hp_lock{
        new_options.hp_drain = options.hp_drain;
    }
    if settings.od_lock{
        new_options.overall_difficulty = options.overall_difficulty;
    }
    Ok(new_options)
}

pub fn generate_map(map: &MapOptions) -> Result<()>{
    let path = &map.songs_path.join(&map.map_path);
    let rate = map.rate;
    let map_file = File::open(path)?;
    let mut map_data = libosu::beatmap::Beatmap::parse(map_file)?;
    let audio_path = path.parent().unwrap().join(&map_data.audio_filename);
    let cache_dir = match dirs::cache_dir(){
        Some(k) => k,
        None => return Err(anyhow::anyhow!("Couldn't find cache directory"))
    }.join("ruso");
    let mut cache_file = match OpenOptions::new().append(true).open(cache_dir.join("maps.txt")){
        Ok(k) => k,
        Err(e) if e.kind() == ErrorKind::NotFound => File::create(cache_dir.join("maps.txt"))?,
        Err(e) => return Err(anyhow::anyhow!("Error opening maps.txt: {}", e))
    };

    map_data.audio_filename = format!("{}({}).{}", &audio_path.file_stem().unwrap().to_str().unwrap(), rate, &audio_path.extension().unwrap().to_str().unwrap());
    map_data.difficulty_name += format!("({}x)",rate).as_str(); 
    map_data.difficulty.approach_rate = map.approach_rate as f32;
    map_data.difficulty.circle_size = map.circle_size as f32;
    map_data.difficulty.hp_drain_rate = map.hp_drain as f32;
    map_data.difficulty.overall_difficulty = map.overall_difficulty as f32;
    map_data.tags.push("ruso-map".to_string());

    let audio_closure = audio_path.clone();
    let audio_thread = std::thread::spawn(move || {
        generate_audio(&audio_closure, rate)?;
        Ok::<(), anyhow::Error>(())
    });

    for h in &mut map_data.hit_objects{
        h.start_time.0 = (*h.start_time as f64 / rate).round() as i32;
        match &mut h.kind {
            HitObjectKind::Hold(k) => {
                k.end_time.0 = (*k.end_time as f64 / rate).round() as i32;
            },
            HitObjectKind::Spinner(k) => {
                k.end_time.0 = (*k.end_time as f64 / rate).round() as i32;
            },
            _ => {}
        }
    }

    for t in &mut map_data.timing_points{
        t.time.0 = (t.time.0 as f64 / rate).round() as i32;
    }

    let new_path = path.parent().unwrap().join(path.file_stem().unwrap());
    write!(File::create(format!("{}({}).osu", new_path.display(), rate))?,"{}", map_data)?;
    if let Err(e) = audio_thread.join(){
        return Err(anyhow::anyhow!("Error generating audio file: {:?}", e))
    }

    writeln!(cache_file, "{}({}).osu", new_path.display(), rate)?;
    writeln!(cache_file, "{}({}).{}", audio_path.parent().unwrap().join(audio_path.file_stem().unwrap()).display(), rate, audio_path.extension().unwrap().to_str().unwrap())?;

    Ok(())
}

fn generate_audio(audio_path: &Path, rate: f64) -> Result<()>{
    gst::init()?;
    let final_path = audio_path.parent().unwrap().join(audio_path.file_stem().unwrap()).join(audio_path.extension().unwrap());

    let pipeline_description = match audio_path.extension().unwrap().to_str().unwrap().to_lowercase().as_str(){
        "mp3" => format!(
           "filesrc location=\"{}\" ! mpegaudioparse ! mpg123audiodec ! decodebin ! audioconvert ! audioresample ! speed speed={} ! audioconvert ! audioresample ! lamemp3enc target=quality quality=0 ! id3v2mux ! filesink location=\"{}\"",
           &audio_path.display(),
           &rate,
           &final_path.display()
        ),
        "ogg" => format!(
           "filesrc location=\"{}\" ! oggdemux ! vorbisdec ! audioconvert ! speed speed={} ! vorbisenc ! oggmux ! filesink location=\"{}\"",
           &audio_path.display(),
           &rate,
           &final_path.display()
        ),
        "wav" => format!(
           "filesrc location=\"{}\" ! wavparse ! audioconvert ! audioresample ! speed speed={} ! audioconvert ! wavenc ! filesink location=\"{}\"",
           &audio_path.display(),
           &rate,
           &final_path.display()
        ),
        e => return Err(anyhow::anyhow!("Unsupported file type: {}", e))
    };
    
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

pub fn clean_maps() -> Result<()>{
    let cache = match dirs::cache_dir(){
        Some(k) => k,
        None => return Err(anyhow::anyhow!("Couldn't find cache directory"))
    }.join("ruso");
    let mut cache_file = match File::open(cache.join("ruso-map").join("maps.txt")){
        Ok(k) => k,
        Err(e) if e.kind() == ErrorKind::NotFound => return Err(anyhow::anyhow!("No maps to clean")),
        Err(e) => return Err(anyhow::anyhow!("Error opening maps.txt: {}", e))
    };
    let mut buf = String::new();
    cache_file.read_to_string(&mut buf)?;
    for line in buf.lines(){
        let path = PathBuf::from(line);
        if !path.exists(){
            continue;
        }
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub fn round_dec(x: f64, decimals: u32) -> f64 {
    let y = 10i32.pow(decimals) as f64;
    (x * y).round() / y
}

pub async fn gosu_websocket_listen(settings: &Settings) -> Result<()>{
    let (mut socket, response) = connect_async(&settings.websocket_url).await?;
    if response.status().is_success(){
        println!("Connected to websocket");
    }
    let (_, mut read) = socket.split();
    let recent_state: Arc<Mutex<serde_json::Value>> = Arc::new(Mutex::new(from_str(&read.next().await.unwrap()?.into_text()?)?));
    println!("{}", recent_state.lock().await["menu"]["bm"]["path"]["file"]);
    let read_future = read.for_each(|message| async{
        let data: serde_json::Value = from_str(&message.unwrap().into_text().unwrap()).unwrap();
        let mut state = recent_state.lock().await;
        if (*state)["menu"]["bm"]["path"]["file"] != data["menu"]["bm"]["path"]["file"]{
            tokio::io::stdout().write_all(data.to_string().as_bytes()).await.unwrap();
            *state = data;
        }
    });
    read_future.await;

    Ok(())
}


#[cfg(test)]
mod test{
    use super::*;
    // #[tokio::test]
    // async fn test1(){
    //     generate_map(&PathBuf::from("/home/cyan/.local/share/osu-wine/osu!/Songs/991895 Kondo Koji - Slider/Kondo Koji - Slider (NikoSek) [YaHoo!!].osu"), 1.9).await.unwrap();
    // }
    // #[tokio::test]
    // async fn test2(){
    //     generate_map(&PathBuf::from("/home/cyan/.local/share/osu-wine/osu!/Songs/1869337 Fellowship - Glory Days/Fellowship - Glory Days (EdgyKing) [Selfless Journey].osu"), 3.0).await.unwrap();
    // }
    #[tokio::test]
    async fn gosu_test(){
        gosu_websocket_listen(&Settings::new()).await.unwrap();
    }
}

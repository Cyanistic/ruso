use std::{path::{PathBuf, Path}, fs::{File, OpenOptions}, io::{Write, ErrorKind, Read, BufWriter}, sync::Arc, process, any::Any};
use anyhow::{Result, anyhow};
use libosu::{prelude::*, events::Event::Background};
use std::process::Child;
use tokio_tungstenite::connect_async;
use tokio::{io::AsyncWriteExt, sync::Mutex};
use futures_util::StreamExt;
use serde_json::from_str;
pub mod structs;
pub mod audio;
pub mod props;
pub mod cli;
pub use structs::{MapOptions, Settings};
use audio::*;

pub fn read_map_metadata(options: MapOptions, settings: &Settings) -> Result<MapOptions>{
    let map = libosu::beatmap::Beatmap::parse(File::open(settings.songs_path.join(&options.map_path))?)?;
    let mut new_options = MapOptions{
        approach_rate: map.difficulty.approach_rate as f64,
        overall_difficulty: map.difficulty.overall_difficulty as f64,
        circle_size: map.difficulty.circle_size as f64,
        hp_drain: map.difficulty.hp_drain_rate as f64,
        bpm: calculate_bpm(&map.timing_points),
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
        title: map.title.into(),
        artist: map.artist.into(),
        difficulty_name: map.difficulty_name.into(),
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

pub fn generate_map(map: &MapOptions, settngs: &Settings) -> Result<()>{
    let path = &settngs.songs_path.join(&map.map_path);
    let rate = map.rate;
    let map_file = File::open(path)?;
    let mut map_data = libosu::beatmap::Beatmap::parse(map_file)?;
    let audio_path = path.parent().unwrap().join(&map_data.audio_filename);
    let cache_dir = match dirs::cache_dir(){
        Some(k) => k,
        None => return Err(anyhow::anyhow!("Couldn't find cache directory"))
    }.join("ruso");
    if !cache_dir.exists(){
        std::fs::create_dir_all(&cache_dir)?;
    }
    let mut cache_file = match OpenOptions::new().append(true).open(cache_dir.join("maps.txt")){
        Ok(k) => k,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let mut temp = File::create(cache_dir.join("maps.txt"))?;
            writeln!(temp, "// Files generated by ruso")?;
            writeln!(temp, "// Do not delete this file as it is used to keep track of files generated by ruso for easy removal if needed")?;
            writeln!(temp, "// For safety reasons, ruso only removes files that start with your current osu! songs path")?;
            temp
        },
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
        generate_audio(&audio_closure, rate)
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

    for point in &mut map_data.timing_points{
        point.time.0 = (point.time.0 as f64 / rate).round() as i32;
        if let TimingPointKind::Uninherited(point) = &mut point.kind{
            point.mpb /= rate;
        }
    
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

// fn generate_audio(audio_path: &Path, rate: f64) -> Result<()>{
//     gst::init()?;
//     let final_path = format!("{}({}).{}",audio_path.parent().unwrap().join(audio_path.file_stem().unwrap()).display(), rate, audio_path.extension().unwrap().to_str().unwrap());
//     let pipeline_description = match audio_path.extension().unwrap().to_str().unwrap().to_lowercase().as_str(){
//         "mp3" => format!(
//            "filesrc location=\"{}\" ! mpegaudioparse ! mpg123audiodec ! decodebin ! audioconvert ! audioresample ! speed speed={} ! audioconvert ! audioresample ! lamemp3enc target=quality quality=0 ! id3v2mux ! filesink location=\"{}\"",
//            &audio_path.display(),
//            &rate,
//            &final_path
//         ),
//         "ogg" => format!(
//            "filesrc location=\"{}\" ! oggdemux ! vorbisdec ! audioconvert ! speed speed={} ! vorbisenc ! oggmux ! filesink location=\"{}\"",
//            &audio_path.display(),
//            &rate,
//            &final_path
//         ),
//         "wav" => format!(
//            "filesrc location=\"{}\" ! wavparse ! audioconvert ! audioresample ! speed speed={} ! audioconvert ! wavenc ! filesink location=\"{}\"",
//            &audio_path.display(),
//            &rate,
//            &final_path
//         ),
//         e => return Err(anyhow::anyhow!("Unsupported file type: {}", e))
//     };
//     
//     let pipeline = gst::parse_launch(&pipeline_description)?;
//     pipeline.set_state(gst::State::Playing)?;
//     let bus = pipeline.bus().unwrap();
//     bus.add_signal_watch();
//         loop {
//             if bus.pop().is_some_and(|x| x.as_ref().type_().eq(&MessageType::Eos)){
//                 break;
//             }
//         }
//     pipeline.set_state(gst::State::Null)?;
//     Ok(())
// }

fn generate_audio(audio_path: &PathBuf, rate: f64) -> Result<()>{
    let final_path = PathBuf::from(format!("{}({}).{}",
        audio_path.parent().unwrap_or(Path::new("")).join(audio_path.file_stem().ok_or(anyhow!("Couldn't find the file stem for audio file"))?).display(),
        rate,
        audio_path.extension().ok_or(anyhow!("Invalid audio file extension"))?.to_str().unwrap()));
    
    if !final_path.exists(){
        match audio_path.extension().unwrap_or(std::ffi::OsStr::new("")).to_str().unwrap(){
            "ogg" => change_speed_ogg(audio_path, rate)?,
            "wav" => change_speed_wav(audio_path, rate)?,
            "mp3" => change_speed_mp3(audio_path, rate)?,
            _ => {
                 if change_speed_mp3(audio_path, rate).is_err(){
                    return Err(anyhow!("Unsupported/unknown file type!"))
                }
            }
        };
    }

    Ok(())
}

pub fn change_map_difficulty(map: &MapOptions, settings: &Settings) -> Result<()>{
    let path = settings.songs_path.join(&map.map_path);
    let map_file = File::open(&path)?;
    let mut map_data = libosu::beatmap::Beatmap::parse(map_file)?;
    let cache_dir = match dirs::cache_dir(){
        Some(k) => k,
        None => return Err(anyhow::anyhow!("Couldn't find cache directory"))
    }.join("ruso");
    if !cache_dir.exists(){
        std::fs::create_dir_all(&cache_dir)?;
    }
    let mut cache_file = match OpenOptions::new().append(true).open(cache_dir.join("maps.txt")){
        Ok(k) => k,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let mut temp = File::create(cache_dir.join("maps.txt"))?;
            writeln!(temp, "// Files generated by ruso")?;
            writeln!(temp, "// Do not delete this file as it is used to keep track of files generated by ruso for easy removal if needed")?;
            writeln!(temp, "// For safety reasons, ruso only removes files that start with your current osu! songs path")?;
            temp
        },
        Err(e) => return Err(anyhow::anyhow!("Error opening maps.txt: {}", e))
    };

    map_data.difficulty_name += format!(" (AR {} CS {} HP {} OD {})", map.approach_rate, map.circle_size, map.hp_drain, map.overall_difficulty ).as_str();
    map_data.difficulty.approach_rate = map.approach_rate as f32;
    map_data.difficulty.circle_size = map.circle_size as f32;
    map_data.difficulty.hp_drain_rate = map.hp_drain as f32;
    map_data.difficulty.overall_difficulty = map.overall_difficulty as f32;
    map_data.tags.push("ruso-map".to_string());

    let new_path = format!("{} (AR {} CS {} HP {} OD {})", path.parent().unwrap().join(path.file_stem().unwrap()).display(),
        map.approach_rate,
        map.circle_size,
        map.hp_drain,
        map.overall_difficulty);

    write!(File::create(&new_path)?, "{}", map_data)?;
    writeln!(cache_file, "{}", new_path)?;

    Ok(())
}

pub fn calculate_bpm(points: &[TimingPoint]) -> usize{
    (60000.0 / points.iter().filter_map(|x| match &x.kind{
        TimingPointKind::Uninherited(k) => Some(k.mpb.abs()),
        _ => None
    }).max_by(|x,y| x.partial_cmp(y).unwrap()).unwrap_or(100.0)).round() as usize
}

pub fn clean_maps(settings: &Settings) -> Result<usize>{
    let cache = match dirs::cache_dir(){
        Some(k) => k,
        None => return Err(anyhow::anyhow!("Couldn't find cache directory"))
    }.join("ruso");

    let file_contents = match std::fs::read_to_string(cache.join("maps.txt")){
        Ok(k) => k,
        Err(e) if e.kind() == ErrorKind::NotFound => return Err(anyhow::anyhow!("No maps to clean")),
        Err(e) => return Err(anyhow::anyhow!("Error opening maps.txt: {}", e))
    };

    let undeleted = OpenOptions::new().write(true).open(cache.join("maps.txt"))?;
    let mut writer = BufWriter::new(undeleted);
    let mut cleaned: usize = 0;

    for line in file_contents.lines(){
        let path: PathBuf;
        if let Some(ind) = line.find("//"){
            path = PathBuf::from(line[..ind].trim());
        }else{
            path = PathBuf::from(line);
        }
        if !path.exists() || !path.starts_with(&settings.songs_path){
            writeln!(writer, "{}", line)?;
        }else{
            std::fs::remove_file(path)?;
            cleaned += 1;
        }
    }
    writer.flush()?;
    Ok(cleaned)
}

pub fn round_dec(x: f64, decimals: u32) -> f64 {
    let y = 10i32.pow(decimals) as f64;
    (x * y).round() / y
}

pub async fn gosu_websocket_listen(settings: &Settings) -> Result<()>{
    let (socket, response) = connect_async(&settings.websocket_url).await?;
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

#[cfg(target_os = "linux")]
pub fn gosu_startup(settings: &Settings) -> Result<Child>{
    use std::{process::Command, io::{IsTerminal, stderr}};

    if settings.gosumemory_path.is_file(){
        if settings.songs_path.is_dir(){
            if std::io::stdin().is_terminal(){
                eprintln!("gosumemory requires root permissions to read /proc on linux");
                stderr().flush()?;

                // Spawn a dummy command to get the sudo password prompt out of the way
                let mut dummy = Command::new("sudo")
                .args(["sleep", "0"])
                .spawn()?;
                dummy.wait()?;

                // Spawn the actual gosumemory process
                Command::new("sudo")
                .args([settings.gosumemory_path.to_str().unwrap(), "--path", settings.songs_path.to_str().unwrap()])
                .spawn().map_err(|e| anyhow::anyhow!("Error starting gosumemory: {}", e))
            }else{
                Command::new("pkexec")
                .args([settings.gosumemory_path.to_str().unwrap(), "--path", settings.songs_path.to_str().unwrap()])
                .stderr(process::Stdio::piped())
                .stdout(process::Stdio::piped())
                .stdin(process::Stdio::piped())
                .spawn().map_err(|e| anyhow::anyhow!("Error starting gosumemory: {}", e))
            }
        }else{
            Err(anyhow::anyhow!("Songs path not found"))
        }
    }else{
        Err(anyhow::anyhow!("gosumemory executable not found"))
    }
}

#[cfg(not(target_os = "linux"))]
pub fn gosu_startup(settings: &Settings) -> Result<()>{
    use std::process::Command;
    if settings.gosumemory_path.is_file(){
        if settings.songs_path.is_dir() {
            Command::new(settings.gosumemory_path.to_str().unwrap())
            .args(["--path", settings.songs_path.to_str().unwrap()])
            .spawn()?;
        }else{
            return Err(anyhow::anyhow!("Songs path not found"))
        }
    }else{
        return Err(anyhow::anyhow!("gosumemory executable not found"))
    }
    Ok(())
}

pub fn write_config(settings: &Settings) -> Result<()>{
    let config_path = dirs::config_dir().unwrap().join("ruso");
    if !config_path.exists(){
        std::fs::create_dir_all(&config_path)?;
    }
    let mut config_file = File::create(config_path.join("settings.json"))?;
    let config_json = serde_json::to_string_pretty(settings)?;
    write!(config_file, "{}", config_json)?;
    Ok(())
}


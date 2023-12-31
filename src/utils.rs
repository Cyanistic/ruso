use std::{path::{PathBuf, Path}, fs::{File, OpenOptions}, io::{Write, ErrorKind, BufWriter}, sync::Arc, process, collections::HashSet};
use anyhow::{Result, anyhow};
use libosu::prelude::*;
use std::process::Child;
use tokio_tungstenite::connect_async;
use tokio::{io::AsyncWriteExt, sync::Mutex};
use futures_util::StreamExt;
use serde_json::from_str;
use crate::{structs::{MapOptions, Settings}, audio::*};


/// Generates an audio and .osu file using the given Settings and MapOptions structs.
pub async fn generate_map(map: &MapOptions, settings: &Settings) -> Result<()>{
    let path = &settings.songs_path.join(&map.map_path);
    let rate = map.rate;
    let map_file = File::open(path)?;
    let mut map_data = libosu::beatmap::Beatmap::parse(map_file)?;
    let audio_path = path.parent().unwrap().join(&map_data.audio_filename);
    let cache_dir = dirs::cache_dir().ok_or(anyhow!("Couldn't find cache directory"))?.join("ruso");
    if !cache_dir.exists(){
        std::fs::create_dir_all(&cache_dir)?;
    }

    // Open the cache file to append new maps or create a new one with help info
    let mut cache_file = match OpenOptions::new().append(true).open(cache_dir.join("maps.txt")){
        Ok(k) => k,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let mut temp = OpenOptions::new().create(true).append(true).open(cache_dir.join("maps.txt"))?;
            writeln!(temp, "// Files generated by ruso")?;
            writeln!(temp, "// Do not delete this file as it is used to keep track of files generated by ruso for easy removal if needed")?;
            writeln!(temp, "// For safety reasons, ruso only removes files that start with your current osu! songs path")?;
            temp
        },
        Err(e) => return Err(anyhow!("Error opening maps.txt: {}", e))
    };
    map_data.difficulty.approach_rate = map.approach_rate;
    map_data.difficulty.circle_size = map.circle_size;
    map_data.difficulty.hp_drain_rate = map.hp_drain;
    map_data.difficulty.overall_difficulty = map.overall_difficulty;
    map_data.preview_time.0 = (*map_data.preview_time as f64 / rate).round() as i32;
    map_data.tags.push("ruso-map".to_string());

    // Change beatmap properties to match those given by the user
    let mut new_audio_path = audio_path.clone();
    if rate != 1.0{
        let new_name = format!("{}({}).{}", &audio_path.file_stem().unwrap().to_str().unwrap(), rate, audio_path.extension().unwrap().to_str().unwrap());
        new_audio_path.set_file_name(&new_name);
        map_data.audio_filename = new_name;
        map_data.difficulty_name += format!(" {}x ({}bpm)", rate, (map.bpm as f64 * rate) as usize).as_str(); 
    }else{
        map_data.difficulty_name += format!(" (AR {} CS {} HP {} OD {})", map.approach_rate, map.circle_size, map.hp_drain, map.overall_difficulty ).as_str();
    }

    let mut audio_thread = None;
    if settings.force_generation || !new_audio_path.exists(){
        // Generate audio file on a new thread
        audio_thread = Some(tokio::task::spawn({
            let audio_path = audio_path.clone();
            let change_pitch = settings.change_pitch;
            async move{
                generate_audio(&audio_path, rate, change_pitch)
            }
        }));
    }

    // Change time value for each hit object to match the new rate of the map
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

    // Change time value for each timing point to match the new rate of the map
    for point in &mut map_data.timing_points{
        point.time.0 = (point.time.0 as f64 / rate).round() as i32;
        if let TimingPointKind::Uninherited(point) = &mut point.kind{
            point.mpb /= rate;
        }
    }

    // Generate path for the new .osu file
    let new_path = PathBuf::from(format!("{}({}).osu", path.parent().unwrap().join(path.file_stem().unwrap()).display(), rate));

    // Wait for the audio threat to finish and return an error if something went wrong
    if let Some(audio_thread) = audio_thread{
        audio_thread.await.map_err(|e| anyhow::anyhow!("Error generating audio file: {:?}", e))??;
    } 
    
    // Generate .osz file or .osu depending on user selection
    if settings.generate_osz{
        generate_osz(&new_path, &map_data)?;
    }else{
        write!(File::create(&new_path)?,"{}", map_data)?;
    }

    // Write the new paths to the cache file for easy deletion and space usage calculation
    writeln!(cache_file, "{}", new_path.display())?;
    writeln!(cache_file, "{}", new_audio_path.display())?;

    Ok(())
}

/// Generates a new audio file with the given rate.
fn generate_audio(audio_path: &PathBuf, rate: f64, change_pitch: bool) -> Result<()>{
    // Generate audio file based on extension
    match audio_path.extension().unwrap_or(std::ffi::OsStr::new("")).to_str().unwrap(){
        "ogg" => change_speed_ogg(audio_path, rate, change_pitch)?,
        "wav" => change_speed_wav(audio_path, rate, change_pitch)?,
        "mp3" => change_speed_mp3(audio_path, rate, change_pitch)?,
        _ => {
            // Attempt to process file as mp3 if it is not a known file type
            if change_speed_mp3(audio_path, rate, change_pitch).is_err(){
                return Err(anyhow!("Unsupported/unknown file type!"))
            }
        }
    };

    Ok(())
}

/// Calculates the bpm of beatmap using the timing points.
pub fn calculate_bpm(points: &[TimingPoint]) -> usize{
    (60000.0 / points.iter().filter_map(|x| match &x.kind{
        TimingPointKind::Uninherited(k) => Some(k.mpb.abs()),
        _ => None
    }).min_by(|x,y| x.partial_cmp(y).unwrap()).unwrap_or(100.0)).round() as usize
}

/// Removes all files generated by ruso.
pub fn clean_maps(settings: &Settings) -> Result<usize>{
    let cache = dirs::cache_dir().ok_or(anyhow!("Couldn't find cache directory"))?.join("ruso");

    let file_contents = match std::fs::read_to_string(cache.join("maps.txt")){
        Ok(k) => k,
        Err(e) if e.kind() == ErrorKind::NotFound => return Err(anyhow::anyhow!("No maps to clean")),
        Err(e) => return Err(anyhow!("Error opening maps.txt: {}", e))
    };

    let undeleted = OpenOptions::new().write(true).open(cache.join("maps.txt"))?;
    let mut writer = BufWriter::new(undeleted);
    let mut cleaned: usize = 0;

    for line in file_contents.lines(){
        let path: PathBuf;
        // Treat "//" as the start of a single-line comment and ignore everything after it 
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

/// Calculates space used by all files generated by ruso and saves it to a cache file.
pub fn calculate_space(file_name: &str) -> Result<usize>{
    let cache = dirs::cache_dir().ok_or(anyhow!("Couldn't find cache directory"))?.join("ruso");

    let file_contents = match std::fs::read_to_string(cache.join("maps.txt")){
        Ok(k) => k,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(anyhow!("Error opening maps.txt: {}", e))
    };
    let mut files: HashSet<PathBuf> = HashSet::with_capacity(file_contents.lines().count());

    for line in file_contents.lines(){
        let path: PathBuf;
        if let Some(ind) = line.find("//"){
            path = PathBuf::from(line[..ind].trim());
        }else{
            path = PathBuf::from(line);
        }
        files.insert(path);
    }

    let used_space: usize = files.into_iter().map(|x| match x.metadata(){
        Ok(k) => k.len() as usize,
        Err(_) => 0
    }).sum();

    let mut cache_file = match OpenOptions::new().write(true).open(cache.join(file_name)){
        Ok(k) => k,
        Err(e) if e.kind() == ErrorKind::NotFound => File::create(cache.join(file_name))?,
        Err(e) => return Err(anyhow!("Error opening used_space.txt: {}", e))
    };

    cache_file.write_all(used_space.to_string().as_bytes())?;

    Ok(used_space)
}

/// Reads the cached space used by files generated by ruso.
pub fn read_space(file_name: &str) -> Result<usize>{
    let cache = dirs::cache_dir().ok_or(anyhow!("Couldn't find cache directory"))?.join("ruso");
    
    Ok(match std::fs::read_to_string(cache.join(file_name)){
        Ok(k) => k.parse::<usize>()?,
        Err(e) if e.kind() == ErrorKind::NotFound => 0,
        Err(e) => return Err(anyhow!("Error reading used space cache: {}", e))
    })
}

/// Rounds a float to the given number of decimal places.
pub fn round_dec(x: f64, decimals: u32) -> f64 {
    let y = 10i32.pow(decimals) as f64;
    (x * y).round() / y
}

/// Connects to gosumemory with the settings from the Settings struct.
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

/// Starts gosumemory with the settings from the Settings struct.
#[cfg(target_os = "linux")]
pub fn gosu_startup(settings: &Settings) -> Result<Child>{
    use std::{process::Command, io::{IsTerminal, stderr}};

    if settings.gosumemory_path.is_file(){
        if settings.songs_path.is_dir(){
            // Check if the user is root and use sudo if they are not
            if unsafe{ libc::getuid() } != 0{
                if std::io::stdin().is_terminal(){
                    eprintln!("gosumemory requires root permissions to read /proc on linux");
                    stderr().flush()?;

                    // Spawn a dummy command to get the sudo password prompt out of the way and abuse the
                    // sudo password cooldown to spawn the actual command
                    // This is a hack but I couldn't find another way to do it without increasing
                    // complexity
                    let mut dummy = Command::new("sudo")
                    .args(["sleep", "0"])
                    .spawn()?;
                    dummy.wait()?;

                    // Spawn the actual gosumemory process
                    Command::new("sudo")
                    .args([settings.gosumemory_path.to_str().ok_or(anyhow!("Could not convert the gosumemory path to a UTF-8 string."))?, "--path", settings.songs_path.to_str().ok_or(anyhow!("Could not convert the gosumemory path to a UTF-8 string."))?])
                    .spawn().map_err(|e| anyhow!("Error starting gosumemory: {}", e))
                }else{
                    Command::new("pkexec")
                    .args([settings.gosumemory_path.to_str().ok_or(anyhow!("Could not convert the gosumemory path to a UTF-8 string."))?, "--path", settings.songs_path.to_str().ok_or(anyhow!("Could not convert the gosumemory path to a UTF-8 string."))?])
                    .stderr(process::Stdio::piped())
                    .stdout(process::Stdio::piped())
                    .stdin(process::Stdio::piped())
                    .spawn().map_err(|e| anyhow!("Error starting gosumemory: {}", e))
                }
            }else{
                Command::new(settings.gosumemory_path.to_str().ok_or(anyhow!("Could not convert the gosumemory path to a UTF-8 string."))?)
                .args(["--path", settings.songs_path.to_str().ok_or(anyhow!("Could not convert the gosumemory path to a UTF-8 string."))?])
                .spawn().map_err(|e| anyhow!("Error starting gosumemory: {}", e))
            }
        }else{
            Err(anyhow!("Songs path not found"))
        }
    }else{
        Err(anyhow!("gosumemory executable not found.\nMake sure that you set \"gosumemory_path\" in {}", dirs::config_dir().ok_or(anyhow!("Error: could not find config directory"))?.join("ruso").join("settings.json").display()))
    }
}

#[cfg(not(target_os = "linux"))]
pub fn gosu_startup(settings: &Settings) -> Result<Child>{
    use std::process::Command;
    if settings.gosumemory_path.is_file(){
        if settings.songs_path.is_dir() {
            Ok(Command::new(settings.gosumemory_path.to_str().ok_or(anyhow!("Could not convert the gosumemory path to a UTF-8 string."))?)
            .args(["--path", settings.songs_path.to_str().ok_or(anyhow!("Could not convert the osu! songs path to a UTF-8 string."))?])
            .spawn()?)
        }else{
            Err(anyhow!("Songs path not found"))
        }
    }else{
        Err(anyhow!("gosumemory executable not found"))
    }
}

pub fn write_config(settings: &Settings, file_name: &str) -> Result<()>{
    let config_path = dirs::config_dir().ok_or(anyhow!("Error: could not find config directory"))?.join("ruso");
    if !config_path.exists(){
        std::fs::create_dir_all(&config_path)?;
    }
    let mut config_file = BufWriter::new(File::create(config_path.join(file_name))?);
    let config_json = serde_json::to_string_pretty(settings)?;
    write!(config_file, "{}", config_json)?;
    Ok(())
}

/// Generates an .osz file from an .osu file.
pub fn generate_osz(map_path: &Path, map_data: &Beatmap) -> Result<()>{
    let osz_file = File::create(map_path.parent().ok_or(anyhow!("Couldn't get parent path."))?.with_extension("osz"))?;
    let mut zip = zip::ZipWriter::new(BufWriter::new(osz_file));
    zip.start_file(map_path.file_name()
        .ok_or(anyhow!("Couldn't get file name."))?.to_str()
        .ok_or(anyhow!("Couldn't convert file name to a UTF-8 string."))?, Default::default())?;
    map_data.write(&mut zip)?;

    zip.finish()?;

    Ok(())
}

/// Generates an example theme file with the osu! theme colors.
pub fn generate_example_theme(file_name: &str) -> Result<()>{
    let config = match dirs::config_dir(){
        Some(k) => k,
        None => return Err(anyhow::anyhow!("Couldn't find cache directory"))
    }.join("ruso");
    let mut example_file = BufWriter::new(OpenOptions::new().create_new(true).write(true).open(config.join(file_name))?);
    write!(example_file, r#"
:root{{
  --primary: #262335;
  --secondary: #FF66AA;
  --text-primary: #A79AE9;
  --text-secondary: #FFFFFF;
}}"#)?;
    Ok(())
}


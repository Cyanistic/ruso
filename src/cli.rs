use std::{process::{exit, Command}, path::PathBuf, io::{stdout, IsTerminal, stderr, Write}, time::Duration};

use anyhow::{Result, anyhow};
use futures_util::StreamExt;
use crate::{MapOptions, Settings, generate_map, gosu_startup, read_map_metadata, round_dec};
use serde_json::Value;
use tokio_tungstenite::connect_async;

/// Entry point for cli
pub async fn run() -> Result<()>{
    // Reset the SIGPIPE handler to the default one to allow for proper unix piping
    reset_sigpipe();

    let args = Vec::from_iter(std::env::args());
    let mut args = args.iter().skip(1).map(AsRef::as_ref).collect::<Vec<&str>>();
    const AVAILABLE_COMMANDS: [&str; 22] = [
        "-a", "--approach-rate",
        "-b", "--bpm",
        "-c", "--circle-size",
        "-d", "--hp-drain",
        "-h", "--help",
        "-g", "--gosumemory",
        "-o", "--overall-difficulty",
        "-p", "--path",
        "-r", "--rate",
        "-V", "--version",
        "-z", "+z"
    ];

    const FLAGS: [&str; 8] = [
        "-h", "--help",
        "-V", "--version",
        "-g", "--gosumemory",
        "-z", "+z"
    ];
    
    // Add empty string after argument if the argument is a flag for easier error handling
    { let mut count: usize = 0;
        for (ind, val) in args.clone().iter().enumerate(){ 
            if FLAGS.contains(val){
                args.insert(ind+count+1, "");
                count += 1;
            }
        }
    }

    // Check if the provided command args are valid
    for arg in args.clone().iter().step_by(2){
        if !AVAILABLE_COMMANDS.contains(arg){
            return Err(anyhow!("Invalid command: {}", arg));
        }
    }

    //Create a new map and settings instance
    let mut map = MapOptions::new();
    let mut settings = Settings::new_from_config();
    
    // Set all locks to false to allow user to change them
    settings.ar_lock = false;
    settings.cs_lock = false;
    settings.hp_lock = false;
    settings.od_lock = false;

    let mut bpm: Option<usize> = None;

    // Iterate over each argument and apply the respective changes to the map
    // Stepping by 2 since args are in the format: [command, value]
    let mut gosu_process: Option<std::process::Child> = None;
    for ind in (0..args.len()).step_by(2){
        match args[ind]{
            "-a"| "--approach-rate" => {
                map.approach_rate = args[ind+1].parse::<f64>()?;
                settings.ar_lock = true;
            },
            "-b"| "--bpm" => bpm = Some(args[ind+1].parse::<usize>()?),
            "-c"| "--circle-size" => {
                map.circle_size = args[ind+1].parse::<f64>()?;
                settings.cs_lock = true;
            },
            "-d"| "--hp-drain" => {
                map.hp_drain = args[ind+1].parse::<f64>()?;
                settings.hp_lock = true;
            },
            "-h"| "--help" => {
                print_help();
                exit(0);
            },
            "-g"| "--gosumemory" => gosu_process = match gosu_startup(&settings){
                Ok(process) => {
                    tokio::select!{
                        _ = poll_gosu(&settings) => (),
                        _ = tokio::time::sleep(Duration::from_secs(5)) => {
                            writeln!(stderr(), "No response from {} after 5 seconds, continuing...", settings.websocket_url)?;
                            stderr().flush()?;
                        },
                    };

                    // Fix terminal to avoid staircase effect
                    if let Ok(mut process) = Command::new("stty").args(["opost", "onlcr"]).spawn(){
                        process.wait()?;
                    }
                    Some(process)
                },
                Err(e) => return Err(anyhow!("Could not start gosumemory: {}", e))
            },
            "-o"| "--overall-difficulty" => {
                map.overall_difficulty = args[ind+1].parse::<f64>()?;
                settings.od_lock = true;
            },
            "-p"| "--path" => {
                let temp_path: PathBuf = args[ind+1].into();
                if temp_path.exists(){
                    map.map_path = temp_path;
                }else if settings.songs_path.join(&temp_path).exists(){
                    map.map_path = settings.songs_path.join(temp_path);
                }else{
                    return Err(anyhow!("The provided path: '{}' does not exist", temp_path.display()));
                }
            },
            "-r"| "--rate" => map.rate = args[ind+1].parse::<f64>()?,
            "-V"| "--version" => {
                println!("Ruso v{}", env!("CARGO_PKG_VERSION"));
                exit(0);
            }
            "-z" => settings.generate_osz = false,
            "+z" => settings.generate_osz = true,
            _ => return Err(anyhow!("Invalid command: {}", args[ind]))
        }
    }

    // Attempt to get the path from the gosu websocket url if no path was provided
    if map.map_path == PathBuf::new(){
        writeln!(stderr(), "No path specified, attempting to get path from gosu!")?;
        map.map_path = match path_from_gosu(&settings).await{
            Ok(path) => path,
            Err(e) => return Err(anyhow!("Could not connect to gosu: {}", e))
        };
        writeln!(stderr(), "Got path from gosu: {}", map.map_path.display())?;
    }

    // Kill gosumemory if it was started by ruso
    if let Some(mut process) = gosu_process{
        #[cfg(not(unix))]
        process.kill().unwrap_or_else(|_| {writeln!(stderr(), "Could not kill spawned gosumemory process");});

        #[cfg(unix)]
        unsafe{
            libc::kill(process.id() as i32, libc::SIGTERM);
        }
    }

    // Get metadata for the map and set its rate based on
    // bpm if it was provided
    map = read_map_metadata(map, &settings)?;
    if let Some(bpm) = bpm{
        map.rate = round_dec(bpm as f64/map.bpm as f64, 2);
    }

    // Making the generate_map function generate the path only from map in order to avoid conflicts
    // with paths in cwd and paths that start with the provided osu! songs path.
    settings.songs_path = PathBuf::new();
    writeln!(stderr(), "Generating map...")?;
    generate_map(&map, &settings)?;

    // Fix terminal carriage return
    if let Ok(mut process) = Command::new("stty").arg("sane").spawn(){
        process.wait()?;
    }

    writeln!(stderr(), "Map successfully generated!")?;
    Ok(())
}

fn print_help(){
    const BOLD: &str = "\x1b[1m";
    const UND: &str = "\x1b[4m";
    const RES: &str = "\x1b[0m";

    if stdout().is_terminal(){
        println!("{}Generates osu! maps based on given args.", BOLD);
        println!("{}Running with no arguments runs the GUI version.", BOLD);
        println!("{}{}Usage:{}{} ruso [OPTIONS]{}\n", BOLD, UND, RES, BOLD, RES);
        println!("{}{}OPTIONS:{}", BOLD, UND, RES);
        println!("  {}-h, --help                      {}Print the help information and exit.", BOLD, RES);
        println!("  {}-V, --version                   {}Print version and exit.", BOLD, RES);
        println!("  {}-a, --approach-rate      [AR]   {}The approach rate of the map. Will remain unchanged if not provided.", BOLD, RES);
        println!("  {}-b, --bpm                [BPM]  {}The new bpm of the map. This will override --rate if provided.", BOLD, RES);
        println!("  {}-c, --circle-size        [CS]   {}The circle size of the map. Will remain unchanged if not provided.", BOLD, RES);
        println!("  {}-d, --hp-drain           [HP]   {}The hp drain of the map. Will remain unchanged if not provided.", BOLD, RES);
        println!("  {}-g, --gosumemory                {}Spawn gosumemory as a child process.", BOLD, RES);
        println!("                                    This will use the paths provided in '{}' as the gosumemory and osu! songs path respectively.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
        println!("  {}-o, --overall-difficulty [OD]   {}The overall difficulty of the map. Will remain unchanged if not provided.", BOLD, RES);
        println!("  {}-p, --path               [PATH] {}The path to the osu! map.", BOLD, RES);
        println!("                                    This can be a regular path or a path the osu! songs path provided in '{}' as the root.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
        println!("                                    This is inferred, and the former will take precedence over the latter.");
        println!("                                    If this is not provided, ruso will attempt to connect to a running gosumemory instance with the websocket url provided in '{}'.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
        println!("  {}-r, --rate               [RATE] {}The playback rate (or speed) of the map.", BOLD, RES);
        println!("                                    This will speed up the .osu file and the corresponding audio file.");
        println!("  {}-/+z                            {}Enable (+z) or disable (-z) generation of .osz files.", BOLD, RES);
        println!("                                    This will use the 'generate_osz' value in '{}' if not provided.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
    }else{
        println!("Generates osu! maps based on given args.");
        println!("Running with no arguments runs the GUI version.");
        println!("Usage: ruso [OPTIONS]\n");
        println!("OPTIONS:");
        println!("  -h, --help                      Print the help information and exit.");
        println!("  -V, --version                   Print version and exit.");
        println!("  -a, --approach-rate      [AR]   The approach rate of the map. Will remain unchanged if not provided.");
        println!("  -b, --bpm                [BPM]  The new bpm of the map. This will override --rate if provided.");
        println!("  -c, --circle-size        [CS]   The circle size of the map. Will remain unchanged if not provided.");
        println!("  -d, --hp-drain           [HP]   The hp drain of the map. Will remain unchanged if not provided.");
        println!("  -g, --gosumemory                Spawn gosumemory as a child process.");
        println!("                                  This will use the paths provided in '{}' as the gosumemory and osu! songs path respectively.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
        println!("  -o, --overall-difficulty [OD]   The overall difficulty of the map. Will remain unchanged if not provided.");
        println!("  -p, --path               [PATH] The path to the osu! map.");
        println!("                                  This can be a regular path or a path the osu! songs path provided in '{}' as the root.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
        println!("                                  This is inferred, and the former will take precedence over the latter.");
        println!("                                  If this is not provided, ruso will attempt to connect to a running gosumemory instance with the websocket url provided in '{}'.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
        println!("  -r, --rate               [RATE] The playback rate (or speed) of the map.");
        println!("                                  This will speed up the .osu file and the corresponding audio file.");
        println!("  -/+z                            Enable (+z) or disable (-z) generation of .osz files.");
        println!("                                  This will use the 'generate_osz' value in '{}' if not provided.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
    }
}

async fn path_from_gosu(settings: &Settings) -> Result<PathBuf>{
    let (socket, _) = connect_async(&settings.websocket_url).await?;
    let ( _, mut read) = socket.split();

    match read.next().await{
        Some(message) => {
            let message = message?;
            let json_data: Value = serde_json::from_str(message.to_text()?)?;
            return Ok(PathBuf::from(json_data["settings"]["folders"]["songs"].as_str().unwrap())
                .join(json_data["menu"]["bm"]["path"]["folder"].as_str().unwrap())
                .join(json_data["menu"]["bm"]["path"]["file"].as_str().unwrap()));
        },
        None => Err(anyhow!("No response from gosu!"))
    }
}

async fn poll_gosu(settings: &Settings){
    loop{
        let (socket, _) = match connect_async(&settings.websocket_url).await{
            Ok(k) => k,
            Err(_) => continue
        };
        let ( _, mut read) = socket.split();

            match read.next().await{
                Some(_) => return,
                None => continue
            }
    };
}

#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {
    // no-op
}

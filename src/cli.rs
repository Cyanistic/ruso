use std::{process::exit, path::PathBuf, io::{stdout, IsTerminal, Read, BufReader, BufRead}, time::Duration};

use anyhow::{Result, anyhow};
use futures_util::StreamExt;
use ruso::{MapOptions, Settings, generate_map, gosu_startup};
use serde_json::Value;
use tokio_tungstenite::connect_async;

pub async fn run() -> Result<()>{

    // Reset the SIGPIPE handler to the default one to allow for proper unix piping
    reset_sigpipe();

    let args = Vec::from_iter(std::env::args());
    let mut args = args.iter().skip(1).map(AsRef::as_ref).collect::<Vec<&str>>();
    const AVAILABLE_COMMANDS: [&str; 18] = [
        "-a", "--approach-rate",
        "-c", "--circle-size",
        "-d", "--hp-drain",
        "-h", "--help",
        "-g", "--gosumemory",
        "-o", "--overall-difficulty",
        "-p", "--path",
        "-r", "--rate",
        "-V", "--version",
    ];

    const FLAGS: [&str; 6] = [
        "-h", "--help",
        "-V", "--version",
        "-g", "--gosumemory",
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

    // Iterate over each argument and apply the respective changes to the map
    // Stepping by 2 since args are in the format: [command, value]
    let mut gosu_process: Option<std::process::Child> = None;
    for ind in (0..args.len()).step_by(2){
        match args[ind]{
            "-a"| "--approach-rate" => map.approach_rate = args[ind+1].parse::<f64>()?,
            "-c"| "--circle-size" => map.circle_size = args[ind+1].parse::<f64>()?,
            "-d"| "--hp-drain" => map.hp_drain = args[ind+1].parse::<f64>()?,
            "-h"| "--help" => {
                print_help();
                exit(0);
            },
            "-g"| "--gosumemory" => gosu_process = match gosu_startup(&settings){
                Ok(process) => {
                    std::thread::sleep(Duration::from_secs(1));
                    Some(process)
                },
                Err(e) => return Err(anyhow!("Could not start gosumemory: {}", e))
            },
            "-o"| "--overall-difficulty" => map.overall_difficulty = args[ind+1].parse::<f64>()?,
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
            _ => return Err(anyhow!("Invalid command: {}", args[ind]))
        }
    }

    // Attempt to get the path from the gosu websocket url if no path was provided
    if map.map_path == PathBuf::new(){
        eprintln!("No path specified, attempting to get path from gosu!");
        map.map_path = match path_from_gosu(&settings).await{
            Ok(path) => path,
            Err(e) => return Err(anyhow!("Could not connect to gosu: {}", e))
        };
    }

    // Making the generate_map function generate the path only from map in order to avoid conflicts
    // with paths in cwd and paths that start with the provided osu! songs path.
    settings.songs_path = PathBuf::new();
    generate_map(&map, &settings)?;
    println!("Map successfully generated!");

    // Kill gosumemory if it was started by ruso
    if let Some(mut process) = gosu_process{
        process.kill().unwrap_or_else(|_| eprintln!("Could not kill spawned gosumemory process"));

        #[cfg(target_os = "linux")]
        unsafe{
            libc::kill(process.id() as i32, libc::SIGKILL);
        }
    }
    Ok(())
}

pub fn print_help(){
    const BOLD: &str = "\x1b[1m";
    const UND: &str = "\x1b[4m";
    const RES: &str = "\x1b[0m";

    if stdout().is_terminal(){
        println!("{}Generates osu! maps based on given args.", BOLD);
        println!("{}Running with no arguments runs the GUI version.", BOLD);
        println!("{}{}Usage:{}{} ruso [OPTIONS]{}\n", BOLD, UND, RES, BOLD, RES);
        println!("{}{}OPTIONS:{}", BOLD, UND, RES);
        println!("  {}-h, --help                    {}Print the help information and exit.", BOLD, RES);
        println!("  {}-V, --version                 {}Print version and exit.", BOLD, RES);
        println!("  {}-a, --approach-rate           {}The approach rate of the map. Will remain unchanged if not provided.", BOLD, RES);
        println!("  {}-c, --circle-size             {}The circle size of the map. Will remain unchanged if not provided.", BOLD, RES);
        println!("  {}-d, --hp-drain                {}The hp drain of the map. Will remain unchanged if not provided.", BOLD, RES);
        println!("  {}-g, --gosumemory              {}Spawn gosumemory as a child process.", BOLD, RES);
        println!("                                  This will use the paths provided in '{}' as the gosumemory and osu! songs path respectively.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
        println!("  {}-o, --overall-difficulty      {}The overall difficulty of the map. Will remain unchanged if not provided.", BOLD, RES);
        println!("  {}-p, --path                    {}The path to the osu! map.", BOLD, RES);
        println!("                                  This can be a regular path or a path the osu! songs path provided in '{}' as the root.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
        println!("                                  This is inferred, and the former will take precedence over the latter.");
        println!("                                  If this is not provided, ruso will attempt to connect to a running gosumemory instance with the websocket url provided in '{}'.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
        println!("  {}-r, --rate                    {}The playback rate (or speed) of the map. This will speed up the .osu file and the corresponding audio file.", BOLD, RES);
        println!("                                  This will speed up the .osu file and the corresponding audio file.");
    }else{
        println!("Generates osu! maps based on given args.");
        println!("Running with no arguments runs the GUI version.");
        println!("Usage: ruso [OPTIONS]\n");
        println!("OPTIONS:");
        println!("  -h, --help                    Print the help information and exit.");
        println!("  -V, --version                 Print version and exit.");
        println!("  -a, --approach-rate           The approach rate of the map. Will remain unchanged if not provided.");
        println!("  -c, --circle-size             The circle size of the map. Will remain unchanged if not provided.");
        println!("  -d, --hp-drain                The hp drain of the map. Will remain unchanged if not provided.");
        println!("  -g, --gosumemory              Spawn gosumemory as a child process.");
        println!("                                This will use the paths provided in '{}' as the gosumemory and osu! songs path respectively.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
        println!("  -o, --overall-difficulty      The overall difficulty of the map. Will remain unchanged if not provided.");
        println!("  -p, --path                    The path to the osu! map.");
        println!("                                This can be a regular path or a path the osu! songs path provided in '{}' as the root.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
        println!("                                This is inferred, and the former will take precedence over the latter.");
        println!("                                If this is not provided, ruso will attempt to connect to a running gosumemory instance with the websocket url provided in '{}'.", dirs::config_dir().unwrap().join("ruso").join("settings.json").display());
        println!("  -r, --rate                    The playback rate (or speed) of the map. This will speed up the .osu file and the corresponding audio file.");
        println!("                                This will speed up the .osu file and the corresponding audio file.");
    }
}

pub async fn path_from_gosu(settings: &Settings) -> Result<PathBuf>{
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

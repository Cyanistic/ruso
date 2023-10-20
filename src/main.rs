#![allow(non_snake_case, clippy::redundant_closure)]
#![windows_subsystem = "windows"]
#[allow(unused_imports)]
use std::{process::{Child, Command}, sync::{Arc, Mutex}, path::PathBuf, io::Write};
use dioxus::prelude::*;
use dioxus_desktop::{Config, WindowBuilder, tao::window::Icon, WindowCloseBehaviour, LogicalSize};
use ruso::{structs::*, components::*,utils::*, cli};

#[tokio::main]
async fn main() -> anyhow::Result<()>{
    let mut gosu_process: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let _ = generate_example_theme("custom.css");
    ctrlc::set_handler({
        let gosu_process = gosu_process.clone();
            move ||{
            // Fix terminal carriage return
            if let Ok(mut process) = Command::new("stty").arg("sane").spawn(){
                let _ = process.wait();
            }
            let mut gosu_process = gosu_process.lock().unwrap();
            
            // Kill gosumemory if it was started by ruso
            if let Some(ref mut process) = gosu_process.as_mut(){
                #[cfg(not(unix))]
                process.kill().unwrap_or_else(|_| {writeln!(std::io::stderr(), "Could not kill spawned gosumemory process");});

                #[cfg(unix)]
                unsafe{
                    libc::kill(process.id() as i32, libc::SIGTERM);
                }
            }
        }
    })?;
    if std::env::args().len() > 1{
        cli::run().await?;
    }else{
        let settings = Settings::new_from_config();
        tokio::spawn(async move{
            if tokio_tungstenite::connect_async(&settings.websocket_url).await.is_err() && settings.gosumemory_startup  {
                gosu_process = match gosu_startup(&settings){
                    Ok(k) => Arc::new(Mutex::new(Some(k))),
                    Err(e) => return eprintln!("Could not start gosumemory: {}", e)
                } 
            }
        });

        // #[cfg(target_os = "windows")]
        // let window_icon: Icon = Icon::from_rgba(include_bytes!("../assets/icons/icon.ico").to_vec(), 512, 512).unwrap();
        // #[cfg(target_os = "macos")]
        // let window_icon: Icon = Icon::from_rgba(include_bytes!("../assets/icons/icon.icns").to_vec(), 512, 512).unwrap();
        // #[cfg(target_os = "linux")]
        // let window_icon: Icon = Icon::from_rgba(include_bytes!("../assets/icons/icon.png").to_vec(), 512, 512).unwrap();

        let window_icon = Icon::from_rgba(include_bytes!("../assets/icons/icon.bin").to_vec(), 512, 512);
        
        match window_icon{
            Ok(window_icon) => dioxus_desktop::launch_cfg(App,
                Config::default()
                    .with_resource_directory(PathBuf::from("assets"))
                    .with_data_directory(dirs::data_dir().unwrap())
                    .with_close_behaviour(WindowCloseBehaviour::LastWindowExitsApp)
                    .with_background_color((0,0,0,255))
                    .with_icon(window_icon.clone())
                    .with_disable_context_menu(false)
                    .with_custom_head(format!(
                        r#"
                    <title>ruso!</title>
                    <script>{}</script>
                    "#
                    , include_str!("js/script.js")))
                    .with_window(WindowBuilder::new()
                        .with_maximizable(true)
                        .with_resizable(true)
                        .with_title("ruso!")
                        // Using bin file since png doesn't work for some reason
                        .with_window_icon(Some(window_icon))
                        // .with_max_inner_size(LogicalSize::new(1100.0, 800.0))
                        .with_min_inner_size(LogicalSize::new(400.0, 500.0))
                        .with_inner_size(LogicalSize::new(550.0, 650.0))
                    )
                ),
            Err(_) => dioxus_desktop::launch_cfg(App,
                Config::default()
                    .with_resource_directory(PathBuf::from("assets"))
                    .with_data_directory(dirs::data_dir().unwrap())
                    .with_close_behaviour(WindowCloseBehaviour::LastWindowExitsApp)
                    .with_background_color((0,0,0,255))
                    .with_disable_context_menu(false)
                    .with_custom_head(
                        r#"
                    <title>ruso!</title>
                    "#
                    .to_string())
                    .with_window(WindowBuilder::new()
                        .with_maximizable(true)
                        .with_resizable(true)
                        .with_title("ruso!")
                        // .with_max_inner_size(LogicalSize::new(1100.0, 800.0))
                        .with_min_inner_size(LogicalSize::new(400.0, 500.0))
                        .with_inner_size(LogicalSize::new(550.0, 650.0))
                    )
                )
        }
    }
    
    Ok(())
}

fn App(cx: Scope) -> Element {
    use_shared_state_provider(cx, || Settings::new_from_config());
    use_shared_state_provider(cx, || MapOptions::new());
    use_shared_state_provider(cx, || Tab::Manual);
    use_shared_state_provider(cx, || StatusMessage::new());
    let tab = use_shared_state::<Tab>(cx)?;
    let settings = use_shared_state::<Settings>(cx)?;
    let msg = use_shared_state::<StatusMessage>(cx)?;
    
    cx.render(rsx! {
        style { include_str!("css/style.css") }
        div{
            class: "document",
            match settings.read().theme{
                Theme::Light => rsx! {
                    style { include_str!("css/light.css") }
                },                                          
                Theme::Dark => rsx! {                       
                    style { include_str!("css/dark.css") }
                },                                          
                Theme::Osu => rsx! {                        
                    style { include_str!("css/osu.css") }
                },
                Theme::Custom => {
                    // let content = std::fs::read_to_string(dirs::config_dir().unwrap().join("ruso").join("custom.css")).unwrap();
                    match std::fs::read_to_string(dirs::config_dir().unwrap().join("ruso").join("custom.css")){
                        Ok(k) => rsx!{
                                style{ k }
                                },
                        Err(_) => {
                            rsx!{
                                style { include_str!("css/dark.css") }
                            }
                        }
                    }
                }
            }
            div {
                class: "tab-container",
                button{
                    class: "tab-button",
                    title: "Auto select: Automatically select the map from a running osu! instance using gosumemory. This will continually poll gosumemory until it receives a response, no need to refresh.",
                    onclick: move |_| *tab.write() = Tab::Auto,
                    "Auto Select"
                }
                button{
                    class: "tab-button",
                    title: "Manual select: Manually select a map to modify using the file picker",
                    onclick: move |_| *tab.write() = Tab::Manual,
                    "Manual Select"
                }
                button{
                    class: "tab-button",
                    title: "Settings: Configure ruso settings",
                    onclick: move |_| *tab.write() = Tab::Settings,
                    "Settings"
                }
            }
            match *tab.read() {
                Tab::Auto => rsx!{ AutoTab{} },
                Tab::Manual => rsx!{ ManualTab{} },
                Tab::Settings => rsx! { SettingsTab{} },
            }
        }
    })
}


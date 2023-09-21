#![allow(non_snake_case)]
use std::{process::{Child, Command}, sync::{Arc, Mutex}};
use dioxus::prelude::*;
use dioxus_desktop::{Config, WindowBuilder};
mod cli;
use ruso::{structs::*, components::*,*};

#[tokio::main]
async fn main() -> anyhow::Result<()>{
    let mut gosu_process: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let gosu_process_clone = gosu_process.clone();
    let _ = generate_example_theme("custom.css");
    ctrlc::set_handler(move ||{
        // Fix terminal carriage return
        if let Ok(mut process) = Command::new("stty").arg("sane").spawn(){
            let _ = process.wait();
        }
        let mut gosu_process_clone = gosu_process_clone.lock().unwrap();
        
        // Kill gosumemory if it was started by ruso
        if let Some(ref mut process) = gosu_process_clone.as_mut(){
            #[cfg(not(unix))]
            process.kill().unwrap_or_else(|_| {writeln!(std::io::stderr(), "Could not kill spawned gosumemory process");});

            #[cfg(unix)]
            unsafe{
                libc::kill(process.id() as i32, libc::SIGTERM);
            }
        }
    })?;
    if std::env::args().len() > 1{
        cli::run().await?;
    }else{
        let settings = Settings::new_from_config();
        if tokio_tungstenite::connect_async(&settings.websocket_url).await.is_err() && settings.gosumemory_startup  {
            gosu_process = Arc::new(Some(gosu_startup(&settings)?).into());
        }
        dioxus_desktop::launch_cfg(App,
            Config::default().with_window(WindowBuilder::new().with_maximizable(true).with_maximizable(true).with_resizable(true)
            .with_inner_size(dioxus_desktop::wry::application::dpi::LogicalSize::new(400.0, 600.0)))
        );
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
                        Err(e) => {
                            // msg.write().text = Some(format!("Could not read custom.css: {:?}. Reverting to dark theme.", e));
                            // msg.write().status = Status::Error;
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
                    onclick: move |_| *tab.write() = Tab::Auto,
                    "Auto Select"
                }
                button{
                    class: "tab-button",
                    onclick: move |_| *tab.write() = Tab::Manual,
                    "Manual Select"
                }
                button{
                    class: "tab-button",
                    onclick: move |_| *tab.write() = Tab::Settings,
                    "Settings"
                }
            }
            match *tab.read() {
                Tab::Auto => rsx!{ AutoTab{} },
                Tab::Manual => rsx!{ ManualTab{} },
                Tab::Settings => rsx! { SettingsTab{} },
            }
    })
}


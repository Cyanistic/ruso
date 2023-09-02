#![allow(non_snake_case)]
use std::path::PathBuf;
use dioxus::prelude::*;
use dioxus_desktop::{Config, WindowBuilder};
use tokio_tungstenite::connect_async;
use futures_util::StreamExt;
use serde_json::from_str;
use rfd::FileDialog;
mod props;
mod cli;
use props::*;
use ruso::*;
use crate::structs::*;

#[tokio::main]
async fn main() {
    if std::env::args().len() > 1{
        cli::run().await.unwrap();
    }else{
        let settings = Settings::new_from_config();
        if tokio_tungstenite::connect_async(&settings.websocket_url).await.is_err() && settings.gosumemory_startup  {
            gosu_startup(&settings).unwrap();
        }
        dioxus_desktop::launch_cfg(App,
            Config::default().with_window(WindowBuilder::new().with_maximizable(true).with_maximizable(true).with_resizable(true)
            .with_inner_size(dioxus_desktop::wry::application::dpi::LogicalSize::new(400.0, 600.0)))
        );
    }
}

fn App(cx: Scope) -> Element {
    use_shared_state_provider(cx, || Settings::new_from_config());
    use_shared_state_provider(cx, || MapOptions::new());
    use_shared_state_provider(cx, || Tab::Manual);
    use_shared_state_provider(cx, || StatusMessage::new());
    let tab = use_shared_state::<Tab>(cx)?;
    let settings = use_shared_state::<Settings>(cx)?;

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
                    rsx! {
                        style { std::fs::read_to_string(dirs::config_dir().unwrap().join("ruso").join("custom.css")).unwrap() }
                    }
                }
            }
            div {
                button{
                    onclick: move |_| *tab.write() = Tab::Auto,
                    "Auto Select"
                }
                button{
                    onclick: move |_| *tab.write() = Tab::Manual,
                    "Manual Select"
                }
                button{
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

fn GenericSlider<'a>(cx: Scope<'a, SliderProps<'a>>) -> Element{
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    cx.render(rsx! {
        div {
            title: "{cx.props.name}",
            "{cx.props.acronym}"
            input {
                r#type: "range",
                min: 0,
                max: 100,
                value: "{cx.props.read * 10.0}",
                class: "slider",
                id: "{cx.props.acronym}",
                onwheel: move |ev|{
                    let mut temp_val = round_dec(cx.props.read - ev.data.delta().strip_units().y / 1500.0, 2);
                    if temp_val > 10.0 {
                        temp_val  = 10.0;
                    } else if temp_val < 0.0 {
                        temp_val = 0.0;
                    }                    
                    cx.props.on_event.call(temp_val);
                },
                oninput: move |ev|{
                    cx.props.on_event.call(ev.data.value.parse::<f64>().unwrap() / 10.0);
                },
            }
            input { 
                r#type: "number",
                min: 0,
                max: 10,
                step: 0.1,
                value: "{cx.props.read}",
                id: "{cx.props.acronym}_number",
                onwheel: move |ev|{
                    let mut temp_val = round_dec(cx.props.read - ev.data.delta().strip_units().y / 1500.0, 2);
                    if temp_val > 10.0 {
                        temp_val  = 10.0;
                    } else if temp_val < 0.0 {
                        temp_val = 0.0;
                    }                    
                    cx.props.on_event.call(temp_val);
                },
                onchange: move |ev|{
                    let mut temp_val = ev.data.value.parse::<f64>().unwrap_or(cx.props.read);
                    if temp_val > 10.0 {
                        temp_val  = 10.0;
                    } else if temp_val < 0.0 {
                        temp_val = 0.0;
                    }                    
                    cx.props.on_event.call(temp_val);
                },
            }
            if cx.props.locked {
                rsx!{
                    img {
                        src: "{root_dir.join(\"assets/locked-lock.png\").display()}",
                        width: "26px",
                        height: "26px",
                        onclick: move |_| {
                            cx.props.on_lock.call(cx.props.locked);
                        },
                    }
                }
            } else {
                rsx!{
                    img {
                        src: "{root_dir.join(\"assets/unlocked-lock.png\").display()}",
                        width: "26px",
                        height: "26px",
                        onclick: move |_| {
                            cx.props.on_lock.call(cx.props.locked);
                        },
                    }
                }
            }
        }
    })
}

#[inline_props]
fn RateSlider<'a>(cx: Scope, on_event: EventHandler<'a, f64>, bpm: usize, rate: f64) -> Element{
    let value = use_state(cx, || 1.0);
    let new_bpm = use_memo(cx, (bpm, value), |(bpm, value)| {
        (bpm as f64 * *value.get()).round() as usize
    });
    
    cx.render(rsx! {
        div {
            title: "Rate",
            "Rate"
            input {
                r#type: "range",
                min: 1,
                max: 60,
                value: *value.get() * 20.0,
                class: "slider",
                id: "Rate",
                onwheel: move |ev|{
                    let mut temp_val = round_dec(*value.get() - ev.data.delta().strip_units().y / 3000.0, 2);
                    if temp_val > 10.0 {
                        temp_val = 10.0;
                    } else if temp_val < 0.05 {
                        temp_val = 0.05;
                    }
                    value.set(temp_val);
                    cx.props.on_event.call(temp_val);
                },
                oninput: move |ev|{
                    value.set(ev.data.value.parse::<f64>().unwrap() / 20.0);
                    cx.props.on_event.call(*value.get());
                }
            }
            input { 
                r#type: "number",
                min: 0.05,
                max: 40,
                step: 0.05,
                value: round_dec(*value.get(), 2),
                id: "Rate_number",
                onwheel: move |ev|{
                    let mut temp_val = round_dec(*value.get() - ev.data.delta().strip_units().y / 3000.0, 2);
                    if temp_val > 40.0 {
                        temp_val = 40.0;
                    } else if temp_val < 0.05 {
                        temp_val = 0.05;
                    }
                    value.set(temp_val);
                    cx.props.on_event.call(round_dec(temp_val, 2));
                },
                onchange: move |ev|{
                    let mut temp_val = ev.data.value.parse::<f64>().unwrap_or(*value.get());
                    if temp_val > 40.0 {
                        temp_val = 40.0;
                    } else if temp_val < 0.05 {
                        temp_val = 0.05;
                    }
                    value.set(temp_val);
                    cx.props.on_event.call(round_dec(temp_val, 2));
                },
            }
            br {}
            "BPM"
            input { 
                r#type: "number",
                min: 0,
                max: f64::MAX,
                step: 1,
                value: "{new_bpm}",
                id: "bpm_number",
                onwheel: move |ev|{
                    let mut temp_val = round_dec(*value.get() - ev.data.delta().strip_units().y / 3000.0, 2);
                    if temp_val > 40.0 {
                        temp_val = 40.0;
                    } else if temp_val < 0.05 {
                        temp_val = 0.05;
                    }
                    value.set(temp_val);
                    cx.props.on_event.call(temp_val);
                },
                onchange: move |ev|{
                    let temp_val = ev.data.value.parse::<usize>().unwrap_or(*new_bpm);
                    let new_rate = temp_val as f64 / *bpm as f64;
                    value.set(new_rate);
                    cx.props.on_event.call(new_rate);
                },
            }
        }
    })
}

fn SettingsTab(cx: Scope) -> Element{
    let map = use_shared_state::<MapOptions>(cx)?;
    let msg = use_shared_state::<StatusMessage>(cx)?;
    let settings = use_shared_state::<Settings>(cx)?;
    cx.render(rsx!{
        h1 { "Settings" }
        div {
            title: "Settings",
            "Theme: "
            select {
                value: match settings.read().theme{
                    Theme::Light => "Light",
                    Theme::Dark => "Dark",
                    Theme::Osu => "osu!",
                    Theme::Custom => "Custom"
                },
                
                onchange: move |ev|{
                    settings.write().theme = match ev.data.value.as_str(){
                        "Light" => Theme::Light,
                        "Dark" => Theme::Dark,
                        "osu!" => Theme::Osu,
                        "Custom" => Theme::Custom,
                        _ => Theme::Dark
                    }
                },
                option { "Light" }
                option { "Dark" }
                option { "osu!" }
                option { "Custom" }
            }
            br {}
            "Websocket URL: "
            input {
                r#type: "text",
                value: "{settings.read().websocket_url}",
                title: "Websocket URL: This is the url of the websocket that ruso will connect to when auto is chosen, you probably don't want to touch this.",
                placeholder: "ws://localhost:24050/ws",
                oninput: move |ev| settings.write().websocket_url = ev.value.clone()
            }
            br {}
            "gosumemory path: "
            input {
                r#type: "text",
                value: "{settings.read().gosumemory_path.display()}",
                title: "gosumemory path: This is the path to your gosumemory executable, which ruso requires for auto selection",
                placeholder: "C:\\Program Files\\gosumemory",
                oninput: move |ev| settings.write().gosumemory_path = PathBuf::from(ev.value.clone())
            }
            button {
                    onclick: move |_| {
                        let file_picker = FileDialog::new()
                        .set_title("Choose your gosumerory executable");
                        let selected = match file_picker.pick_file(){
                            Some(k) => k,
                            None => return
                        };
                        settings.write().gosumemory_path = selected;
                    },
                    "Choose path"
            }
            br {}
            "Run gosumemory on startup: "
            input {
                r#type: "checkbox",
                checked: "{settings.read().gosumemory_startup}",
                title: "Attempt to run gosumemory on startup using given path (requires sudo permissions on linux)",
                onclick: move |_| {
                    let temp = settings.read().gosumemory_startup;
                    settings.write().gosumemory_startup = !temp;
                }
            }
            br {}
            "osu! songs path: "
            input {
                r#type: "text",
                value: "{settings.read().songs_path.display()}",
                title: "osu! songs path: This is the path to your osu! songs folder",
                placeholder: "C:\\Users\\User\\AppData\\Local\\osu!\\Songs",
                oninput: move |ev| settings.write().songs_path = PathBuf::from(ev.value.clone())
            }
            button {
                    onclick: move |_| {
                        let dir_picker = FileDialog::new()
                        .set_title("Choose your osu! Songs directory");
                        let selected = match dir_picker.pick_folder(){
                            Some(k) => k,
                            None => return
                        };
                        settings.write().songs_path = selected;
                    },
                    "Choose path"
            }
            br {}
            button {
                onclick: move |_| {
                    match write_config(&settings.read()){
                        Ok(_) => println!("Settings saved to file successfully!"),
                        Err(e) => eprintln!("Error saving settings: {}", e)
                    }
                },
                "Save settings"
            }
            br {}
            button {
                title: "Clean maps: This will remove all maps that ruso has created, this will not remove any maps that you have created yourself.",
                onclick: move |_| {
                    match clean_maps(&settings.read()){
                        Ok(k) => {
                            msg.write().text = Some(format!("Cleaned {} files successfully!", k));
                            msg.write().status = Status::Success;
                        },
                        Err(e) => {
                            msg.write().text = Some(format!("Error cleaning maps: {}", e));
                            msg.write().status = Status::Error;
                        }
                    }
                },
                "Clean maps"
            }
            h6 {
                "Config Path: {dirs::config_dir().unwrap().join(\"ruso\").display()}"
            }
        }
    })
}

fn AutoTab(cx: Scope) -> Element{
    let map = use_shared_state::<MapOptions>(cx)?;
    let settings = use_shared_state::<Settings>(cx)?;
    let msg = use_shared_state::<StatusMessage>(cx)?;
    let _: &Coroutine<()> = use_coroutine(cx, |_: UnboundedReceiver<_>| { 
        to_owned![map, settings, msg];
        async move{
            loop{
                to_owned![map, settings, msg];
                let settings_url = match url::Url::parse(settings.read().websocket_url.clone().as_str()){
                    Ok(k) => k,
                    Err(e) => {
                        msg.write().text = Some(format!("Error parsing websocket url: {}", e));
                        msg.write().status = Status::Error;
                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                        continue
                    }
                };
                println!("Connecting to websocket: {}", &settings_url);
                let (socket, _) = match connect_async(&settings_url).await{
                    Ok(k) => k,
                    Err(e) => {
                        msg.write().text = Some(format!("Error connecting to websocket: {}", e));
                        msg.write().status = Status::Error;
                        eprintln!("Error connecting to websocket... Retrying in 1 second");
                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                        continue
                    }
                };
                let (_, mut read) = socket.split();
                let local = tokio::task::LocalSet::new();
                local.run_until( async move{
                    tokio::task::spawn_local( async move{
                        while let Some(message) = read.next().await{
                            match message{
                                Ok(message) => {
                                    let data: serde_json::Value = from_str(&message.into_text().unwrap()).unwrap();
                                    if map.read().map_path != PathBuf::from(data["menu"]["bm"]["path"]["folder"].as_str().unwrap()).join(data["menu"]["bm"]["path"]["file"].as_str().unwrap()) {
                                        map.write().map_path = PathBuf::from(data["menu"]["bm"]["path"]["folder"].as_str().unwrap()).join(data["menu"]["bm"]["path"]["file"].as_str().unwrap());
                                        if settings.read().songs_path != PathBuf::from(data["settings"]["folders"]["songs"].as_str().unwrap()){

                                        }
                                        let temp_map = map.read().clone();
                                        *map.write() = match read_map_metadata(temp_map, &settings.read()){
                                            Ok(k) => k,
                                            Err(e) => {
                                                msg.write().text = Some(format!("Error reading map metadata: {}", e));
                                                msg.write().status = Status::Error;
                                                continue
                                            }
                                        };
                                    }
                                },
                                Err(e) => {
                                    msg.write().text = Some(format!("Lost connection to Websocket: {}", e));
                                    msg.write().status = Status::Error;
                                }
                            }
                        };
                    }).await;
                }).await;
            }
        }
    });
    cx.render(rsx!{
        h1 { "Auto" }
        MapOptionsComponent{}
    })
}

fn ManualTab(cx: Scope) -> Element{
    let map = use_shared_state::<MapOptions>(cx)?;
    let settings = use_shared_state::<Settings>(cx)?;
    let msg = use_shared_state::<StatusMessage>(cx)?;
    
    cx.render(rsx!{
            if *settings.read().songs_path == PathBuf::new(){
                rsx!{
                    h2 { "Choose your osu Songs directory in the Settings tab!" }
                }
            }else{
                rsx!{
                    h4 { "Current directory:" "{settings.read().songs_path.display()}" }
                        h4 { "Selected map: " "{map.read().map_path.display()}" }
                        button {
                        onclick: move |_| {
                            let songs_folder = settings.read().songs_path.clone();
                            let map_picker = FileDialog::new()
                                .add_filter("osu! map", &["osu"])
                                .set_title("Choose a map to edit")
                                .set_directory(songs_folder);
                            let prefix = settings.read().songs_path.clone();
                            map.write().map_path = map_picker.clone().pick_file().unwrap().strip_prefix(prefix).unwrap().to_path_buf();
                            let temp_map = map.read().clone();
                            *map.write() = read_map_metadata(temp_map, &settings.read()).unwrap();
                        },
                        "Choose path"
                        }
                }
            }
            div {            
                MapOptionsComponent{}
            }
    })
}

fn MapOptionsComponent(cx: Scope) -> Element{
    let map = use_shared_state::<MapOptions>(cx)?;
    let settings = use_shared_state::<Settings>(cx)?;
    let msg = use_shared_state::<StatusMessage>(cx)?;
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    cx.render(rsx!{
        if let Some(bg) = &map.read().background{
            if settings.read().songs_path.join(bg).exists(){
                rsx!{
                    img {
                        src: "{settings.read().songs_path.join(bg).display()}",
                        width: "100%",
                        height: "100%"
                    }
                }
            }else{
                rsx!{
                    img {
                        src: "{root_dir.join(\"assets/nobg.png\").display()}",
                        width: "100%",
                        height: "100%"
                    }
                }
            }
        }else{
            rsx!{
                img {
                    src: "{root_dir.join(\"assets/nobg.png\").display()}",
                    width: "100%",
                    height: "100%"
                }
            }
        }
        if !map.read().title.is_empty(){
            rsx!{
                h3 {
                "{map.read().artist} - {map.read().title} [{map.read().difficulty_name}]"
                }
            }
        }
        div{
            h2 { "Map Options" }
            GenericSlider {
                name: "Approach Rate",
                acronym: "AR",
                read: map.read().approach_rate,
                locked: settings.read().ar_lock,
                on_event: move |ev| map.write().approach_rate = ev,
                on_lock: move |ev: bool| settings.write().ar_lock = !ev
            }
            GenericSlider {
                name: "Circle Size",
                acronym: "CS",
                read: map.read().circle_size,
                locked: settings.read().cs_lock,
                on_event: move |ev| map.write().circle_size = ev,
                on_lock: move |ev: bool| settings.write().cs_lock = !ev
            }
            GenericSlider {
                name: "HP Drain",
                acronym: "HP",
                read: map.read().hp_drain,
                locked: settings.read().hp_lock,
                on_event: move |ev| map.write().hp_drain = ev,
                on_lock: move |ev: bool| settings.write().hp_lock = !ev
            }
            GenericSlider {
                name: "Overall Difficulty",
                acronym: "OD",
                read: map.read().overall_difficulty,
                locked: settings.read().od_lock,
                on_event: move |ev| map.write().overall_difficulty = ev,
                on_lock: move |ev: bool| settings.write().od_lock = !ev
            }
            RateSlider {
                bpm: map.read().bpm,
                on_event: move |ev| map.write().rate = ev,
                rate: map.read().rate
            }
        }
        div {
            title: "Buttons",
            button {
                onclick: move |_| {
                    if map.read().rate != 1.0{
                        match generate_map(&map.read(), &settings.read()){
                            Ok(_) => {
                                msg.write().text = Some("Map created successfully!".to_string());
                                msg.write().status = Status::Success;
                            },
                            Err(e) => {
                                msg.write().text = Some(format!("Error creating map: {}", e));
                                msg.write().status = Status::Error;
                            }
                        }
                    }else{
                        match change_map_difficulty(&map.read(), &settings.read()){
                            Ok(_) => {
                                msg.write().text = Some("Map created successfully!".to_string());
                                msg.write().status = Status::Success;
                            },
                            Err(e) => {
                                msg.write().text = Some(format!("Error creating map: {}", e));
                                msg.write().status = Status::Error;
                            }
                        };
                    }
                },
                "Create map"
            }
            button {
                onclick: move |_| {
                let temp_map = map.read().clone();
                *map.write() = read_map_metadata(temp_map, &settings.read()).unwrap();
            },
                "Reset"
            }
        }
        div {
            title: "Messages",
            p {
                if let Some(msg) = &msg.read().text{
                    rsx! {"{msg}"}
                }
            }
        }
    })
}

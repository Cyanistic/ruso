use std::path::PathBuf;
use dioxus::prelude::*;
use tokio_tungstenite::{connect_async, tungstenite::Error};
use serde_json::from_str;
use rfd::FileDialog;
use libosu::data::Mode;
use crate::{props::*, structs::*, *};
use futures_util::StreamExt;

pub fn GenericSlider<'a>(cx: Scope<'a, SliderProps<'a>>) -> Element{
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    cx.render(rsx! {
        div {
            class: "slider-container",
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
pub fn RateSlider<'a>(cx: Scope, on_event: EventHandler<'a, f64>, bpm: usize, rate: f64) -> Element{
    let value = use_state(cx, || 1.0);
    let new_bpm = use_memo(cx, (bpm, value), |(bpm, value)| {
        (bpm as f64 * *value.get()).round() as usize
    });
    
    cx.render(rsx! {
        div {
            class: "slider-container",
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
        }
            "Old BPM: {bpm}"
            br {}
            "New BPM: "
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
    })
}

pub fn SettingsTab(cx: Scope) -> Element{
    let map = use_shared_state::<MapOptions>(cx)?;
    let msg = use_shared_state::<StatusMessage>(cx)?;
    let settings = use_shared_state::<Settings>(cx)?;

    #[cfg(windows)]
    let placeholder_songs_path = "C:\\Users\\User\\AppData\\Local\\osu!\\Songs";
    #[cfg(windows)]
    let placeholder_gosu_path = "C:\\Program Files\\gosumemory";

    #[cfg(not(windows))]
    let placeholder_gosu_path = "/usr/bin/gosumemory";
    #[cfg(not(windows))]
    let placeholder_songs_path = "/home/user/.local/share/osu-wine/osu!/Songs";

    cx.render(rsx!{
        h1 {
            style: "text-align: center;",
            "Settings"
        }
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
                placeholder: placeholder_gosu_path,
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
                placeholder: placeholder_songs_path,
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

pub fn AutoTab(cx: Scope) -> Element{
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
                    Ok(k) => {
                        msg.write().text = Some("Connected to websocket!".to_string());
                        msg.write().status = Status::Success;
                        k
                    },
                    Err(Error::Io(e)) if e.kind() == ErrorKind::ConnectionRefused => {
                        msg.write().text = Some(format!("Error connecting to websocket. Is gosumemory running with the websocket url set in settings?"));
                        msg.write().status = Status::Error;
                        eprintln!("Error connecting to websocket. Is gosumemory running? Retrying in 1 second");
                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                        continue
                    },
                    Err(e) =>{
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
        h1 {
            style: "text-align: center;",
            "Auto"
        }
        MapOptionsComponent{}
    })
}

pub fn ManualTab(cx: Scope) -> Element{
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
                    h4 { "Songs directory:" "{settings.read().songs_path.display()}" }
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

pub fn MapOptionsComponent(cx: Scope) -> Element{
    let map = use_shared_state::<MapOptions>(cx)?;
    let settings = use_shared_state::<Settings>(cx)?;
    let msg = use_shared_state::<StatusMessage>(cx)?;
    let assets = &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");

    // Determine status message color
    let status_color = *use_memo(cx, &(msg.read().status), |status|{
        match status{
            Status::Success => "#87E37D",
            Status::Error => "#FF6B6B"
        }
    });

    // Determine image path for background image
    let bg_path = use_memo(cx, &(map.read().background), |bg|{
        if let Some(path) = bg{
            if settings.read().songs_path.join(&path).exists(){
                settings.read().songs_path.join(&path)
            }else{
                assets.join("no-bg.jpg")
            }
        }else{
            assets.join("no-bg.jpg")
        }
    });

    // Get image for respective osu! gamemode
    let mode_img = use_memo(cx, &(map.read().mode), |mode|{
        match mode{
            Mode::Osu => assets.join("standard.png"),
            Mode::Taiko => assets.join("taiko.png"),
            Mode::Catch => assets.join("catch.png"),
            Mode::Mania => assets.join("mania.png")
        }
    });

    // Using css filters for the respective star range colors since I don't want to color the image
    // manually
    let css_filter = use_memo(cx, &(map.read().stars), |stars|{
        match stars{
            _ if stars < 2.0 => "invert(69%) sepia(33%) saturate(2985%) hue-rotate(175deg) brightness(102%) contrast(101%)",
            _ if stars < 2.7 => "invert(76%) sepia(69%) saturate(421%) hue-rotate(50deg) brightness(98%) contrast(111%)",
            _ if stars < 4.0 => "invert(90%) sepia(39%) saturate(654%) hue-rotate(357deg) brightness(96%) contrast(100%)",
            _ if stars < 5.3 => "invert(72%) sepia(61%) saturate(7424%) hue-rotate(320deg) brightness(101%) contrast(101%)",
            _ if stars < 6.5 => "invert(51%) sepia(35%) saturate(6862%) hue-rotate(278deg) brightness(82%) contrast(87%)",
            _ if stars < 7.5 => "invert(40%) sepia(27%) saturate(3352%) hue-rotate(220deg) brightness(91%) contrast(90%)",
            _ => "grayscale(100%)"
        }
    });

    cx.render(rsx!{
        div {
            class: "map-image",
            style: r#"background-image: linear-gradient(rgba(0, 0, 0, 0.5), rgba(0, 0, 0, 0.5)), url("{bg_path.display()}");"#,
            if !map.read().title.is_empty(){
                rsx!{
                    div{
                        class: "map-title",
                        "{map.read().title}"
                    }
                    div{
                        class: "map-artist",
                        "{map.read().artist}"
                    }
                    div{
                        class: "map-difficulty",
                        "{map.read().difficulty_name}"
                    }
                    div{
                        class: "map-stars",
                        "{map.read().stars} "
                        img {
                            src: "{mode_img.display()}",
                            width: "24px",
                            height: "24px",
                            style: "filter: {css_filter}; margin-bottom: -6px;"
                        }
                    }
                }
            }
        }
        div{
            h2 { 
                class: "title",
                "Map Options"
            }
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
            class: "button-container",
            title: "Buttons",
            button {
                class: "create-button",
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
                class: "reset-button",
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
                    rsx! {
                        div{
                            class: "status-message",
                            style: "background-color: {status_color};",
                            p{
                                style: "text-align: center;",
                                "{msg}"
                            }
                        }
                    }
                }
            }
        }
    })
}

#![allow(non_snake_case)]
use std::{path::PathBuf, time::Duration, io::ErrorKind};
use dioxus::prelude::*;
use include_base64::include_base64_std;
use tokio_tungstenite::{connect_async, tungstenite::Error};
use serde_json::from_str;
use rfd::FileDialog;
use libosu::data::Mode;
use crate::{props::{SliderProps, ToggleableProps}, structs::{MapOptions, Settings, Status, StatusMessage, Theme, Tab}, utils::*};
use futures_util::StreamExt;

pub fn GenericSlider<'a>(cx: Scope<'a, SliderProps<'a>>) -> Element{
    cx.render(rsx! {
        div {
            class: "slider-container",
            title: "{cx.props.name}",
            span{ class: "slider-label", "{cx.props.acronym}"}
            input {
                r#type: "range",
                min: 0,
                max: 100,
                value: "{cx.props.read * 10.0}",
                class: "slider generic-slider",
                id: "{cx.props.acronym}",
                onwheel: move |ev|{
                    let mut temp_val = round_dec(cx.props.read - (ev.data.delta().strip_units().y.signum()/10.0), 2);
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
                    let mut temp_val = round_dec(cx.props.read - (ev.data.delta().strip_units().y.signum()/10.0), 2);
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
            div{
                class: "lock-container",
                title: "{cx.props.name} Lock: Locks the {cx.props.acronym} slider to the current value, preventing it from being changed by loading a new map",
                onclick: move |_| {
                    cx.props.on_lock.call(cx.props.locked);
                },
                if cx.props.locked {
                    rsx!{
                        LockedLock{}
                    }
                }else {
                    rsx!{
                        UnlockedLock{}
                    }
                }
            }
        }
    })
}

#[inline_props]
pub fn RateSlider<'a>(cx: Scope, on_event: EventHandler<'a, f64>, bpm: usize) -> Element{
    let value = use_state(cx, || 1.0);
    let new_bpm = (*bpm as f64 * *value.get()).round() as usize;
    let settings = use_shared_state::<Settings>(cx)?;
    
    cx.render(rsx! {
        div {
            class: "slider-container rate-slider-container",
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
                    let mut temp_val = round_dec(*value.get() - (ev.data.delta().strip_units().y.signum()/20.0), 2);
                    if temp_val > 10.0 {
                        temp_val = 10.0;
                    } else if temp_val < 0.05 {
                        temp_val = 0.05;
                    }
                    value.set(temp_val);
                    on_event.call(temp_val);
                },
                oninput: move |ev|{
                    value.set(ev.data.value.parse::<f64>().unwrap() / 20.0);
                    on_event.call(*value.get());
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
                    let mut temp_val = round_dec(*value.get() - (ev.data.delta().strip_units().y.signum()/20.0), 2);
                    if temp_val > 40.0 {
                        temp_val = 40.0;
                    } else if temp_val < 0.05 {
                        temp_val = 0.05;
                    }
                    value.set(temp_val);
                    on_event.call(round_dec(temp_val, 2));
                },
                onchange: move |ev|{
                    let mut temp_val = ev.data.value.parse::<f64>().unwrap_or(*value.get());
                    if temp_val > 40.0 {
                        temp_val = 40.0;
                    } else if temp_val < 0.05 {
                        temp_val = 0.05;
                    }
                    value.set(temp_val);
                    on_event.call(round_dec(temp_val, 2));
                },
            }
        }
        div{
            class: "bpm-grid",
            div{
                class: "bpm-label",
                "Old BPM "
            }
            div{
                class: "bpm-input",
                "{bpm}"
            }
            div{
                class: "bpm-label",
                "New BPM "
            }
            input { 
                r#type: "number",
                min: 0,
                max: f64::MAX,
                step: 1,
                value: "{new_bpm}",
                class: "bpm-input",
                id: "bpm_number",
                onwheel: move |ev|{
                    let mut temp_val = round_dec(*value.get() - (ev.data.delta().strip_units().y.signum()/20.0), 2);
                    if temp_val > 40.0 {
                        temp_val = 40.0;
                    } else if temp_val < 0.05 {
                        temp_val = 0.05;
                    }
                    value.set(temp_val);
                    cx.props.on_event.call(temp_val);
                },
                onchange: move |ev|{
                    let temp_val = ev.data.value.parse::<usize>().unwrap_or(new_bpm);
                    let new_rate = temp_val as f64 / *bpm as f64;
                    value.set(new_rate);
                    cx.props.on_event.call(new_rate);
                },
            }
            Toggleable{
                name: "Change pitch",
                title: "Change pitch: If checked, the pitch of the audio file will scale with the rate. If disabled, the pitch will remain the same, no matter the rate.",
                toggled: settings.read().change_pitch,
                on_event: move |ev: bool| settings.write().change_pitch = !ev
            }
            Toggleable{
                name: "Override audio",
                title: "Override audio: Forces the generation of a new audio file even if one of the same rate already exists. This is useful if you already generated an audio file with a changed pitch and now want to regenerate the same audio file with an unchanged pitch, or vice versa.",
                toggled: settings.read().force_generation,
                on_event: move |ev: bool| settings.write().force_generation = !ev
            }
        }
    })
}

pub fn Toggleable<'a>(cx: Scope<'a, ToggleableProps<'a>>) -> Element{
    cx.render(rsx!{
        div{
            class: "toggleable-container",
            title: "{cx.props.title}",
            "{cx.props.name}"
            input{
                r#type: "checkbox",
                checked: "{cx.props.toggled}",
                onclick: move |_| {
                    cx.props.on_event.call(cx.props.toggled);
                }
            }
        }
    })
}

pub fn SettingsTab(cx: Scope) -> Element{
    let msg = use_shared_state::<StatusMessage>(cx)?;
    let settings = use_shared_state::<Settings>(cx)?;
    let used_space = use_state(cx, || match read_space("used_space.txt"){
        Ok(k) => k,
        Err(e) => {
            msg.write().text = Some(format!("Error reading used space: {}", e));
            msg.write().status = Status::Error;
            0
        }
    });
    let used_space_pretty = use_memo(cx, used_space.get(), |used_space|{

        // Had to calculate my own log10() because the built in one had huge rounding errors
        let mut accurate_log: f64 = used_space as f64;
        let mut places: usize = 0;
        while accurate_log > 10.0{
            accurate_log /= 10.0;
            places += 1;
        }

        let shortened_space = round_dec(accurate_log * 10.0_f64.powi(places as i32 % 3), 2);
        let suffix = match places / 3{
            0 => "",
            1 => "K",
            2 => "M",
            3 => "G",
            4 => "T",
            5 => "P",
            6 => "E",
            7 => "Z",
            8 => "Y",
            _ => "touch grass"
        };
        format!("{shortened_space} {suffix}B")
    });
    
    let clean_confirmation = use_state(cx, || false);
    let clean_text = match clean_confirmation.get(){
            true => "Are you sure?",
            false => "Clean maps"
        };

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
            class: "settings-container",
            title: "Settings",
            p {
                style: "display: inline;",
                title: "Config Path: This is the path to the directory where all of your settings are stored. The custom theme is custom.css and settings are in settings.json",
                "Config Path: {dirs::config_dir().unwrap().join(\"ruso\").display()}"
            }
            br {}
            
            div{
                class: "option-container",
                title: "Theme: This is the theme that ruso will use, you can also create your own theme by editing custom.css",
                "Theme "
                select {
                    class: "theme-selector",
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
            }
            br {}
            div{
                class: "option-container",
                title: "Websocket URL: This is the url of the websocket that ruso will connect to when auto is chosen, you probably don't want to touch this.",
                "Websocket URL "
                input {
                    r#type: "text",
                    value: "{settings.read().websocket_url}",
                    placeholder: "ws://localhost:24050/ws",
                    oninput: move |ev| settings.write().websocket_url = ev.value.clone()
                }
            }
            br {}
            div{
                class: "option-container",
                title: "gosumemory path: This is the path to your gosumemory executable, which ruso requires for auto selection",
                "gosumemory path "
                input {
                    r#type: "text",
                    value: "{settings.read().gosumemory_path.display()}",
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
            }
            br {}
            div{
                class: "option-container",
                title: "Attempt to run gosumemory on startup using given path (requires sudo permissions on linux)",
                "Run gosumemory on startup "
                input {
                    r#type: "checkbox",
                    checked: "{settings.read().gosumemory_startup}",
                    onclick: move |_| {
                        let temp = settings.read().gosumemory_startup;
                        settings.write().gosumemory_startup = !temp;
                    }
                }
            }
            br {}
            div{
                class: "option-container",
                title: "Generate .osz files: If checked, ruso generates .osz files instead of .osu files. Enable this if you want a more seamless experience between generating maps with ruso and playing them in osu!",
                "Generate .osz files "
                input {
                    r#type: "checkbox",
                    checked: "{settings.read().generate_osz}",
                    onclick: move |_| {
                        let temp = settings.read().generate_osz;
                        settings.write().generate_osz = !temp;
                    }
                }
            }
            br {}
            div{
                class: "option-container",
                title: "osu! songs path: This is the path to your osu! songs folder",
                "osu! songs path "
                input {
                    r#type: "text",
                    value: "{settings.read().songs_path.display()}",
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
            }
            br {}
            p{ style: "display: inline;", title: "Space used: The amount of space used by the .osu and audio files generated by ruso", "Space used {used_space_pretty}" }
            br {}
            div{
                class: "settings-button-container",
                button {
                    class: "settings-button",
                    title: "Save settings: Saves current settings to settings.json",
                    onclick: move |_| {
                        match write_config(&settings.read(), "settings.json"){
                            Ok(_) => {
                                msg.write().text = Some("Settings saved successfully!".to_string());
                                msg.write().status = Status::Success;
                            },
                            Err(e) => {
                                msg.write().text = Some(format!("Error saving settings: {}", e));
                                msg.write().status = Status::Error;
                            }
                        }
                    },
                    "Save settings"
                }
                button {
                        class: "settings-button",
                        title: "Calculate space: Calculates the amount of space used by the .osu and audio files generated by ruso",
                        onclick: move |_| {
                            match calculate_space("used_space.txt"){
                                Ok(k) => used_space.set(k),
                                Err(e) => {
                                    msg.write().text = Some(format!("Error calculating used space: {}", e));
                                    msg.write().status = Status::Error;
                                }
                            };
                        },
                        "Calculate space"
                }
                button {
                    class: "settings-button",
                    title: "Clean maps: This will remove all maps that ruso has generated, including audio files generated by ruso, this will not remove any maps that you have created yourself.",
                    onclick: move |_| {
                        if *clean_confirmation.get(){
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
                        }
                        clean_confirmation.set(!clean_confirmation.get());
                    },
                    "{clean_text}"
                }
            }
        }
        MessageBox{}
    })
}

pub fn AutoTab(cx: Scope) -> Element{
    let map = use_shared_state::<MapOptions>(cx)?;
    let settings = use_shared_state::<Settings>(cx)?;
    let msg = use_shared_state::<StatusMessage>(cx)?;
    let _: &Coroutine<()> = use_coroutine(cx, |_: UnboundedReceiver<_>| { 
        to_owned![map, settings, msg];
        async move{
            // Spawning a new local thread since the application freezes up when too many loops run
            // on the main thread
            let outer_local = tokio::task::LocalSet::new();
            outer_local.run_until( async move{
                let _ = tokio::task::spawn_local( async move{
                    loop{
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
                                msg.write().text = Some("Error connecting to websocket. Is gosumemory running with the websocket url set in settings?".to_string());
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
                        while let Some(message) = read.next().await{
                            match message{
                                Ok(message) => {
                                    let data: serde_json::Value = from_str(&message.into_text().unwrap()).unwrap();
                                    if map.read().map_path != PathBuf::from(data["menu"]["bm"]["path"]["folder"].as_str().unwrap()).join(data["menu"]["bm"]["path"]["file"].as_str().unwrap()) {
                                        map.write().map_path = PathBuf::from(data["menu"]["bm"]["path"]["folder"].as_str().unwrap()).join(data["menu"]["bm"]["path"]["file"].as_str().unwrap());
                                        if settings.read().songs_path == PathBuf::new() {
                                            settings.write().songs_path = PathBuf::from(data["settings"]["folders"]["songs"].as_str().unwrap());
                                        }
                                        if let Err(e) = map.write().read_map_metadata(&settings.read()){
                                            msg.write().text = Some(format!("Error reading map metadata: {}", e));
                                            msg.write().status = Status::Error;
                                        }
                                    }
                                },
                                Err(e) => {
                                    msg.write().text = Some(format!("Lost connection to Websocket: {}", e));
                                    msg.write().status = Status::Error;
                                }
                            }
                        };
                    }
                }).await;
            }).await;
        }
    });


    cx.render(rsx!{
        MapOptionsComponent{}
    })
}

pub fn ManualTab(cx: Scope) -> Element{
    let map = use_shared_state::<MapOptions>(cx)?;
    let settings = use_shared_state::<Settings>(cx)?;
    
    cx.render(rsx!{
            if *settings.read().songs_path == PathBuf::new(){
                rsx!{
                    h2 { "Choose your osu Songs directory in the Settings tab!" }
                }
            }else if map.read().map_path == PathBuf::new(){
                rsx!{
                    h2 { "Choose a map to modify!" }
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
    let tab = use_shared_state::<Tab>(cx)?;
    let generating_map = use_state(cx, || false);

    // Determine image path for background image
    let bg_path = use_memo(cx, &(map.read().background), |bg|{
        if let Some(path) = bg{
            if settings.read().songs_path.join(&path).exists(){
                settings.read().songs_path.join(&path).display().to_string()
            }else{
                format!("data:image/jpg;base64,{}", include_base64_std!("./assets/no-bg.jpg"))
            }
        }else{
            format!("data:image/jpg;base64,{}", include_base64_std!("./assets/no-bg.jpg"))
        }
    });

    // Get image for respective osu! gamemode
    let mode_img = match map.read().mode{
            Mode::Osu =>   concat!("data:image/png;base64,", include_base64_std!("./assets/standard.png")),
            Mode::Taiko => concat!("data:image/png;base64,", include_base64_std!("./assets/taiko.png")),
            Mode::Catch => concat!("data:image/png;base64,", include_base64_std!("./assets/catch.png")),
            Mode::Mania => concat!("data:image/png;base64,", include_base64_std!("./assets/mania.png"))
        };

    // Using css filters for the respective star range colors since I don't want to color the image
    // manually
    let css_filter = {
            let stars = map.read().stars;
            match stars{
            _ if stars < 2.0 => "brightness(0) saturate(100%) invert(77%) sepia(49%) saturate(4262%) hue-rotate(176deg) brightness(102%) contrast(105%)",
            _ if stars < 2.7 => "brightness(0) saturate(100%) invert(87%) sepia(45%) saturate(722%) hue-rotate(43deg) brightness(101%) contrast(103%)",
            _ if stars < 4.0 => "brightness(0) saturate(100%) invert(87%) sepia(31%) saturate(782%) hue-rotate(4deg) brightness(107%) contrast(93%)",
            _ if stars < 5.3 => "brightness(0) saturate(100%) invert(69%) sepia(66%) saturate(6689%) hue-rotate(320deg) brightness(101%) contrast(101%)",
            _ if stars < 6.5 => "brightness(0) saturate(100%) invert(34%) sepia(32%) saturate(3109%) hue-rotate(276deg) brightness(97%) contrast(84%)",
            _ if stars < 7.5 => "brightness(0) saturate(100%) invert(39%) sepia(37%) saturate(1433%) hue-rotate(208deg) brightness(96%) contrast(91%)",
            _ => "brightness(0) saturate(100%) invert(9%) sepia(85%) saturate(5685%) hue-rotate(247deg) brightness(91%) contrast(90%)"
        }
    };

    cx.render(rsx!{
        div {
            class: "map-image",
            style: r#"background-image: linear-gradient(rgba(0, 0, 0, 0.5), rgba(0, 0, 0, 0.5)), url("{bg_path}");"#,
            if let Tab::Manual = *tab.read() {
                rsx!{
                    button {
                        class: "map-button",
                        title: "Choose map: This will open a file picker where you can choose the map you want to edit. The root directory will be the osu! songs directory that you have chosen in the settings tab.",
                        onclick: move |_| {
                            let map_picker = FileDialog::new()
                                .add_filter("osu! map", &["osu"])
                                .set_title("Choose a map to edit")
                                .set_directory(&settings.read().songs_path);
                            match map_picker.pick_file(){
                                Some(k) => map.write().map_path = k.strip_prefix(&settings.read().songs_path).unwrap().to_path_buf(),
                                None => return
                            };
                            if let Err(e) = map.write().read_map_metadata(&settings.read()){
                                msg.write().text = Some(format!("Error reading map metadata: {}", e));
                                msg.write().status = Status::Error;
                            }
                        },
                        "Choose map"
                    }
                }
            }
            if !map.read().title.is_empty(){
                rsx!{
                    div{
                        class: "map-title",
                        title: "Title: {map.read().title}",
                        "{map.read().title}"
                    }
                    div{
                        class: "map-artist",
                        title: "Artist: {map.read().artist}",
                        "{map.read().artist}"
                    }
                    div{
                        class: "map-difficulty",
                        title: "Difficulty Name: {map.read().difficulty_name}",
                        "{map.read().difficulty_name}"
                    }
                    div{
                        class: "map-stars",
                        title: "Star Rating: {map.read().stars}",
                        "{map.read().stars} "
                        img {
                            src: "{mode_img}",
                            width: "24px",
                            height: "24px",
                            style: "filter: {css_filter}; margin-bottom: -6px;"
                        }
                    }
                }
            }
        }
        div{
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
                // rate: map.read().rate
            }
        }
        div {
            class: "button-container",
            title: "Buttons",
            button {
                class: "create-button",
                title: "Create map: This will create a map with the settings you have chosen, you can then play the map in osu!",
                onclick: move |_| if !*generating_map.get(){
                    cx.spawn({
                        generating_map.set(true);
                        msg.write().text = Some("Please wait, generating map...".to_string());
                        msg.write().status = Status::Success;
                        to_owned![map, settings, msg, generating_map];
                        async move{
                            tokio::time::sleep(Duration::from_millis(100)).await; // Wait so the message can be displayed
                            match generate_map(&map.read(), &settings.read()).await{
                                Ok(_) => {
                                    msg.write().text = Some("Map created successfully!".to_string());
                                    msg.write().status = Status::Success;
                                },
                                Err(e) => {
                                    msg.write().text = Some(format!("Error creating map: {}", e));
                                    msg.write().status = Status::Error;
                                }
                            };
                            generating_map.set(false);
                        }
                    })
                },
                Triangles{
                    class_name: "create-triangles",
                    background_color: "var(--darkened-secondary)",
                    triangle_range: "var(--secondary)",
                }
                "Create map"
            }
            button {
                class: "reset-button",
                title: "Reset: Ths will reset the current map settings to the original map settings",
                onclick: move |_| {
                    if let Err(e) = map.write().read_map_metadata(&settings.read()){
                        msg.write().text = Some(format!("Error reading map metadata: {}", e));
                        msg.write().status = Status::Error;
                    }
                },
                Triangles{
                    class_name: "reset-triangles",
                    background_color: "var(--darkened-primary)",
                    triangle_range: "var(--primary)",
                }
                "Reset"
            }
        }
        MessageBox{}
    })
}

fn LockedLock(cx: Scope) -> Element{
    cx.render(rsx!{
        svg {
            class: "lock",
            view_box: "0 0 24.00 24.00",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            transform: "rotate(0)",
            stroke: "#000000",
            stroke_width: "0.00024000000000000003",
            g {
                id: "SVGRepo_bgCarrier",
                stroke_width: "0"
            }
            g {
                id: "SVGRepo_tracerCarrier",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                stroke: "#7d7d7d",
                stroke_width: "4.8",
                path {
                    class: "lock-outer",
                    fill_rule: "evenodd",
                    clip_rule: "evenodd",
                    d: "M5.25 10.0546V8C5.25 4.27208 8.27208 1.25 12 1.25C15.7279 1.25 18.75 4.27208 18.75 8V10.0546C19.8648 10.1379 20.5907 10.348 21.1213 10.8787C22 11.7574 22 13.1716 22 16C22 18.8284 22 20.2426 21.1213 21.1213C20.2426 22 18.8284 22 16 22H8C5.17157 22 3.75736 22 2.87868 21.1213C2 20.2426 2 18.8284 2 16C2 13.1716 2 11.7574 2.87868 10.8787C3.40931 10.348 4.13525 10.1379 5.25 10.0546ZM6.75 8C6.75 5.10051 9.10051 2.75 12 2.75C14.8995 2.75 17.25 5.10051 17.25 8V10.0036C16.867 10 16.4515 10 16 10H8C7.54849 10 7.13301 10 6.75 10.0036V8ZM14 16C14 17.1046 13.1046 18 12 18C10.8954 18 10 17.1046 10 16C10 14.8954 10.8954 14 12 14C13.1046 14 14 14.8954 14 16Z",
                    fill: "#000000"
                }
            }
            g {
                id: "SVGRepo_iconCarrier",
                path {
                    class: "lock-inner",
                    fill_rule: "evenodd",
                    clip_rule: "evenodd",
                    d: "M5.25 10.0546V8C5.25 4.27208 8.27208 1.25 12 1.25C15.7279 1.25 18.75 4.27208 18.75 8V10.0546C19.8648 10.1379 20.5907 10.348 21.1213 10.8787C22 11.7574 22 13.1716 22 16C22 18.8284 22 20.2426 21.1213 21.1213C20.2426 22 18.8284 22 16 22H8C5.17157 22 3.75736 22 2.87868 21.1213C2 20.2426 2 18.8284 2 16C2 13.1716 2 11.7574 2.87868 10.8787C3.40931 10.348 4.13525 10.1379 5.25 10.0546ZM6.75 8C6.75 5.10051 9.10051 2.75 12 2.75C14.8995 2.75 17.25 5.10051 17.25 8V10.0036C16.867 10 16.4515 10 16 10H8C7.54849 10 7.13301 10 6.75 10.0036V8ZM14 16C14 17.1046 13.1046 18 12 18C10.8954 18 10 17.1046 10 16C10 14.8954 10.8954 14 12 14C13.1046 14 14 14.8954 14 16Z",
                    fill: "#000000"
                }
            }
        }
    })
}

fn UnlockedLock(cx: Scope) -> Element{
    cx.render(rsx!{
        svg { 
            class: "lock",
            view_box: "0 0 24.00 24.00",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            g {
                id: "SVGRepo_bgCarrier",
                stroke_width: "0"
            } 
            g {
                id: "SVGRepo_tracerCarrier",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                stroke: "#CCCCCC",
                stroke_width: "4.8",
                path {
                    class: "lock-outer",
                    fill_rule: "evenodd",
                    clip_rule: "evenodd",
                    d: "M6.75 8C6.75 5.10051 9.10051 2.75 12 2.75C14.4453 2.75 16.5018 4.42242 17.0846 6.68694C17.1879 7.08808 17.5968 7.32957 17.9979 7.22633C18.3991 7.12308 18.6405 6.7142 18.5373 6.31306C17.788 3.4019 15.1463 1.25 12 1.25C8.27208 1.25 5.25 4.27208 5.25 8V10.0546C4.13525 10.1379 3.40931 10.348 2.87868 10.8787C2 11.7574 2 13.1716 2 16C2 18.8284 2 20.2426 2.87868 21.1213C3.75736 22 5.17157 22 8 22H16C18.8284 22 20.2426 22 21.1213 21.1213C22 20.2426 22 18.8284 22 16C22 13.1716 22 11.7574 21.1213 10.8787C20.2426 10 18.8284 10 16 10H8C7.54849 10 7.13301 10 6.75 10.0036V8ZM14 16C14 17.1046 13.1046 18 12 18C10.8954 18 10 17.1046 10 16C10 14.8954 10.8954 14 12 14C13.1046 14 14 14.8954 14 16Z",
                    fill: "#000000"
                    }
            }
            g {
                id: "SVGRepo_iconCarrier",
                path {
                    class: "lock-inner",
                    fill_rule: "evenodd",
                    clip_rule: "evenodd",
                    d: "M6.75 8C6.75 5.10051 9.10051 2.75 12 2.75C14.4453 2.75 16.5018 4.42242 17.0846 6.68694C17.1879 7.08808 17.5968 7.32957 17.9979 7.22633C18.3991 7.12308 18.6405 6.7142 18.5373 6.31306C17.788 3.4019 15.1463 1.25 12 1.25C8.27208 1.25 5.25 4.27208 5.25 8V10.0546C4.13525 10.1379 3.40931 10.348 2.87868 10.8787C2 11.7574 2 13.1716 2 16C2 18.8284 2 20.2426 2.87868 21.1213C3.75736 22 5.17157 22 8 22H16C18.8284 22 20.2426 22 21.1213 21.1213C22 20.2426 22 18.8284 22 16C22 13.1716 22 11.7574 21.1213 10.8787C20.2426 10 18.8284 10 16 10H8C7.54849 10 7.13301 10 6.75 10.0036V8ZM14 16C14 17.1046 13.1046 18 12 18C10.8954 18 10 17.1046 10 16C10 14.8954 10.8954 14 12 14C13.1046 14 14 14.8954 14 16Z",
                    fill: "#000000"
                }
            }
        }
    })
}

fn MessageBox(cx: Scope) -> Element{
    let msg = use_shared_state::<StatusMessage>(cx)?;

    // Determine status message color
    let status_color = match msg.read().status{
        Status::Success => "#87E37D",
        Status::Error => "#FF6B6B"
    };

    cx.render(rsx!{
        div {
            title: "Messages",
            class: "message-box",
            p {
                if let Some(msg_text) = &msg.read().text{
                    rsx! {
                        div{
                            class: "status-message",
                            style: "background-color: {status_color};",
                            div{
                                class: "close-button",
                                title: "Close message",
                                onclick: move|_|{
                                    msg.write().text = None;
                                },
                                "X"
                            }
                            p{
                                style: "text-align: center;",
                                title: "{msg_text}",
                                "{msg_text}"
                            }
                        }
                    }
                }
            }
        }
    })
}

/// FLoating triangles behind the create and reset buttons
#[inline_props]
fn Triangles<'a>(cx: Scope, background_color: &'a str, triangle_range: &'a str, class_name: &'a str) -> Element{
    cx.render(rsx!{
        style{
            r#"       
            .{class_name}:nth-child(1n){{
              border-bottom-color: hsl(from {triangle_range} h s calc(l - 2%));
            }}

            .{class_name}:nth-child(2n){{
              border-bottom-color: hsl(from {triangle_range} h s calc(l - 4%));
            }}

            .{class_name}:nth-child(3n){{
              border-bottom-color: hsl(from {triangle_range} h s calc(l - 6%));
            }}

            .{class_name}:nth-child(4n){{
              border-bottom-color: hsl(from {triangle_range} h s calc(l - 8%));
            }}

            .{class_name}:nth-child(5n){{
              border-bottom-color: hsl(from {triangle_range} h s calc(l - 10%));
            }}

            .{class_name}:nth-child(6n){{
              border-bottom-color: hsl(from {triangle_range} h s calc(l - 12%));
            }}

            .{class_name}:nth-child(7n){{
              border-bottom-color: hsl(from {triangle_range} h s calc(l + 2%));
            }}

            .{class_name}:nth-child(8n){{
              border-bottom-color: hsl(from {triangle_range} h s calc(l + 4%));
            }}

            .{class_name}:nth-child(9n){{
              border-bottom-color: hsl(from {triangle_range} h s calc(l + 6%));
            }}

            .{class_name}:nth-child(10n){{
              border-bottom-color: hsl(from {triangle_range} h s calc(l + 8%));
            }}

            .{class_name}:nth-child(11n){{
              border-bottom-color: hsl(from {triangle_range} h s calc(l + 10%));
            }}

            .{class_name}:nth-child(12n){{
              border-bottom-color: hsl(from {triangle_range} h s calc(l + 12%));
            }}
        "#
        }
        div{
            class: "triangle-container",
            style: "background-color: {background_color};",
            div{ class: "triangle-up {class_name}", style: "--size: 13px; --speed: 61; --start: -8%" }
            div{ class: "triangle-up {class_name}", style: "--size: 29px; --speed: 60; --start: 23%" }
            div{ class: "triangle-up {class_name}", style: "--size: 19px; --speed: 59; --start: 54%" }
            div{ class: "triangle-up {class_name}", style: "--size: 20px; --speed: 58; --start: -1%" }
            div{ class: "triangle-up {class_name}", style: "--size: 27px; --speed: 57; --start: 18%" }
            div{ class: "triangle-up {class_name}", style: "--size: 12px; --speed: 56; --start: 59%" }
            div{ class: "triangle-up {class_name}", style: "--size: 19px; --speed: 55; --start: 94%" }
            div{ class: "triangle-up {class_name}", style: "--size: 12px; --speed: 54; --start: 24%" }
            div{ class: "triangle-up {class_name}", style: "--size: 14px; --speed: 53; --start: 83%" }
            div{ class: "triangle-up {class_name}", style: "--size: 23px; --speed: 52; --start: -2%" }
            div{ class: "triangle-up {class_name}", style: "--size: 30px; --speed: 51; --start: 47%" }
            div{ class: "triangle-up {class_name}", style: "--size: 19px; --speed: 1; --start: 2%" }
            div{ class: "triangle-up {class_name}", style: "--size: 17px; --speed: 2; --start: 75%" }
            div{ class: "triangle-up {class_name}", style: "--size: 18px; --speed: 3; --start: 34%" }
            div{ class: "triangle-up {class_name}", style: "--size: 13px; --speed: 4; --start: 99%" }
            div{ class: "triangle-up {class_name}", style: "--size: 23px; --speed: 5; --start: 3%" }
            div{ class: "triangle-up {class_name}", style: "--size: 18px; --speed: 6; --start: 71%" }
            div{ class: "triangle-up {class_name}", style: "--size: 10px; --speed: 7; --start: 14%" }
            div{ class: "triangle-up {class_name}", style: "--size: 21px; --speed: 8; --start: 17%" }
            div{ class: "triangle-up {class_name}", style: "--size: 10px; --speed: 9; --start: 18%" }
            div{ class: "triangle-up {class_name}", style: "--size: 26px; --speed: 10; --start: 19%" }
            div{ class: "triangle-up {class_name}", style: "--size: 26px; --speed: 11; --start: 67%" }
            div{ class: "triangle-up {class_name}", style: "--size: 15px; --speed: 12; --start: 95%" }
            div{ class: "triangle-up {class_name}", style: "--size: 10px; --speed: 13; --start: 29%" }
            div{ class: "triangle-up {class_name}", style: "--size: 18px; --speed: 14; --start: 10%" }
            div{ class: "triangle-up {class_name}", style: "--size: 14px; --speed: 15; --start: -21%" }
            div{ class: "triangle-up {class_name}", style: "--size: 15px; --speed: 16; --start: 32%" }
            div{ class: "triangle-up {class_name}", style: "--size: 12px; --speed: 17; --start: 91%" }
            div{ class: "triangle-up {class_name}", style: "--size: 14px; --speed: 18; --start: 36%" }
            div{ class: "triangle-up {class_name}", style: "--size: 13px; --speed: 19; --start: 64%" }
            div{ class: "triangle-up {class_name}", style: "--size: 22px; --speed: 20; --start: 27%" }
            div{ class: "triangle-up {class_name}", style: "--size: 27px; --speed: 21; --start: 42%" }
            div{ class: "triangle-up {class_name}", style: "--size: 10px; --speed: 22; --start: 78%" }
            div{ class: "triangle-up {class_name}", style: "--size: 19px; --speed: 23; --start: 46%" }
            div{ class: "triangle-up {class_name}", style: "--size: 24px; --speed: 24; --start: 21%" }
            div{ class: "triangle-up {class_name}", style: "--size: 11px; --speed: 25; --start: 42%" }
            div{ class: "triangle-up {class_name}", style: "--size: 10px; --speed: 26; --start: 0%" }
            div{ class: "triangle-up {class_name}", style: "--size: 16px; --speed: 27; --start: 54%" }
            div{ class: "triangle-up {class_name}", style: "--size: 22px; --speed: 28; --start: 76%" }
            div{ class: "triangle-up {class_name}", style: "--size: 27px; --speed: 29; --start: 58%" }
            div{ class: "triangle-up {class_name}", style: "--size: 11px; --speed: 30; --start: 23%" }
            div{ class: "triangle-up {class_name}", style: "--size: 16px; --speed: 31; --start: 62%" }
            div{ class: "triangle-up {class_name}", style: "--size: 17px; --speed: 32; --start: 69%" }
            div{ class: "triangle-up {class_name}", style: "--size: 22px; --speed: 33; --start: 34%" }
            div{ class: "triangle-up {class_name}", style: "--size: 29px; --speed: 34; --start: 68%" }
            div{ class: "triangle-up {class_name}", style: "--size: 19px; --speed: 35; --start: 70%" }
            div{ class: "triangle-up {class_name}", style: "--size: 11px; --speed: 36; --start: 72%" }
            div{ class: "triangle-up {class_name}", style: "--size: 17px; --speed: 37; --start: -3%" }
            div{ class: "triangle-up {class_name}", style: "--size: 18px; --speed: 38; --start: 76%" }
            div{ class: "triangle-up {class_name}", style: "--size: 21px; --speed: 39; --start: 78%" }
            div{ class: "triangle-up {class_name}", style: "--size: 16px; --speed: 40; --start: 21%" }
            div{ class: "triangle-up {class_name}", style: "--size: 17px; --speed: 41; --start: 82%" }
            div{ class: "triangle-up {class_name}", style: "--size: 21px; --speed: 42; --start: 34%" }
            div{ class: "triangle-up {class_name}", style: "--size: 14px; --speed: 43; --start: -4%" }
            div{ class: "triangle-up {class_name}", style: "--size: 10px; --speed: 44; --start: 76%" }
            div{ class: "triangle-up {class_name}", style: "--size: 30px; --speed: 45; --start: 47%" }
            div{ class: "triangle-up {class_name}", style: "--size: 18px; --speed: 46; --start: 92%" }
            div{ class: "triangle-up {class_name}", style: "--size: 15px; --speed: 47; --start: 18%" }
            div{ class: "triangle-up {class_name}", style: "--size: 23px; --speed: 48; --start: 57%" }
            div{ class: "triangle-up {class_name}", style: "--size: 10px; --speed: 49; --start: 31%" }
            div{ class: "triangle-up {class_name}", style: "--size: 20px; --speed: 50; --start: 58%" }
        }
    })
}

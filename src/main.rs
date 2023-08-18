#![allow(non_snake_case)]
use std::path::PathBuf;
use dioxus::{prelude::*, html::{native_bind::NativeFileEngine, button}};
use dioxus_desktop::{Config, WindowBuilder};
use rfd::FileDialog;
mod props;
use props::*;
use ruso::*;
use crate::structs::Tab;
fn main() {
    dioxus_desktop::launch_cfg(App,
        Config::default().with_window(WindowBuilder::new().with_resizable(true)
        .with_inner_size(dioxus_desktop::wry::application::dpi::LogicalSize::new(400.0, 800.0)))
    );
    unsafe { gstreamer::deinit() };
}

fn App(cx: Scope) -> Element {
    use_shared_state_provider(cx, || ruso::structs::MapOptions::new());
    use_shared_state_provider(cx, || Settings::new());
    use_shared_state_provider(cx, || Tab::Auto);
    let map = use_shared_state::<MapOptions>(cx)?;
    let mut map_clone = MapOptions::new();
    let settings = use_shared_state::<Settings>(cx)?;
    let tab = use_shared_state::<Tab>(cx)?;
    let songs_folder = use_state(cx, || PathBuf::new());
    let selected_map = use_state(cx, || PathBuf::new());
    cx.render(rsx! {
        h2 { "Choose your osu Songs directory!" }
        div {            
            h4 { "Current directory: {songs_folder.display()}" }
            button {
                onclick: move |_| {
                    let dir_picker = FileDialog::new()
                        .set_title("Choose your osu! Songs directory");
                    map.write().songs_path = dir_picker.pick_folder().unwrap();
                    songs_folder.set(map.read().songs_path.clone());
                },
                "Choose path"
            }
            if *songs_folder.get() != PathBuf::new(){
               rsx!{
                    h4 { "Selected map: {selected_map.display()}" }
                    button {
                    onclick: move |_| {
                        let map_picker = FileDialog::new()
                            .add_filter("osu! map", &["osu"])
                            .set_title("Choose a map to edit")
                            .set_directory(songs_folder.get());
                        let prefix = map.read().songs_path.clone();
                        map.write().map_path = map_picker.clone().pick_file().unwrap().strip_prefix(prefix).unwrap().to_path_buf();
                        selected_map.set(map.read().map_path.clone());
                        let temp_map = map.read().clone();
                        *map.write() = read_map_metadata(temp_map, &settings.read()).unwrap();
                        map_clone = map.read().clone();
                    },
                    "Choose path"
                    }
                }
            }
            div {
                h2 { "Map Options" }
                GenericSlider {
                    name: "HP Drain",
                    acronym: "HP",
                    read: map.read().hp_drain,
                    on_event: move |ev| map.write().hp_drain = ev
                }
                GenericSlider {
                    name: "Circle Size",
                    acronym: "CS",
                    read: map.read().circle_size,
                    on_event: move |ev| map.write().circle_size = ev
                }
                GenericSlider {
                    name: "Approach Rate",
                    acronym: "AR",
                    read: map.read().approach_rate,
                    on_event: move |ev| map.write().approach_rate = ev
                }
                GenericSlider {
                    name: "Overall Difficulty",
                    acronym: "OD",
                    read: map.read().overall_difficulty,
                    on_event: move |ev| map.write().overall_difficulty = ev,
                }
                RateSlider {
                    on_event: move |ev| map.write().rate = ev
                }
            }
            div {
                title: "Buttons",
                button {
                    onclick: move |_| {
                        generate_map(&map.read()).unwrap();
                    },
                    "Create map"
                }
                button {
                    onclick: move |_| {
                    },
                    "Reset"
                }
            }
        }
    })
}

fn GenericSlider<'a>(cx: Scope<'a, SliderProps<'a>>) -> Element{
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
        }
    })
}

#[inline_props]
fn RateSlider<'a>(cx: Scope, on_event: EventHandler<'a, f64>) -> Element{
    let map = use_shared_state::<MapOptions>(cx).unwrap();
    let value = use_state(cx, || 1.0);
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
                    let temp_val = round_dec(*value.get() - ev.data.delta().strip_units().y / 3000.0, 2);
                    if temp_val > 10.0 {
                        value.set(10.0);
                    } else if temp_val < 0.05 {
                        value.set(0.05);
                    } else {
                        value.set(temp_val);
                    }
                    cx.props.on_event.call(*value.get());
                },
                oninput: move |ev|{
                    value.set(ev.data.value.parse::<f64>().unwrap() / 20.0);
                    cx.props.on_event.call(*value.get());
                },
            }
            input { 
                r#type: "number",
                min: 0.05,
                max: 40,
                step: 0.05,
                value: *value.get(),
                id: "Rate_number",
                onwheel: move |ev|{
                    let temp_val = round_dec(*value.get() - ev.data.delta().strip_units().y / 3000.0, 2);
                    if temp_val > 40.0 {
                        value.set(40.0);
                    } else if temp_val < 0.05 {
                        value.set(0.05);
                    } else {
                        value.set(temp_val);
                    }
                    cx.props.on_event.call(*value.get());
                },
                onchange: move |ev|{
                    let temp_val = ev.data.value.parse::<f64>().unwrap_or(*value.get());
                    if temp_val > 40.0 {
                        value.set(40.0);
                    } else if temp_val < 0.5 {
                        value.set(0.05);
                    } else {
                        value.set(temp_val);
                    }
                    cx.props.on_event.call(*value.get());
                },
            }
        }
    })
}

#![allow(non_snake_case)]
use std::path::PathBuf;
use dioxus::{prelude::*, html::{native_bind::NativeFileEngine, button}};
use dioxus_desktop::{Config, WindowBuilder};
use ruso::*;
use rfd::FileDialog;
mod props;
use crate::props::SliderProps;
fn main() {
    // launch the dioxus app in a webview
    dioxus_desktop::launch_cfg(App,
        Config::default().with_window(WindowBuilder::new().with_resizable(true)
        .with_inner_size(dioxus_desktop::wry::application::dpi::LogicalSize::new(400.0, 800.0)))
    );
}

// define a component that renders a div with the text "Hello, world!"
fn App(cx: Scope) -> Element {
    let options = MapOptions::new();
    let mut current_file = use_state(cx, || PathBuf::new());
    let file_picker = FileDialog::new()
        .set_title("Choose your osu! Songs directory");
    cx.render(rsx! {
        h2 { "Choose your osu Songs directory!" }
        div {            
            h4 { "Current directory: {current_file.display()}" }
            button {
                onclick: (move |_| {
                    current_file.set(file_picker.clone().pick_folder().unwrap())
                }),
                "Choose path"
            }
            GenericSlider {
                name: "Approach Rate",
                acronym: "AR"
            }
        }
    })
}

fn GenericSlider<'a>(cx: Scope<'a, SliderProps<'a>>) -> Element{
    let value = use_state(cx, || 5.0);
    cx.render(rsx! {
        div {
            title: "{cx.props.name}",
            h6 { "{cx.props.acronym}" }
            input {
                r#type: "range",
                min: 0,
                max: 100,
                value: *value.get() * 10.0,
                class: "slider",
                id: "myRange",
                onwheel: move |ev|{
                    value.set(round_dec(*value.get() - ev.data.delta().strip_units().y / 1500.0, 2));
                    println!("{}", *value.get());
                },
                onchange: move |ev|{
                    value.set(ev.data.value.parse::<f64>().unwrap() / 10.0);
                },
            }
            h6 { "{value}" }
        }
    })
}

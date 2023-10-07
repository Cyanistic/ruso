use dioxus::prelude::*;

#[derive(Props)]
pub struct SliderProps<'a>{
    pub name: &'a str,
    pub acronym: &'a str,
    pub read: f64,
    pub locked: bool,
    pub on_event: EventHandler<'a, f64>, 
    pub on_lock: EventHandler<'a, bool>, 
}

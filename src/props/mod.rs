use dioxus::prelude::*;

#[derive(Props)]
pub struct SliderProps<'a>{
    pub name: &'a str,
    pub acronym: &'a str,
    pub read: f64,
    pub on_event: EventHandler<'a, f64>, 
    // pub bind: Option<&'a mut f64>,
}

use dioxus::prelude::Props;

#[derive(Props)]
pub struct SliderProps<'a>{
    name: &'a str,
    acronym: &'a str
}

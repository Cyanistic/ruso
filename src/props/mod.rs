use dioxus::prelude::Props;

#[derive(Props)]
pub struct SliderProps<'a>{
    pub name: &'a str,
    pub acronym: &'a str
}

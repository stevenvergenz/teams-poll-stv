use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    rsx! {
        div {
            id: "home",
            h1 { "Dioxus" }
            p { "A fullstack web framework for Rust." }
        }
    }
}

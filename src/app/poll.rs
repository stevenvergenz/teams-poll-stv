use dioxus::prelude::*;

#[component]
pub fn Poll(id: String) -> Element {
    rsx! {
        div {
            id: "poll",
            h1 { "Poll" }
            p { "A poll viewer." }
        }
    }
}

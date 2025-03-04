use dioxus::prelude::*;

#[component]
pub fn New() -> Element {
    rsx! {
        div {
            id: "new",
            h1 { "New" }
            p { "A new poll wizard." }
        }
    }
}

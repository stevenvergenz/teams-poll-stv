use dioxus::prelude::*;
use super::{
    assets::{FAVICON, TAILWIND_CSS},
    route::Route,
};

#[component]
pub fn Layout() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        div {
            id: "layout",
            class: "min-h-screen flex flex-col items-center bg-light-bg1",
            header {
                class: "w-page bg-light-bg2",
                nav {
                    class: "flex flex-row py-2 gap-2",
                    Link { to: Route::Home {}, "Home" }
                    Link { to: Route::New {}, "New" }
                }
            }
            div {
                id: "content",
                class: "w-page flex-grow",
                Outlet::<Route> {}
            }
            footer {
                class: "w-page bg-light-bg2 py-2 text-center",
                "© 2025 Microsoft Corporation. All rights reserved."
            }
        }
    }
}

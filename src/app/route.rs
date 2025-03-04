use dioxus::prelude::*;

use super::{
    home::Home,
    layout::Layout,
    new::New,
    poll::Poll,
};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Layout)]
    #[route("/")]
    Home {},

    #[route("/new")]
    New {},

    #[route("/poll/:id")]
    Poll { id: String },
}

#[component]
pub fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

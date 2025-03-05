use dioxus::prelude::*;
use uuid::Uuid;
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
    Poll { id: Uuid },
}

#[component]
pub fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

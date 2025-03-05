use dioxus::prelude::*;
use crate::{
    rest_api::poll_api::list,
    voting::Poll,
};

#[component]
pub fn Home() -> Element {
    let polls_future = use_server_future(list);

    let poll_items = if let Ok(polls_res) = polls_future {
        if let Ok(polls) = polls_res.value().try_read() {
            polls.iter().map(|poll: &Poll| {
                rsx! {
                    li {
                        "&poll.title"
                    }
                }
            }).collect()
        }
        else {
            vec![]
        }
    }
    else {
        vec![]
    };

    rsx! {
        div {
            id: "home",
            h1 { "Dioxus" }
            p { "A fullstack web framework for Rust." }
        }
        div {
            {poll_items}
        }
    }
}

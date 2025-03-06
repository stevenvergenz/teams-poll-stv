use dioxus::prelude::*;
use uuid::Uuid;
use crate::rest_api::poll_api::get_ssr;

#[component]
pub fn Poll(id: Uuid) -> Element {
    let poll_future = use_server_future(move || get_ssr(id))?;
    let readable = &*poll_future.read();
    let poll = match readable {
        None => {
            return rsx! {
                div {
                    h1 { "Loading..." }
                }
            };
        },
        Some(Err(e)) => {
            return rsx! {
                div {
                    h1 { "Error" }
                    p { "{e}" }
                }
            };
        },
        Some(Ok(poll)) => poll,
    };

    rsx! {
        div {
            id: "poll",
            h1 {
                { poll.title.as_str() }
            }
            ul {
                for option in poll.options.as_ref().unwrap().iter() {
                    li {
                        { option.description.as_str() }
                    }
                }
            }
        }
    }
}

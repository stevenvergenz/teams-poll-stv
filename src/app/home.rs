use dioxus::prelude::*;
use crate::rest_api::poll_api::list_ssr;

#[component]
pub fn Home() -> Element {
    let future = use_server_future(list_ssr)?;
    let readable = &*future.read();
    let polls = match readable {
        None => {
            return rsx! {
                div {
                    h2 { "Loading..." }
                }
            };
        },
        Some(Err(e)) => {
            return rsx! {
                div {
                    h2 { "Error" }
                    p { "{e}" }
                }
            };
        },
        Some(Ok(polls)) => polls,
    };

    rsx! {
        div {
            id: "home",
            h1 { "Polls" }
        }
        div {
            p { "{polls.len()} items" }
            ul {
                for p in polls {
                    li {
                        Link {
                            to: format!("/poll/{}", p.id),
                            title: p.title.as_str(),
                            { p.title.as_str() }
                        }
                    }
                }
            }
        }
    }
}

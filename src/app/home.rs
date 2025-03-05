use dioxus::prelude::*;
use crate::rest_api::poll_api::list_ssr;

#[component]
pub fn Home() -> Element {
    let polls = use_server_future(list_ssr)?;
    let poll_items = match &*polls.read_unchecked() {
        Some(Ok(polls)) => {
            rsx! {
                p { "{polls.len()} items" }
                ul {
                    for p in polls {
                        li {
                            Link {
                                to: format!("/poll/{}", p.id),
                                title: p.title.as_str(),
                            }
                        }
                    }
                }
            }
        },
        Some(Err(e)) => {
            rsx! {
                div {
                    h2 { "Error" }
                    p { "{e}" }
                }
            }
        },
        None => {
            rsx! {
                div {
                    h2 { "Loading..." }
                }
            }
        }
    };

    rsx! {
        div {
            id: "home",
            h1 { "Polls" }
        }
        div {
            {poll_items}
        }
    }
}

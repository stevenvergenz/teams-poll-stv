use dioxus::prelude::*;
use crate::{
    app::route::Route, rest_api::poll_api::new_ssr, voting::{CreatePollSettings, UnvalidatedCreatePollSettings}
};

#[component]
pub fn New() -> Element {
    let mut title = use_signal(|| String::new());
    let mut options = use_signal(|| vec![String::from("Option A"), String::from("Option B")]);
    let mut validation = use_signal(|| Option::<String>::None);

    let create = move |e: Event<FormData>| async move {
        e.prevent_default();
        let settings = UnvalidatedCreatePollSettings {
            title: title.read().clone(),
            options: options.read().clone(),
            ..Default::default()
        };

        let settings: CreatePollSettings = match settings.try_into() {
            Ok(s) => s,
            Err(e) => {
                validation.set(Some(e.to_string()));
                return;
            }
        };

        match new_ssr(settings.into()).await {
            Err(e) => {
                validation.set(Some(e.to_string()));
            },
            Ok(poll) => {
                let router = router();
                let dest = NavigationTarget::Internal(Route::Poll { id: poll.id.0 });
                router.replace(dest);
            }
        }
    };

    rsx! {
        div {
            id: "new",
            h1 { "Create a new poll" }
            p { "Fill out the form below to create a new transferrable-vote poll." }

            if let Some(msg) = validation.read().as_ref() {
                div {
                    class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded relative",
                    role: "alert",
                    span { "{msg}" }
                }
            }

            form {
                onsubmit: create,

                div {
                    p { "What is the poll called?" }
                    label {
                        class: "block",
                        "Title: "
                        input {
                            type: "text",
                            name: "title",
                            required: true,
                            value: title,
                            onchange: move |e: Event<FormData>| {
                                title.set(e.value());
                            },
                        }
                    }
                }

                div {
                    class: "my-4",
                    p { "What are the choices?" }
                    for (i, opt) in options.read().iter().enumerate() {
                        label {
                            class: "block my-2",
                            "Option {i + 1}: "
                            input {
                                type: "text",
                                placeholder: "Option X",
                                value: opt.as_str(),
                                onchange: move |e: Event<FormData>| {
                                    let mut o = options.write();
                                    o[i] = e.value();
                                },
                            }
                            button {
                                type: "button",
                                onclick: move |_| {
                                    let mut o = options.write();
                                    o.remove(i);
                                },
                                "Remove"
                            }
                        }
                    }
                    button {
                        type: "button",
                        onclick: move |_| {
                            let mut o = options.write();
                            o.push(String::new());
                        },
                        "Add option"
                    }
                }

                button {
                    type: "submit",
                    "Create poll"
                }
            }
        }
    }
}

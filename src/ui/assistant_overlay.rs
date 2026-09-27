use dioxus::prelude::*;

use crate::assistant::control::chat_with_assistant;

const AURA_AVATAR: Asset = asset!("/assets/aura.png");

#[component]
pub fn AssistantOverlay() -> Element {
    let mut open = use_signal(|| false);
    let mut input = use_signal(String::new);
    let mut history = use_signal(|| Vec::<(String, String)>::new());
    let mut thinking = use_signal(|| false);

    let mut do_send = move || {
        let msg = input().trim().to_string();
        if msg.is_empty() || thinking() {
            return;
        }
        history.write().push(("user".to_string(), msg.clone()));
        input.set(String::new());
        thinking.set(true);

        spawn(async move {
            match chat_with_assistant(msg, "TEXT".to_string(), "TEXT".to_string()).await {
                Ok(response) => {
                    history.write().push(("aura".to_string(), response.message));
                }
                Err(e) => {
                    history.write().push((
                        "aura".to_string(),
                        format!("Je n'ai pas pu répondre : {e}"),
                    ));
                }
            }
            thinking.set(false);
        });
    };

    rsx! {
        if !open() {
            // ============================================================
            //  Bouton flottant AURA — animation au survol uniquement
            // ============================================================
            div {
                class: "aura-fab-wrapper",
                // Anneaux radar (déclenchés au hover via CSS)
                span { class: "aura-radar-ring" }
                span { class: "aura-radar-ring-2" }
                span { class: "aura-radar-ring-3" }

                button {
                    class: "aura-fab",
                    title: "Ouvrir AURA — votre assistante",
                    onclick: move |_| open.set(true),
                    div {
                        class: "aura-fab-avatar",
                        img {
                            src: AURA_AVATAR,
                            alt: "AURA",
                            class: "aura-avatar-img",
                        }
                    }
                }
            }
        } else {
            // ============================================================
            //  Panneau ouvert
            // ============================================================
            div {
                class: "aura-panel",

                // En-tête avec avatar
                div {
                    class: "aura-header",
                    div {
                        class: "aura-avatar-lg",
                        img {
                            src: AURA_AVATAR,
                            alt: "AURA",
                            class: "aura-avatar-img",
                        }
                    }
                    div { class: "aura-id",
                        div { class: "aura-name", "AURA" }
                        div { class: "aura-status",
                            span { class: "aura-dot" }
                            "En ligne"
                        }
                    }
                    button {
                        class: "aura-close",
                        onclick: move |_| open.set(false),
                        "×"
                    }
                }

                // Historique
                div {
                    class: "aura-chat",
                    if history().is_empty() {
                        div {
                            class: "aura-empty",
                            div { class: "aura-empty-title", "Bonjour." }
                            div { class: "aura-empty-text",
                                "Je suis AURA, votre assistante SCI. Posez-moi une question sur vos entités, vos baux, vos échéances ou votre trésorerie."
                            }
                        }
                    }
                    for (role, text) in history().iter() {
                        div {
                            class: if role == "user" { "aura-msg user" } else { "aura-msg aura" },
                            "{text}"
                        }
                    }
                    if thinking() {
                        div {
                            class: "aura-msg aura aura-typing",
                            span { class: "aura-tdot" }
                            span { class: "aura-tdot" }
                            span { class: "aura-tdot" }
                        }
                    }
                }

                // Saisie
                div {
                    class: "aura-input-row",
                    input {
                        class: "aura-input",
                        value: "{input}",
                        placeholder: "Posez votre question…",
                        oninput: move |e| input.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                do_send();
                            }
                        },
                    }
                    button {
                        class: "aura-send",
                        disabled: thinking(),
                        onclick: move |_| do_send(),
                        "→"
                    }
                }
            }
        }
    }
}
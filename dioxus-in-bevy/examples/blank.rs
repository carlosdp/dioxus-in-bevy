use bevy::prelude::*;
use dioxus::prelude::*;
use dioxus_in_bevy::prelude::*;

dioxus_in_bevy::events! {
    my_custom_events:

    onclick: |_trigger: Trigger<Pointer<Click>>| -> () {}
}

dioxus_in_bevy::elements! {
    my_custom_elements:

    node, (Node::default(), Interaction::default()), {
        background_color: Color {
            (background_color_node: &mut BackgroundColor) => {
                background_color_node.0 = background_color
            }
        }
    },
    text, Text::new("uninitialized"), {
        text: String {
            (text_node: &mut Text) => {
                text_node.0 = text
            }
        }
    },
}

dioxus_in_bevy::elements! {
    my_custom_elements_more:

    nade, (Node::default(), Interaction::default()), {
        background_color: Color {
            (background_color_node: &mut BackgroundColor) => {
                background_color_node.0 = background_color
            }
        }
    },
}

dioxus_in_bevy::dioxus_elements! {
    elements: { my_custom_elements, my_custom_elements_more }
    events: { my_custom_events }
}

fn main() {
    bevy::prelude::App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DioxusPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());
    commands.spawn(DioxusRoot::new(App));
}

#[component]
fn App() -> Element {
    rsx! {
        node { background_color: attr(Color::srgb(1.0, 0.0, 0.0)),
            text {
                text: "Hello, world!",
                onclick: |_| {
                    println!("onclick triggered");
                },
                text { text: "Another!" }
            }
        }
    }
}

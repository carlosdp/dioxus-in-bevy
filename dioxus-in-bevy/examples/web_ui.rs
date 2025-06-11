use bevy::prelude::*;
use dioxus::prelude::*;
use dioxus_in_bevy::prelude::*;

fn main() {
    bevy::prelude::App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DioxusPlugin::default())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());
    commands.spawn(DioxusNode::new(App));
}

#[component]
fn App() -> Element {
    let text = use_bevy_update(test_sys);

    rsx! {
        div {
            h1 {
                if let Some(ref text) = *text.read() {
                    "{text}"
                } else {
                    "Loading..."
                }
            }
        }
    }
}

fn test_sys(_: In<()>) -> Option<String> {
    Some("Hello, world!".to_string())
}

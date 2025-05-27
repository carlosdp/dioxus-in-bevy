use bevy::prelude::*;
use dioxus::prelude::*;
use dioxus_in_bevy::prelude::*;

fn main() {
    bevy::prelude::App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DioxusPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());
    commands.spawn(WebNode::new(App));
}

#[component]
fn App() -> Element {
    rsx! {
        div {
            h1 { "Hello, world!" }
        }
    }
}

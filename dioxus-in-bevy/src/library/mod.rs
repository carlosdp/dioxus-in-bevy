use bevy::prelude::*;
use bevy_cosmic_edit::{input::CosmicTextChanged, prelude::*, MaxLines};

dioxus_in_bevy::elements! {
    silly_elements:

    node, (Node::default(), Interaction::default()), {
        background_color: Color {
            (background_color_node: &mut BackgroundColor) => {
                background_color_node.0 = background_color
            }
        },

        params: Node {
            (params_node: &mut Node) => {
                params_node.set_if_neq(params);
            }
        }
    },

    text, Text::default(), {
        text: String {
            (text_node: &mut Text) => {
                text_node.0 = text
            }
        },
    },

    textinput, (TextEdit, CosmicEditBuffer::default(), MaxLines(1)), {
        @init: <this>(_none: ())[mut commands: Commands] => {
            commands.entity(this).observe(focus_on_click);
        },

        params: Node {
            (params_node: &mut Node) => {
                params_node.set_if_neq(params);
            }
        }
    }
}

dioxus_in_bevy::events! {
    silly_events:

    onclick: |_trigger: Trigger<Pointer<Click>>| -> () {}
    ontextchange: |trigger: Trigger<TextChanged>| -> String {
        trigger.text.clone()
    }
}

dioxus_in_bevy::dioxus_elements! {
    elements: { silly_elements }
    events: { silly_events }
}

pub struct CosmicTextExtPlugin;

impl Plugin for CosmicTextExtPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, rebroadcast_text_change);
    }
}

#[derive(Event)]
pub struct TextChanged {
    pub text: String,
}

fn rebroadcast_text_change(mut commands: Commands, mut ev_reader: EventReader<CosmicTextChanged>) {
    for CosmicTextChanged((entity, text)) in ev_reader.read() {
        commands.trigger_targets(TextChanged { text: text.clone() }, *entity);
    }
}

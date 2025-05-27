use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    component::EventHandlerInstaller, DioxusBuilders, DioxusRoot, ElementTag, EventChannels,
};
use bevy::prelude::*;
use dioxus_core::{AttributeValue, TemplateAttribute, TemplateNode, WriteMutations};

pub struct WorldRenderer {
    root_entity: Entity,
    stack: Arc<Mutex<Vec<Entity>>>,
    commands: Vec<Box<dyn FnOnce(&mut World) + 'static>>,
}

impl WorldRenderer {
    pub fn new(root_entity: Entity) -> Self {
        Self {
            root_entity,
            stack: Arc::new(Mutex::new(Vec::new())),
            commands: Vec::new(),
        }
    }

    pub fn command(&mut self, command: impl FnOnce(&mut World) + 'static) {
        self.commands.push(Box::new(command));
    }

    pub fn drain_commands(&mut self) -> Vec<Box<dyn FnOnce(&mut World) + 'static>> {
        self.commands.drain(..).collect()
    }
}

impl WriteMutations for WorldRenderer {
    fn append_children(&mut self, id: dioxus_core::ElementId, m: usize) {
        tracing::trace!("append_children: {:?}", id);
        let stack = self.stack.clone();
        let root_entity = self.root_entity;

        self.command(move |world| {
            let parent = id_to_entity(world, root_entity, id);
            let mut stack = stack.lock().unwrap();
            let len = stack.len();
            for child in stack.drain(len - m..) {
                world.entity_mut(parent).add_child(child);
            }
        });
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: dioxus_core::ElementId) {
        tracing::trace!("assign_node_id: {:?}", id);
        let stack = self.stack.clone();
        let root_entity = self.root_entity;

        self.command(move |world| {
            let stack = stack.lock().unwrap();
            let mut node = stack.last().unwrap();
            for index in path {
                let children = world.get::<Children>(*node).unwrap();
                node = children.get(*index as usize).unwrap();
            }
            assign_entity_to_id(world, root_entity, id, *node);
        });
    }

    fn create_event_listener(&mut self, name: &'static str, id: dioxus_core::ElementId) {
        tracing::trace!("create_event_listener: {}", name);
        let root_entity = self.root_entity;

        self.command(move |world| {
            let entity = id_to_entity(world, root_entity, id);

            if let Some(installer) = inventory::iter::<EventHandlerInstaller>().find(|i| {
                if i.name.starts_with("on") {
                    &i.name[2..].to_string() == name
                } else {
                    i.name == name
                }
            }) {
                (installer.handler)(world, root_entity, entity, id);
            }
        });
    }

    fn create_placeholder(&mut self, id: dioxus_core::ElementId) {
        tracing::trace!("create_placeholder: {:?}", id);
        let stack = self.stack.clone();
        let root_entity = self.root_entity;

        self.command(move |world| {
            let entity = spawn_entity(world, root_entity, id);
            let mut stack = stack.lock().unwrap();
            stack.push(entity);
        });
    }

    fn create_text_node(&mut self, value: &str, id: dioxus_core::ElementId) {
        tracing::trace!("create_text_node: {:?}", id);
        let stack = self.stack.clone();
        let value = value.to_string();
        let root_entity = self.root_entity;

        self.command(move |world| {
            let entity = spawn_entity(world, root_entity, id);
            world.entity_mut(entity).insert(Text::new(value));
            let mut stack = stack.lock().unwrap();
            stack.push(entity);
        });
    }

    fn insert_nodes_after(&mut self, id: dioxus_core::ElementId, m: usize) {
        tracing::trace!("insert_nodes_after: {:?}", id);
        let stack = self.stack.clone();
        let root_entity = self.root_entity;

        self.command(move |world| {
            let reference_child = id_to_entity(world, root_entity, id);
            let parent = world.get::<ChildOf>(reference_child).unwrap().parent();

            let mut stack = stack.lock().unwrap();
            let len = stack.len();
            let new_children = stack.drain(len - m..).collect::<Vec<_>>();
            for child in &new_children {
                world.entity_mut(parent).add_child(*child);
            }

            // Reorder children to ensure newly added ones appear after the reference child
            let mut children = world.get_mut::<Children>(parent).unwrap();
            let ref_idx = children.iter().position(|c| c == reference_child).unwrap();
            let child_count = children.len();

            // Move each new child to position after reference_child
            for (i, _) in new_children.iter().enumerate() {
                // Calculate the current position of this new child
                // It will be at the end of the children list
                let current_pos = child_count - new_children.len() + i;

                // Move it to just after the reference child
                // We need to move each child one by one, incrementing the target position
                let target_pos = ref_idx + 1 + i;

                // Swap positions until the child is in the right place
                for pos in (target_pos..current_pos).rev() {
                    children.swap(pos, pos + 1);
                }
            }
        });
    }

    fn insert_nodes_before(&mut self, id: dioxus_core::ElementId, m: usize) {
        tracing::trace!("insert_nodes_before: {:?}", id);
        let stack = self.stack.clone();
        let root_entity = self.root_entity;

        self.command(move |world| {
            let reference_child = id_to_entity(world, root_entity, id);
            let parent = world.get::<ChildOf>(reference_child).unwrap().parent();

            let mut stack = stack.lock().unwrap();
            let len = stack.len();
            let new_children = stack.drain(len - m..).collect::<Vec<_>>();
            for child in &new_children {
                world.entity_mut(parent).add_child(*child);
            }

            // Reorder children so the newly added ones appear just before the reference child
            let mut children = world.get_mut::<Children>(parent).unwrap();
            let ref_idx = children.iter().position(|c| c == reference_child).unwrap();
            let child_count = children.len();

            for (i, _) in new_children.iter().enumerate() {
                let current_pos = child_count - new_children.len() + i;
                let target_pos = ref_idx - 1 - i;

                for pos in target_pos..current_pos {
                    children.swap(pos, pos + 1);
                }
            }
        });
    }

    fn load_template(
        &mut self,
        template: dioxus_core::Template,
        index: usize,
        id: dioxus_core::ElementId,
    ) {
        tracing::trace!("load_template: {:?}", id);
        let stack = self.stack.clone();
        let root_entity = self.root_entity;

        self.command(move |world| {
            struct CreateElement<'a> {
                f: &'a dyn Fn(
                    &mut CreateElement,
                    &TemplateNode,
                    &mut World,
                    Option<dioxus_core::ElementId>,
                ),
            }

            let mut create_element = CreateElement {
                f: &|create_element: &mut CreateElement,
                     node: &TemplateNode,
                     world: &mut World,
                     id: Option<dioxus_core::ElementId>| {
                    match node {
                        TemplateNode::Element {
                            tag,
                            namespace: _,
                            attrs,
                            children,
                        } => {
                            let entity = if let Some(builder) =
                                world.get_resource::<DioxusBuilders>().unwrap().get(tag)
                            {
                                let entity = (builder.builder)(world);
                                if let Some(id) = id {
                                    assign_entity_to_id(world, root_entity, id, entity);
                                }
                                world.entity_mut(entity).insert(ElementTag(tag));

                                entity
                            } else {
                                world.spawn(Node::default()).id()
                            };
                            stack.lock().unwrap().push(entity);

                            let builder = world.get_resource::<DioxusBuilders>().unwrap().get(tag);
                            if let Some(builder) = builder {
                                let init_mutator = { builder.initializer };
                                init_mutator(world, entity);
                            }

                            for attr in attrs.iter() {
                                if let TemplateAttribute::Static {
                                    name: attr_name,
                                    value,
                                    ..
                                } = attr
                                {
                                    let builder =
                                        world.get_resource::<DioxusBuilders>().unwrap().get(tag);
                                    if let Some(builder) = builder {
                                        if let Some((_, mutator)) = builder
                                            .attribute_mutators
                                            .iter()
                                            .find(|(name, _)| name == attr_name)
                                        {
                                            mutator(
                                                world,
                                                entity,
                                                AttributeValue::Text(value.to_string()),
                                            );
                                        }
                                    }
                                }
                            }

                            for child in children.iter() {
                                (create_element.f)(create_element, child, world, None);
                            }

                            // Pop children off stack and parent them
                            let m = children.len();
                            let len = stack.lock().unwrap().len();
                            for child in stack.lock().unwrap().drain(len - m..) {
                                world.entity_mut(entity).add_child(child);
                            }
                        }
                        TemplateNode::Text { text } => {
                            let entity = if let Some(id) = id {
                                spawn_entity(world, root_entity, id)
                            } else {
                                world.spawn(Node::default()).id()
                            };
                            world.entity_mut(entity).insert(Text::new(text.to_owned()));
                            stack.lock().unwrap().push(entity);
                        }
                        TemplateNode::Dynamic { .. } => {
                            let entity = if let Some(id) = id {
                                spawn_entity(world, root_entity, id)
                            } else {
                                world.spawn(Node::default()).id()
                            };
                            stack.lock().unwrap().push(entity);
                        }
                    }
                },
            };

            (create_element.f)(&mut create_element, &template.roots[index], world, Some(id));
        });
    }

    fn push_root(&mut self, id: dioxus_core::ElementId) {
        tracing::trace!("push_root: {:?}", id);
        let stack = self.stack.clone();
        let root_entity = self.root_entity;

        self.command(move |world| {
            let entity = id_to_entity(world, root_entity, id);
            let mut stack = stack.lock().unwrap();
            stack.push(entity);
        });
    }

    fn remove_event_listener(&mut self, name: &'static str, _id: dioxus_core::ElementId) {
        tracing::trace!("remove_event_listener: {:?}", name);
        // we can't remove observers
    }

    fn remove_node(&mut self, id: dioxus_core::ElementId) {
        tracing::trace!("remove_node: {:?}", id);
        let root_entity = self.root_entity;
        self.command(move |world| {
            let entity = id_to_entity(world, root_entity, id);
            assert!(world.despawn(entity));
        });
    }

    fn replace_node_with(&mut self, id: dioxus_core::ElementId, m: usize) {
        tracing::trace!("replace_node_with: {:?}", id);
        let stack = self.stack.clone();
        let root_entity = self.root_entity;

        self.command(move |world| {
            let entity = id_to_entity(world, root_entity, id);
            let parent = world.get::<ChildOf>(entity).unwrap().parent();

            // Remove the entity from the parent's children
            let children = world.get_mut::<Children>(parent).unwrap();
            let idx = children.iter().position(|c| c == entity).unwrap();
            world.entity_mut(parent).remove_children(&[entity]);

            // Add the new children from the stack
            let mut stack = stack.lock().unwrap();
            let len = stack.len();
            let new_children = stack.drain(len - m..).collect::<Vec<_>>();
            // Add all new children to the parent entity
            world.entity_mut(parent).add_children(&new_children);

            // Now rearrange the children to place them at the correct position
            let mut children = world.get_mut::<Children>(parent).unwrap();
            let child_count = children.len();

            // Move each new child to the position where the old entity was
            for (i, _) in new_children.iter().enumerate() {
                let current_pos = child_count - new_children.len() + i;
                let target_pos = idx + i;

                // Swap the child into position
                for pos in (target_pos..current_pos).rev() {
                    children.swap(pos, pos + 1);
                }
            }

            // Despawn the replaced entity
            assert!(world.despawn(entity));
        });
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        tracing::trace!("replace_placeholder_with_nodes: {:?}", path);
        let stack = self.stack.clone();

        self.command(move |world| {
            let mut stack = stack.lock().unwrap();
            let mut node = stack.last().unwrap();
            for index in path {
                let children = world.get::<Children>(*node).unwrap();
                node = children.get(*index as usize).unwrap();
            }
            let node = *node;

            let parent = world.get::<ChildOf>(node).unwrap().parent();

            // Remove the entity from the parent's children
            let children = world.get_mut::<Children>(parent).unwrap();
            let idx = children.iter().position(|c| c == node).unwrap();
            world.entity_mut(parent).remove_children(&[node]);

            // Add the new children from the stack
            let len = stack.len();
            let new_children = stack.drain(len - m..).collect::<Vec<_>>();
            // Add all new children to the parent entity
            world.entity_mut(parent).add_children(&new_children);

            // Now rearrange the children to place them at the correct position
            let mut children = world.get_mut::<Children>(parent).unwrap();
            let child_count = children.len();

            // Move each new child to the position where the old entity was
            for (i, _) in new_children.iter().enumerate() {
                let current_pos = child_count - new_children.len() + i;
                let target_pos = idx + i;

                // Swap the child into position
                for pos in (target_pos..current_pos).rev() {
                    children.swap(pos, pos + 1);
                }
            }

            // Despawn the replaced entity
            assert!(world.despawn(node));
        });
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        _ns: Option<&'static str>,
        value: &dioxus_core::AttributeValue,
        id: dioxus_core::ElementId,
    ) {
        tracing::trace!("set_attribute: {:?}", name);
        let root_entity = self.root_entity;
        let value = value.clone();

        self.command(move |world| {
            let entity = id_to_entity(world, root_entity, id);
            if let AttributeValue::Listener(event_handler) = value {
                let mut event_channels =
                    world.get_non_send_resource_mut::<EventChannels>().unwrap();
                event_channels
                    .channels
                    .entry(entity)
                    .or_insert_with(HashMap::new)
                    .insert(name, event_handler);
            }

            let tag = world.get::<ElementTag>(entity).unwrap();
            let builder = world.get_resource::<DioxusBuilders>().unwrap().get(tag.0);

            if let Some(builder) = builder {
                if let Some((_, mutator)) = builder
                    .attribute_mutators
                    .iter()
                    .find(|(attr_name, _)| *attr_name == name)
                {
                    mutator(world, entity, value);
                }
            }
        });
    }

    fn set_node_text(&mut self, value: &str, id: dioxus_core::ElementId) {
        tracing::trace!("set_node_text: {:?}", id);
        let value = value.to_string();
        let root_entity = self.root_entity;

        self.command(move |world| {
            let entity = id_to_entity(world, root_entity, id);
            let mut text = world.get_mut::<Text>(entity).unwrap();
            text.0 = value;
        });
    }
}

fn spawn_entity(world: &mut World, root_entity: Entity, id: dioxus_core::ElementId) -> Entity {
    let entity = world.spawn(Node::default()).id();
    world
        .entity_mut(root_entity)
        .get_mut::<DioxusRoot>()
        .unwrap()
        .element_map
        .insert(id, entity);
    entity
}

fn assign_entity_to_id(
    world: &mut World,
    root_entity: Entity,
    id: dioxus_core::ElementId,
    entity: Entity,
) {
    world
        .entity_mut(root_entity)
        .get_mut::<DioxusRoot>()
        .unwrap()
        .element_map
        .insert(id, entity);
}

fn id_to_entity(world: &World, root_entity: Entity, id: dioxus_core::ElementId) -> Entity {
    *world
        .entity(root_entity)
        .get::<DioxusRoot>()
        .unwrap()
        .element_map
        .get(&id)
        .unwrap()
}

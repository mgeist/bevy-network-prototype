use bevy::prelude::*;
use bevy_networking_turbulence::{
    NetworkEvent,
    NetworkResource,
};
use serde::{Deserialize, Serialize};

use crate::{
    Player,
    PlayerMovement,
    server::{
        GameStateMessage,
        ServerMessage,
    },
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    Join,
    Direction(Vec2),
}

#[derive(Debug, Default)]
pub struct ClientsServerState {
    pub has_joined: bool,
    pub handle: u32,
}

pub struct ServerEntityId(u32);
pub struct Frame(u32);

pub fn handle_packets(
    mut net: ResMut<NetworkResource>,
    mut network_events: EventReader<NetworkEvent>,

) {
    for event in network_events.iter() {
        match event {
            NetworkEvent::Connected(handle) => {
                if let Some(_connection) = net.connections.get_mut(handle) {
                    log::info!("Connected on {}", handle);
                    match net.send_message(*handle, ClientMessage::Join) {
                        Ok(msg) => {
                            if let Some(msg) = msg {
                                log::error!("Unable to send Join: {:?}", msg);
                            }
                        },
                        Err(err) => {
                            log::error!("Unable to send Join: {:?}", err);
                        }
                    }
                }
            },
            NetworkEvent::Disconnected(handle) => {
                log::warn!("DISCONNECTED: {}", handle);
            },
            NetworkEvent::Packet(handle, packet) => {
                log::warn!("PACKET FROM {}: {:?}", handle, packet);
            },
            NetworkEvent::Error(handle, err) => {
                log::warn!("ERROR ON {}: {:?}", handle, err);
            },
        }
    }
}

pub fn handle_messages(
    mut commands: Commands,
    mut net: ResMut<NetworkResource>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut clients_server_handle: ResMut<ClientsServerState>,
    mut player_query: Query<(&ServerEntityId, &mut PlayerMovement, &mut Transform, &mut Frame)>
) {
    for (handle, connection) in net.connections.iter_mut() {
        let channels = connection.channels().unwrap();
        while let Some(server_message) = channels.recv::<ServerMessage>() {
            log::debug!("ServerMessage received on {}: {:?}", handle, server_message);
            match server_message {
                ServerMessage::Joined(client_handle) => {
                    log::debug!("Server connected, client's handle is: {}", client_handle);
                    clients_server_handle.has_joined = true;
                    clients_server_handle.handle = client_handle;
                    commands
                        .spawn_bundle(SpriteBundle {
                            material: materials.add(Color::ALICE_BLUE.into()),
                            sprite: Sprite::new(Vec2::new(32.0, 32.0)),
                            ..Default::default()
                        })
                        .insert(ServerEntityId(client_handle))
                        .insert(Frame(0))
                        .insert(PlayerMovement(Vec2::ZERO))
                        .insert(Player);
                }
            }
        }

        while let Some(mut state_message) = channels.recv::<GameStateMessage>() {
            let message_frame = state_message.frame;
            log::debug!("GameStateMessage received on {}: {:?}", handle, state_message);

            for &server_entity_id in state_message.new_players.iter() {
                let is_my_entity = server_entity_id == clients_server_handle.handle;

                if is_my_entity {
                    continue;
                }

                commands
                    .spawn_bundle(SpriteBundle {
                        material: materials.add(Color::BLACK.into()),
                        sprite: Sprite::new(Vec2::new(32.0, 32.0)),
                        ..Default::default()
                    })
                    .insert(ServerEntityId(server_entity_id))
                    .insert(Frame(message_frame))
                    .insert(PlayerMovement(Vec2::ZERO));
            }

            for (server_entity_id, mut movement, mut transform, mut frame) in player_query.iter_mut() {
                if frame.0 > message_frame {
                    continue;
                }

                if let Some(index) = state_message.players.iter().position(|&p| p.0 == server_entity_id.0) {
                    let (_id, movement_vec2, translation) = state_message.players.remove(index);

                    frame.0 = message_frame;
                    movement.0 = movement_vec2;
                    transform.translation = translation;
                }
            }
        }
    }
}

pub fn player_movement(
    mut net: ResMut<NetworkResource>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    let x_axis = -(keyboard_input.pressed(KeyCode::A) as i8)
        + (keyboard_input.pressed(KeyCode::D) as i8);
    let y_axis = -(keyboard_input.pressed(KeyCode::S) as i8)
        + (keyboard_input.pressed(KeyCode::W) as i8);

    net.broadcast_message(ClientMessage::Direction(Vec2::new(x_axis as f32, y_axis as f32)));
}
use bevy::prelude::*;
use bevy_networking_turbulence::{NetworkEvent, NetworkResource};
use serde::{Deserialize, Serialize};

use crate::{client::ClientMessage, MOVEMENT_SPEED, SERVER_TIME_STEP};
use crate::{ControllingHandle, PlayerMovement};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameStateMessage {
    pub frame: u32,
    pub players: Vec<(u32, Vec2, Vec3)>,
    pub new_players: Vec<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    Joined(u32),
}

#[derive(Debug, Default)]
pub struct NetworkBroadcast {
    pub frame: u32,
    pub new_players: Vec<u32>,
}

pub fn handle_packets(
    mut net: ResMut<NetworkResource>,
    mut network_events: EventReader<NetworkEvent>,
) {
    for event in network_events.iter() {
        match event {
            NetworkEvent::Connected(handle) => {
                if let Some(connection) = net.connections.get_mut(handle) {
                    if let Some(remote_address) = connection.remote_address() {
                        log::info!("Incoming connection on {} from {}", handle, remote_address);
                    }
                }
            }
            NetworkEvent::Disconnected(handle) => {
                log::info!("DISCONNECTED: {}", handle);
            }
            NetworkEvent::Packet(handle, packet) => {
                log::info!("PACKET FROM {}: {:?}", handle, packet);
            }
            NetworkEvent::Error(handle, err) => {
                log::info!("ERROR ON {}: {:?}", handle, err);
            }
        }
    }
}

pub fn handle_messages(
    mut commands: Commands,
    mut net: ResMut<NetworkResource>,
    mut network_broadcast: ResMut<NetworkBroadcast>,
    mut player_query: Query<(&ControllingHandle, &mut PlayerMovement)>,
) {
    let mut responses = Vec::<(u32, ServerMessage)>::new();

    for (handle, connection) in net.connections.iter_mut() {
        let channels = connection.channels().unwrap();
        while let Some(client_message) = channels.recv::<ClientMessage>() {
            log::debug!("ClientMessage received on {}: {:?}", handle, client_message);
            match client_message {
                ClientMessage::Join => {
                    log::info!("Client connected on {}", handle);
                    responses.push((*handle, ServerMessage::Joined(*handle)));

                    // new client connecting, spawn them an entity
                    commands.spawn_bundle((
                        ControllingHandle(*handle),
                        PlayerMovement(Vec2::ZERO),
                        Transform::identity(),
                    ));
                    network_broadcast.new_players.push(*handle);
                }
                ClientMessage::Direction(dir) => {
                    for (controlling_handle, mut movement) in player_query.iter_mut() {
                        if controlling_handle.0 == *handle {
                            movement.0 = dir;
                        }
                    }
                }
            }
        }
    }

    for (handle, message) in responses {
        log::debug!("Sending on {}: {:?}", handle, message);
        match net.send_message(handle, message) {
            Ok(msg) => {
                if let Some(msg) = msg {
                    log::error!("Unable to send Joined: {:?}", msg);
                }
            }
            Err(err) => {
                log::error!("Unable to send Joined: {:?}", err);
            }
        }
    }
}

pub fn state_broadcast(
    mut state: ResMut<NetworkBroadcast>,
    mut net: ResMut<NetworkResource>,
    player_query: Query<(Entity, &PlayerMovement, &Transform)>,
) {
    let mut message = GameStateMessage {
        frame: state.frame,
        players: Vec::new(),
        new_players: state.new_players.drain(..).collect(),
    };

    state.frame += 1;

    for (entity, movement, transform) in player_query.iter() {
        message
            .players
            .push((entity.id(), movement.0, transform.translation))
    }

    net.broadcast_message(message);
}

pub fn compute_movement(mut player_query: Query<(&PlayerMovement, &mut Transform)>) {
    for (movement, mut transform) in player_query.iter_mut() {
        let mut delta = movement.0;

        if delta.x != 0.0 && delta.y != 0.0 {
            delta = delta.normalize_or_zero();
        }

        delta *= MOVEMENT_SPEED * SERVER_TIME_STEP;
        transform.translation += delta.extend(0.0);
    }
}

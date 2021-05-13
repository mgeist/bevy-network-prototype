
use bevy::{
    app::{ScheduleRunnerPlugin, ScheduleRunnerSettings},
    core::FixedTimestep,
    prelude::*,
    render::pass::ClearColor,
};
use bevy_networking_turbulence::{
    ConnectionChannelsBuilder,
    find_my_ip_address,
    MessageChannelMode,
    MessageChannelSettings,
    NetworkingPlugin,
    NetworkResource,
};
use log::{LevelFilter};
use simple_logger::SimpleLogger;
use std::{
    net::SocketAddr,
    time::Duration,
};

mod client;
mod server;
use client::{
    ClientMessage,
    ClientsServerState,
};
use server::{
    GameStateMessage,
    NetworkBroadcast,
    ServerMessage,
};

// TODO:
//  - Need to send "existing players" on join, rather a complete gamestate

const SERVER_TIME_STEP: f32 = 1.0 / 20.0;
const TIME_STEP: f32 = 1.0 / 60.0;
const MOVEMENT_SPEED: f32 = 300.0;
const SERVER_PORT: u16 = 18321;
// TODO: Convert this to a reliable channel
const CLIENT_STATE_MESSAGE_SETTINGS: MessageChannelSettings = MessageChannelSettings {
    channel: 0,
    channel_mode: MessageChannelMode::Unreliable,
    message_buffer_size: 8,
    packet_buffer_size: 8,
};

const SERVER_STATE_MESSAGE_SETTINGS: MessageChannelSettings = MessageChannelSettings {
    channel: 1,
    channel_mode: MessageChannelMode::Unreliable,
    message_buffer_size: 8,
    packet_buffer_size: 8,
};

const GAME_STATE_MESSAGE_SETTINGS: MessageChannelSettings = MessageChannelSettings {
    channel: 2,
    channel_mode: MessageChannelMode::Unreliable,
    message_buffer_size: 8,
    packet_buffer_size: 8,
};

pub struct Player;

pub struct PlayerMovement(Vec2);

pub struct ControllingHandle(u32);

pub struct IsServer(bool);

fn main() {
    SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();
    let is_server = IsServer(parse_args());

    let mut app = App::build();

    if is_server.0 {
        app
            .insert_resource(
                ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(SERVER_TIME_STEP as f64))
            )
            .add_plugins(MinimalPlugins)
            .add_plugin(ScheduleRunnerPlugin::default())
            .init_resource::<NetworkBroadcast>()
            .add_system(server::handle_packets.system())
            .add_system(server::compute_movement.system())
            .add_system_to_stage(CoreStage::PreUpdate, server::handle_messages.system())
            .add_system_to_stage(CoreStage::PostUpdate, server::state_broadcast.system());
    } else {
        app
            .add_plugins(DefaultPlugins)
            .init_resource::<ClientsServerState>()
            .insert_resource(ClearColor(Color::rgb(0.5, 0.5, 0.5)))
            .add_system(client_setup.system())
            .add_system(client::handle_packets.system())
            .add_system_to_stage(CoreStage::PreUpdate, client::handle_messages.system())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                    .with_system(client::player_movement.system())
            );
    }

    app
        .add_plugin(NetworkingPlugin::default())
        .add_startup_system(network_setup.system())
        .insert_resource(is_server)
        .run();
}

fn parse_args() -> bool {
    let mut is_server = true;

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        return is_server;
    }
    let arg = &args[1];
    is_server = match arg.as_str() {
        "--server" | "-s" => true,
        "--client" | "-c" => false,
        _ => panic!("Invalid option provided. Use one of the following: --server (-s), --client (-c)."),
    };

    return is_server;
}

fn network_setup(
    mut net: ResMut<NetworkResource>,
    is_server: Res<IsServer>,
) {
    let ip_address = find_my_ip_address().expect("Unable to find IP address.");
    let server_address = SocketAddr::new(ip_address, SERVER_PORT);

    net.set_channels_builder(|builder: &mut ConnectionChannelsBuilder| {
        builder
            .register::<ClientMessage>(CLIENT_STATE_MESSAGE_SETTINGS)
            .unwrap();
        builder
            .register::<ServerMessage>(SERVER_STATE_MESSAGE_SETTINGS)
            .unwrap();
        builder
            .register::<GameStateMessage>(GAME_STATE_MESSAGE_SETTINGS)
            .unwrap();
    });

    if is_server.0 {
        net.listen(server_address, None, None);
        log::info!("Starting as server.");
    } else {
        net.connect(server_address);
        log::info!("Starting as client.");
    }

}

fn client_setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

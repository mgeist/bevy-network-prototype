
use bevy::{
    core::FixedTimestep,
    prelude::*,
    render::pass::ClearColor,
};
use bevy_networking_turbulence::{
    find_my_ip_address,
    NetworkEvent,
    NetworkingPlugin,
    NetworkResource,
    Packet,
};
use log::{LevelFilter, info, warn};
use simple_logger::SimpleLogger;

use std::net::SocketAddr;

const TIME_STEP: f32 = 1.0 / 60.0;
const MOVEMENT_SPEED: f32 = 300.0;
const SERVER_PORT: u16 = 18321;

struct Player;

struct IsServer(bool);

fn main() {
    SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();
    let is_server = IsServer(parse_args());

    let mut app = App::build();

    if is_server.0 {
        app
            .add_plugins(MinimalPlugins);
    } else {
        app
            .add_plugins(DefaultPlugins)
            .insert_resource(ClearColor(Color::rgb(0.5, 0.5, 0.5)))
            .add_system(client_setup.system())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                    .with_system(player_movement.system())
            )
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::step(2.0))
                    .with_system(send_packets.system())
            );
    }

    app
        .add_plugin(NetworkingPlugin::default())
        .add_startup_system(network_setup.system())
        .insert_resource(is_server)
        .add_system(handle_packets.system())
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
    if is_server.0 {
        info!("Starting as server.");
        net.listen(server_address, None, None);
    } else {
        info!("Starting as client.");
        net.connect(server_address);
    }
}

fn client_setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::ALICE_BLUE.into()),
            sprite: Sprite::new(Vec2::new(32.0, 32.0)),
            ..Default::default()
        })
        .insert(Player);
}

fn player_movement(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    if let Ok(mut transform) = query.single_mut() {
        let translation = &mut transform.translation;

        let x_axis = -(keyboard_input.pressed(KeyCode::A) as i8)
            + (keyboard_input.pressed(KeyCode::D) as i8);
        let y_axis = -(keyboard_input.pressed(KeyCode::S) as i8)
            + (keyboard_input.pressed(KeyCode::W) as i8);

        let mut move_delta = Vec2::new(x_axis as f32, y_axis as f32);
        if move_delta.x != 0.0 && move_delta.y != 0.0 {
            move_delta = move_delta.normalize_or_zero();
        }

        move_delta *= MOVEMENT_SPEED * TIME_STEP;
        *translation += move_delta.extend(0.0);
    }

}

fn send_packets(
    mut net: ResMut<NetworkResource>,
    is_server: Res<IsServer>,
) {
    if is_server.0 {
        return;
    }

    info!("Sending PING.");
    net.broadcast(Packet::from("PING"));
}

fn handle_packets(
    mut net: ResMut<NetworkResource>,
    mut reader: EventReader<NetworkEvent>,
    time: Res<Time>,
) {
    for event in reader.iter() {
        match event {
            NetworkEvent::Packet(handle, packet) => {
                let message = String::from_utf8_lossy(packet);
                info!("Received packet on {}: {}", handle, message);
                if message == "PING" {
                    let message = format!("PONG @ {}", time.seconds_since_startup());
                    match net.send(*handle, Packet::from(message)) {
                        Ok(()) => {
                            info!("Responded with PONG.");
                        }
                        Err(error) => {
                            warn!("Error responding to PING: {}", error);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
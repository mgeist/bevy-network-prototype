
use bevy::{
    core::FixedTimestep,
    prelude::*,
    render::pass::ClearColor,
};

const TIME_STEP: f32 = 1.0 / 60.0;
const MOVEMENT_SPEED: f32 = 300.0;

struct Player;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::rgb(0.5, 0.5, 0.5)))
        .add_startup_system(setup.system())
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                .with_system(player_movement.system())
        )
        .run();
}

fn setup(
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
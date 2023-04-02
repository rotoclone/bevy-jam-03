use bevy::sprite::MaterialMesh2dBundle;
use bevy_asset_loader::prelude::*;
use iyes_progress::{ProgressCounter, ProgressPlugin};

use crate::*;

const MOVE_LEFT_KEY: KeyCode = KeyCode::A;
const MOVE_RIGHT_KEY: KeyCode = KeyCode::D;
const MOVE_UP_KEY: KeyCode = KeyCode::W;
const MOVE_DOWN_KEY: KeyCode = KeyCode::S;

const ROTATE_CLOCKWISE_KEY: KeyCode = KeyCode::Right;
const ROTATE_COUNTERCLOCKWISE_KEY: KeyCode = KeyCode::Left;

const MOVE_SPEED: f32 = 1.0;
const ROTATE_SPEED: f32 = 0.025;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_loading_state(LoadingState::new(GameState::GameLoading))
            .add_collection_to_loading_state::<_, ImageAssets>(GameState::GameLoading)
            .add_collection_to_loading_state::<_, AudioAssets>(GameState::GameLoading)
            .add_plugin(ProgressPlugin::new(GameState::GameLoading).continue_to(GameState::Game))
            .add_system(display_loading_progress.run_if(in_state(GameState::GameLoading)));

        app.add_system(loading_setup.in_schedule(OnEnter(GameState::GameLoading)))
            .add_system(
                despawn_components_system::<LoadingComponent>
                    .in_schedule(OnExit(GameState::GameLoading)),
            );

        app.add_system(game_setup.in_schedule(OnEnter(GameState::Game)))
            .add_system(
                despawn_components_system::<GameComponent>.in_schedule(OnExit(GameState::Game)),
            );

        app.add_system(move_player);
    }
}

#[derive(AssetCollection, Resource)]
struct ImageAssets {
    //TODO
    #[asset(path = "images/seed.png")]
    test: Handle<Image>,
}

#[derive(AssetCollection, Resource)]
struct AudioAssets {
    //TODO
    #[asset(path = "sounds/victory.ogg")]
    test: Handle<AudioSource>,
}

#[derive(Component)]
struct LoadingComponent;

#[derive(Component)]
struct LoadingText;

#[derive(Component)]
struct GameComponent;

#[derive(Component)]
struct PlayerShape;

/// Sets up the loading screen.
fn loading_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(
            TextBundle::from_section(
                "Loading...\n0%",
                TextStyle {
                    font: asset_server.load(MAIN_FONT),
                    font_size: 50.0,
                    color: Color::WHITE,
                },
            )
            .with_text_alignment(TextAlignment::Center)
            .with_style(Style {
                margin: UiRect::all(Val::Auto),
                ..default()
            }),
        )
        .insert(LoadingComponent)
        .insert(LoadingText);
}

fn display_loading_progress(
    progress: Option<Res<ProgressCounter>>,
    mut loading_text_query: Query<&mut Text, With<LoadingText>>,
    mut last_done: Local<u32>,
) {
    if let Some(progress) = progress.map(|counter| counter.progress()) {
        if progress.done > *last_done {
            *last_done = progress.done;
            let percent_done = (progress.done as f32 / progress.total as f32) * 100.0;
            for mut loading_text in loading_text_query.iter_mut() {
                loading_text.sections[0].value = format!("Loading...\n{percent_done:.0}%");
            }
        }
    }
}

/// Sets up the game.
fn game_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands
        .spawn(MaterialMesh2dBundle {
            mesh: meshes.add(shape::RegularPolygon::new(50., 3).into()).into(),
            material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..default()
        })
        .insert(PlayerShape);
}

fn move_player(
    mut player_shape_query: Query<&mut Transform, With<PlayerShape>>,
    keycode: Res<Input<KeyCode>>,
) {
    for mut transform in &mut player_shape_query {
        // translation
        if keycode.pressed(MOVE_LEFT_KEY) {
            transform.translation.x -= MOVE_SPEED;
        }

        if keycode.pressed(MOVE_RIGHT_KEY) {
            transform.translation.x += MOVE_SPEED;
        }

        if keycode.pressed(MOVE_UP_KEY) {
            transform.translation.y += MOVE_SPEED;
        }

        if keycode.pressed(MOVE_DOWN_KEY) {
            transform.translation.y -= MOVE_SPEED;
        }

        // rotation
        if keycode.pressed(ROTATE_CLOCKWISE_KEY) {
            transform.rotate_z(-ROTATE_SPEED);
        }

        if keycode.pressed(ROTATE_COUNTERCLOCKWISE_KEY) {
            transform.rotate_z(ROTATE_SPEED);
        }
    }
}

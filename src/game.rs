use bevy::{ecs::system::EntityCommands, sprite::MaterialMesh2dBundle};
use bevy_asset_loader::prelude::*;
use bevy_rapier2d::prelude::*;
use iyes_progress::{ProgressCounter, ProgressPlugin};

use crate::*;

const MOVE_LEFT_KEY: KeyCode = KeyCode::A;
const MOVE_RIGHT_KEY: KeyCode = KeyCode::D;
const MOVE_UP_KEY: KeyCode = KeyCode::W;
const MOVE_DOWN_KEY: KeyCode = KeyCode::S;

const ROTATE_CLOCKWISE_KEY: KeyCode = KeyCode::Right;
const ROTATE_COUNTERCLOCKWISE_KEY: KeyCode = KeyCode::Left;

const MOVE_SPEED: f32 = 450.0;
const ROTATE_SPEED: f32 = 0.5;

const HIT_SOUND_VOLUME: f32 = 0.5;

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

        app.add_system(move_player.run_if(in_state(GameState::Game)))
            .add_system(collision_sounds.run_if(in_state(GameState::Game)));
    }
}

#[derive(AssetCollection, Resource)]
struct ImageAssets {
    #[asset(path = "images/bouncy_side.png")]
    bouncy_side: Handle<Image>,
    #[asset(path = "images/non_bouncy_side.png")]
    non_bouncy_side: Handle<Image>,
}

#[derive(AssetCollection, Resource)]
struct AudioAssets {
    #[asset(path = "sounds/hit.ogg")]
    hit: Handle<AudioSource>,
    #[asset(path = "sounds/up.ogg")]
    up: Handle<AudioSource>,
    #[asset(path = "sounds/down.ogg")]
    down: Handle<AudioSource>,
}

#[derive(Component)]
struct LoadingComponent;

#[derive(Component)]
struct LoadingText;

#[derive(Component)]
struct GameComponent;

#[derive(Component)]
struct PlayerShape;

#[derive(Component, PartialEq)]
struct SideId(u8);

#[derive(Component, PartialEq)]
enum SideType {
    Regular,
    SpeedUp,
    SlowDown,
}

#[derive(Component)]
struct Ball;

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
    image_assets: Res<ImageAssets>,
) {
    let play_area_radius: f32 = WINDOW_HEIGHT / 2.0;
    let player_shape_radius: f32 = 50.0;
    let side_sprite_custom_size = Vec2::new((player_shape_radius.powi(2) * 2.0).sqrt(), 6.0);
    let side_collider = Collider::segment(
        Vec2::new(-player_shape_radius / 2.0, 0.0),
        Vec2::new(player_shape_radius / 2.0, 0.0),
    );

    commands
        .spawn(MaterialMesh2dBundle {
            mesh: meshes
                .add(shape::RegularPolygon::new(player_shape_radius, 4).into())
                .into(),
            material: materials.add(ColorMaterial::from(Color::Rgba {
                red: 1.0,
                green: 1.0,
                blue: 1.0,
                alpha: 0.01,
            })),
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..default()
        })
        .insert(RigidBody::Dynamic)
        .insert(AdditionalMassProperties::MassProperties(MassProperties {
            mass: 100.0,
            principal_inertia: 16000.0,
            ..default()
        }))
        .insert(ExternalImpulse::default())
        .insert(Damping {
            linear_damping: 4.0,
            angular_damping: 10.0,
        })
        .insert(GravityScale(0.0))
        .insert(GameComponent)
        .insert(PlayerShape)
        .with_children(|parent| {
            // side 0
            spawn_side(parent, SideType::SpeedUp, &image_assets)
                .insert(SideId(0))
                .insert(side_collider.clone())
                .insert(
                    Transform::from_translation(Vec3::new(
                        -player_shape_radius / 2.0,
                        player_shape_radius / 2.0,
                        0.0,
                    ))
                    .with_rotation(Quat::from_rotation_z(45.0_f32.to_radians())),
                )
                .insert(Sprite {
                    custom_size: Some(side_sprite_custom_size),
                    ..default()
                });

            // side 1
            spawn_side(parent, SideType::SlowDown, &image_assets)
                .insert(SideId(1))
                .insert(side_collider.clone())
                .insert(
                    Transform::from_translation(Vec3::new(
                        player_shape_radius / 2.0,
                        player_shape_radius / 2.0,
                        0.0,
                    ))
                    .with_rotation(Quat::from_rotation_z(-45.0_f32.to_radians())),
                )
                .insert(Sprite {
                    custom_size: Some(side_sprite_custom_size),
                    ..default()
                });

            // side 2
            spawn_side(parent, SideType::SpeedUp, &image_assets)
                .insert(SideId(2))
                .insert(side_collider.clone())
                .insert(
                    Transform::from_translation(Vec3::new(
                        player_shape_radius / 2.0,
                        -player_shape_radius / 2.0,
                        0.0,
                    ))
                    .with_rotation(Quat::from_rotation_z(-135.0_f32.to_radians())),
                )
                .insert(Sprite {
                    custom_size: Some(side_sprite_custom_size),
                    ..default()
                });

            // side 3
            spawn_side(parent, SideType::SlowDown, &image_assets)
                .insert(SideId(3))
                .insert(side_collider.clone())
                .insert(
                    Transform::from_translation(Vec3::new(
                        -player_shape_radius / 2.0,
                        -player_shape_radius / 2.0,
                        0.0,
                    ))
                    .with_rotation(Quat::from_rotation_z(135.0_f32.to_radians())),
                )
                .insert(Sprite {
                    custom_size: Some(side_sprite_custom_size),
                    ..default()
                });
        });

    // left wall
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(-play_area_radius * 2.0, 0.0, 0.0)),
            sprite: Sprite {
                color: Color::DARK_GRAY,
                custom_size: Some(Vec2::new(play_area_radius * 2.0, play_area_radius * 2.0)),
                ..default()
            },
            ..default()
        })
        .insert(Collider::cuboid(play_area_radius, play_area_radius))
        .insert(Restitution::coefficient(1.0))
        .insert(GameComponent);

    // top wall
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(0.0, play_area_radius * 2.0, 0.0)),
            sprite: Sprite {
                color: Color::DARK_GRAY,
                custom_size: Some(Vec2::new(play_area_radius * 2.0, play_area_radius * 2.0)),
                ..default()
            },
            ..default()
        })
        .insert(Collider::cuboid(play_area_radius, play_area_radius))
        .insert(Restitution::coefficient(1.0))
        .insert(GameComponent);

    // right wall
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(play_area_radius * 2.0, 0.0, 0.0)),
            sprite: Sprite {
                color: Color::DARK_GRAY,
                custom_size: Some(Vec2::new(play_area_radius * 2.0, play_area_radius * 2.0)),
                ..default()
            },
            ..default()
        })
        .insert(Collider::cuboid(play_area_radius, play_area_radius))
        .insert(Restitution::coefficient(1.0))
        .insert(GameComponent);

    // bottom wall
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(0.0, -play_area_radius * 2.0, 0.0)),
            sprite: Sprite {
                color: Color::DARK_GRAY,
                custom_size: Some(Vec2::new(play_area_radius * 2.0, play_area_radius * 2.0)),
                ..default()
            },
            ..default()
        })
        .insert(Collider::cuboid(play_area_radius, play_area_radius))
        .insert(Restitution::coefficient(1.0))
        .insert(GameComponent);

    // ball
    commands
        .spawn(RigidBody::Dynamic)
        .insert(Collider::ball(15.0))
        .insert(MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(15.0).into()).into(),
            material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
            ..default()
        })
        .insert(Restitution {
            coefficient: 1.0,
            combine_rule: CoefficientCombineRule::Multiply,
        })
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 300.0, 0.0)))
        .insert(ExternalImpulse {
            impulse: Vec2::new(0.0, -10.0),
            ..default()
        })
        .insert(ActiveEvents::COLLISION_EVENTS)
        .insert(GameComponent)
        .insert(Ball);
}

fn spawn_side<'w, 's, 'a>(
    parent: &'a mut ChildBuilder<'w, 's, '_>,
    side_type: SideType,
    image_assets: &Res<ImageAssets>,
) -> EntityCommands<'w, 's, 'a> {
    let mut side = parent.spawn(ActiveEvents::COLLISION_EVENTS);

    match side_type {
        SideType::SpeedUp => side
            .insert(SpriteBundle {
                texture: image_assets.bouncy_side.clone(),
                ..default()
            })
            .insert(Restitution::coefficient(1.5)),
        SideType::SlowDown => side
            .insert(SpriteBundle {
                texture: image_assets.non_bouncy_side.clone(),
                ..default()
            })
            .insert(Restitution::coefficient(0.1)),
        SideType::Regular => side.insert(Restitution::coefficient(1.0)),
    };

    side.insert(side_type);

    side
}

fn move_player(
    mut player_shape_query: Query<&mut ExternalImpulse, With<PlayerShape>>,
    keycode: Res<Input<KeyCode>>,
) {
    for mut impulse in &mut player_shape_query {
        // translation
        if keycode.pressed(MOVE_LEFT_KEY) {
            impulse.impulse.x = -MOVE_SPEED;
        }

        if keycode.pressed(MOVE_RIGHT_KEY) {
            impulse.impulse.x = MOVE_SPEED;
        }

        if keycode.pressed(MOVE_UP_KEY) {
            impulse.impulse.y = MOVE_SPEED;
        }

        if keycode.pressed(MOVE_DOWN_KEY) {
            impulse.impulse.y = -MOVE_SPEED;
        }

        // rotation
        if keycode.pressed(ROTATE_CLOCKWISE_KEY) {
            impulse.torque_impulse = -ROTATE_SPEED;
        }

        if keycode.pressed(ROTATE_COUNTERCLOCKWISE_KEY) {
            impulse.torque_impulse = ROTATE_SPEED;
        }
    }
}

fn collision_sounds(
    mut collision_events: EventReader<CollisionEvent>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
    world: &World,
) {
    for event in collision_events.iter() {
        if let CollisionEvent::Started(a, b, _) = event {
            if one_has_component::<Ball>(*a, *b, world) {
                audio.play_with_settings(
                    audio_assets.hit.clone(),
                    PlaybackSettings::ONCE.with_volume(HIT_SOUND_VOLUME),
                );

                if one_has_matching_component(&SideType::SpeedUp, *a, *b, world) {
                    audio.play(audio_assets.up.clone());
                }

                if one_has_matching_component(&SideType::SlowDown, *a, *b, world) {
                    audio.play(audio_assets.down.clone());
                }
            }
        }
    }
}

// Determines if either of the provided entities have a certain type of component
fn one_has_component<T: Component>(a: Entity, b: Entity, world: &World) -> bool {
    world.get::<T>(a).is_some() || world.get::<T>(b).is_some()
}

// Determines if either of the provided entities has a component with a specific value
fn one_has_matching_component<T: Component + PartialEq>(
    component: &T,
    a: Entity,
    b: Entity,
    world: &World,
) -> bool {
    world.get::<T>(a).map(|c| c == component).unwrap_or(false)
        || world.get::<T>(b).map(|c| c == component).unwrap_or(false)
}

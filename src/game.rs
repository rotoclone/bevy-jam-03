use std::time::{Duration, Instant};

use bevy::{ecs::system::EntityCommands, sprite::MaterialMesh2dBundle};
use bevy_asset_loader::prelude::*;
use bevy_rapier2d::prelude::*;
use iyes_progress::{ProgressCounter, ProgressPlugin};
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::*;

const MOVE_LEFT_KEY: KeyCode = KeyCode::A;
const MOVE_RIGHT_KEY: KeyCode = KeyCode::D;
const MOVE_UP_KEY: KeyCode = KeyCode::W;
const MOVE_DOWN_KEY: KeyCode = KeyCode::S;

const ROTATE_CLOCKWISE_KEY: KeyCode = KeyCode::Right;
const ROTATE_COUNTERCLOCKWISE_KEY: KeyCode = KeyCode::Left;

const MOVE_SPEED: f32 = 1000.0;
const ROTATE_SPEED: f32 = 0.5;

const MASTER_VOLUME: f32 = 0.5;
const HIT_SOUND_VOLUME: f32 = 0.5;
const SPAWN_SOUND_VOLUME: f32 = 0.5;
const GOOD_SCORE_VOLUME: f32 = 0.8;
const BAD_SCORE_VOLUME: f32 = 0.5;

const WALL_COLOR: Color = Color::Rgba {
    red: 0.2,
    green: 0.2,
    blue: 0.2,
    alpha: 1.0,
};

const PLAY_AREA_RADIUS: f32 = WINDOW_HEIGHT / 2.0;

const PLAYER_SHAPE_RADIUS: f32 = 60.0;
const PLAYER_COLLISION_GROUP: Group = Group::GROUP_1;

const BALL_SIZE: f32 = 18.0;
const BALL_MIN_START_X: f32 = -PLAY_AREA_RADIUS / 2.0;
const BALL_MAX_START_X: f32 = PLAY_AREA_RADIUS / 2.0;
const BALL_MIN_START_IMPULSE_Y: f32 = -20.0;
const BALL_MAX_START_IMPULSE_Y: f32 = -5.0;
const BALL_MIN_START_IMPULSE_X: f32 = -10.0;
const BALL_MAX_START_IMPULSE_X: f32 = 10.0;
const BALL_COLLISION_GROUP: Group = Group::GROUP_2;

const TIME_BETWEEN_BALL_GROUP_SPAWNS: Duration = Duration::from_secs(5);
const TIME_BETWEEN_BALL_SPAWNS: Duration = Duration::from_millis(500);
const BALL_GROUP_SIZE: u32 = 4;

const FREEZE_DURATION: Duration = Duration::from_secs(2);

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

        app.insert_resource(Score(0))
            .insert_resource(EntitiesToDespawn(Vec::new()))
            .add_system(spawn_balls.run_if(in_state(GameState::Game)))
            .add_system(player_movement.run_if(in_state(GameState::Game)))
            .add_system(collisions.run_if(in_state(GameState::Game)))
            .add_system(
                update_score_display
                    .after(collisions)
                    .run_if(in_state(GameState::Game)),
            )
            .add_system(
                handle_speed_up_effect
                    .after(collisions)
                    .run_if(in_state(GameState::Game)),
            )
            .add_system(
                handle_slow_down_effect
                    .after(collisions)
                    .run_if(in_state(GameState::Game)),
            )
            .add_system(
                handle_freeze_others_effect
                    .after(collisions)
                    .run_if(in_state(GameState::Game)),
            )
            .add_system(unfreeze_entities.run_if(in_state(GameState::Game)))
            .add_system(despawn_entities.in_base_set(CoreSet::PostUpdate));
    }
}

#[derive(AssetCollection, Resource)]
struct ImageAssets {
    #[asset(path = "images/bouncy_side.png")]
    bouncy_side: Handle<Image>,
    #[asset(path = "images/non_bouncy_side.png")]
    non_bouncy_side: Handle<Image>,
    #[asset(path = "images/directional_side.png")]
    directional_side: Handle<Image>,
    #[asset(path = "images/directional_side.png")] //TODO
    freeze_others_side: Handle<Image>,
}

#[derive(AssetCollection, Resource)]
struct AudioAssets {
    #[asset(path = "sounds/hit.ogg")]
    hit: Handle<AudioSource>,
    #[asset(path = "sounds/up.ogg")]
    up: Handle<AudioSource>,
    #[asset(path = "sounds/down.ogg")]
    down: Handle<AudioSource>,
    #[asset(path = "sounds/launch.ogg")]
    launch: Handle<AudioSource>,
    #[asset(path = "sounds/good_2.ogg")]
    good: Handle<AudioSource>,
    #[asset(path = "sounds/bad.ogg")]
    bad: Handle<AudioSource>,
}

#[derive(Resource)]
struct EntitiesToDespawn(Vec<Entity>);

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

#[derive(Component, PartialEq, Clone, Copy)]
enum SideType {
    Regular,
    SpeedUp,
    SlowDown,
    FreezeOthers,
}

impl SideType {
    /// Adds the effect component that corresponds with this side to the provided entity
    fn add_side_effect(&self, entity: Entity, commands: &mut Commands) {
        match self {
            SideType::Regular => commands.entity(entity).insert(NoEffect),
            SideType::SpeedUp => commands.entity(entity).insert(SpeedUpEffect),
            SideType::SlowDown => commands.entity(entity).insert(SlowDownEffect),
            SideType::FreezeOthers => commands.entity(entity).insert(FreezeOthersEffect),
        };
    }
}

#[derive(Component)]
struct NoEffect;

#[derive(Component)]
struct SpeedUpEffect;

#[derive(Component)]
struct SlowDownEffect;

#[derive(Component)]
struct FreezeOthersEffect;

#[derive(Component)]
struct Frozen {
    unfreeze_at: Instant,
    original_velocity: Velocity,
}

#[derive(Component)]
struct Ball {
    ball_type: BallType,
    points: u16,
}

#[derive(PartialEq)]
enum BallType {
    A,
    B,
    C,
}

impl Distribution<BallType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BallType {
        match rng.gen_range(0..=2) {
            0 => BallType::A,
            1 => BallType::B,
            2 => BallType::C,
            _ => unreachable!(),
        }
    }
}

impl BallType {
    /// Gets the color that corresponds to this ball type
    fn color(&self) -> Color {
        match self {
            BallType::A => Color::ORANGE_RED,
            BallType::B => Color::LIME_GREEN,
            BallType::C => Color::YELLOW,
        }
    }
}

#[derive(Component)]
struct ScoreArea(BallType);

#[derive(Resource)]
struct Score(i32);

#[derive(Component)]
struct ScoreText;

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
    asset_server: Res<AssetServer>,
) {
    let side_sprite_original_width = 100.0;
    let side_sprite_original_height = 10.0;
    let side_sprite_custom_width = (PLAYER_SHAPE_RADIUS.powi(2) * 2.0).sqrt();
    let side_sprite_custom_size = Vec2::new(
        side_sprite_custom_width,
        side_sprite_original_height * (side_sprite_custom_width / side_sprite_original_width),
    );
    let side_collider = Collider::segment(
        Vec2::new(-PLAYER_SHAPE_RADIUS / 2.0, 0.0),
        Vec2::new(PLAYER_SHAPE_RADIUS / 2.0, 0.0),
    );

    commands
        .spawn(MaterialMesh2dBundle {
            mesh: meshes
                .add(shape::RegularPolygon::new(PLAYER_SHAPE_RADIUS, 4).into())
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
                        -PLAYER_SHAPE_RADIUS / 2.0,
                        PLAYER_SHAPE_RADIUS / 2.0,
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
                        PLAYER_SHAPE_RADIUS / 2.0,
                        PLAYER_SHAPE_RADIUS / 2.0,
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
                        PLAYER_SHAPE_RADIUS / 2.0,
                        -PLAYER_SHAPE_RADIUS / 2.0,
                        0.0,
                    ))
                    .with_rotation(Quat::from_rotation_z(-135.0_f32.to_radians())),
                )
                .insert(Sprite {
                    custom_size: Some(side_sprite_custom_size),
                    ..default()
                });

            // side 3
            spawn_side(parent, SideType::FreezeOthers, &image_assets)
                .insert(SideId(3))
                .insert(side_collider.clone())
                .insert(
                    Transform::from_translation(Vec3::new(
                        -PLAYER_SHAPE_RADIUS / 2.0,
                        -PLAYER_SHAPE_RADIUS / 2.0,
                        0.0,
                    ))
                    .with_rotation(Quat::from_rotation_z(135.0_f32.to_radians())),
                )
                .insert(Sprite {
                    custom_size: Some(side_sprite_custom_size),
                    ..default()
                });
        });

    // score areas
    /* TODO
    let mut score_area_a_color = BallType::A.color();
    score_area_a_color.set_a(0.1);
    commands
        .spawn(MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(100.0).into()).into(),
            material: materials.add(ColorMaterial::from(score_area_a_color)),
            ..default()
        })
        .insert(Collider::ball(100.0))
        .insert(Sensor)
        .insert(Transform::from_translation(Vec3::new(
            -PLAY_AREA_RADIUS,
            PLAY_AREA_RADIUS,
            0.0,
        )))
        .insert(GameComponent)
        .insert(ScoreArea(BallType::A));

    let mut score_area_b_color = BallType::B.color();
    score_area_b_color.set_a(0.1);
    commands
        .spawn(MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(100.0).into()).into(),
            material: materials.add(ColorMaterial::from(score_area_b_color)),
            ..default()
        })
        .insert(Collider::ball(100.0))
        .insert(Sensor)
        .insert(Transform::from_translation(Vec3::new(
            PLAY_AREA_RADIUS,
            PLAY_AREA_RADIUS,
            0.0,
        )))
        .insert(GameComponent)
        .insert(ScoreArea(BallType::B));

    let mut score_area_c_color = BallType::C.color();
    score_area_c_color.set_a(0.1);
    commands
        .spawn(MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(100.0).into()).into(),
            material: materials.add(ColorMaterial::from(score_area_c_color)),
            ..default()
        })
        .insert(Collider::ball(100.0))
        .insert(Sensor)
        .insert(Transform::from_translation(Vec3::new(
            PLAY_AREA_RADIUS,
            -PLAY_AREA_RADIUS,
            0.0,
        )))
        .insert(GameComponent)
        .insert(ScoreArea(BallType::C));
    */

    // player boundary
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(0.0, PLAY_AREA_RADIUS / 3.0, 0.0)),
            sprite: Sprite {
                color: Color::Rgba {
                    red: 0.2,
                    green: 0.2,
                    blue: 0.2,
                    alpha: 0.2,
                },
                custom_size: Some(Vec2::new(PLAY_AREA_RADIUS * 2.0, 4.0)),
                ..default()
            },
            ..default()
        })
        .insert(Collider::cuboid(PLAY_AREA_RADIUS, 2.0))
        // only collide with player
        .insert(CollisionGroups::new(Group::GROUP_3, PLAYER_COLLISION_GROUP))
        .insert(Restitution::coefficient(1.0))
        .insert(GameComponent);

    // left wall
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(-PLAY_AREA_RADIUS * 2.0, 0.0, 0.0)),
            sprite: Sprite {
                color: WALL_COLOR,
                custom_size: Some(Vec2::new(PLAY_AREA_RADIUS * 2.0, PLAY_AREA_RADIUS * 2.0)),
                ..default()
            },
            ..default()
        })
        .insert(Collider::cuboid(PLAY_AREA_RADIUS, PLAY_AREA_RADIUS))
        .insert(Restitution::coefficient(1.0))
        .insert(GameComponent);

    // top wall
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(0.0, PLAY_AREA_RADIUS * 2.0, 0.0)),
            sprite: Sprite {
                color: WALL_COLOR,
                custom_size: Some(Vec2::new(PLAY_AREA_RADIUS * 2.0, PLAY_AREA_RADIUS * 2.0)),
                ..default()
            },
            ..default()
        })
        .insert(Collider::cuboid(PLAY_AREA_RADIUS, PLAY_AREA_RADIUS))
        .insert(Restitution::coefficient(1.0))
        .insert(ScoreArea(BallType::A)) //TODO
        .insert(GameComponent);

    // right wall
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(PLAY_AREA_RADIUS * 2.0, 0.0, 0.0)),
            sprite: Sprite {
                color: WALL_COLOR,
                custom_size: Some(Vec2::new(PLAY_AREA_RADIUS * 2.0, PLAY_AREA_RADIUS * 2.0)),
                ..default()
            },
            ..default()
        })
        .insert(Collider::cuboid(PLAY_AREA_RADIUS, PLAY_AREA_RADIUS))
        .insert(Restitution::coefficient(1.0))
        .insert(GameComponent);

    // bottom wall
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(0.0, -PLAY_AREA_RADIUS * 2.0, 0.0)),
            sprite: Sprite {
                color: WALL_COLOR,
                custom_size: Some(Vec2::new(PLAY_AREA_RADIUS * 2.0, PLAY_AREA_RADIUS * 2.0)),
                ..default()
            },
            ..default()
        })
        .insert(Collider::cuboid(PLAY_AREA_RADIUS, PLAY_AREA_RADIUS))
        .insert(Restitution::coefficient(1.0))
        .insert(ScoreArea(BallType::B)) //TODO
        .insert(GameComponent);

    // score display
    commands
        .spawn(
            TextBundle::from_section(
                "Score: 0",
                TextStyle {
                    font: asset_server.load(MAIN_FONT),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            )
            .with_text_alignment(TextAlignment::Center)
            .with_style(Style {
                margin: UiRect {
                    left: Val::Px(10.0),
                    top: Val::Px(10.0),
                    ..default()
                },
                ..default()
            }),
        )
        .insert(GameComponent)
        .insert(ScoreText);
}

/// Spawns a side for the player shape
fn spawn_side<'w, 's, 'a>(
    parent: &'a mut ChildBuilder<'w, 's, '_>,
    side_type: SideType,
    image_assets: &Res<ImageAssets>,
) -> EntityCommands<'w, 's, 'a> {
    let mut side = parent.spawn(ActiveEvents::COLLISION_EVENTS);

    match side_type {
        SideType::Regular => side.insert(Restitution::coefficient(1.0)),
        SideType::SpeedUp => side
            .insert(SpriteBundle {
                texture: image_assets.bouncy_side.clone(),
                ..default()
            })
            .insert(Restitution::coefficient(2.0)),
        SideType::SlowDown => side
            .insert(SpriteBundle {
                texture: image_assets.non_bouncy_side.clone(),
                ..default()
            })
            .insert(Restitution::coefficient(0.2)),
        SideType::FreezeOthers => side
            .insert(SpriteBundle {
                texture: image_assets.freeze_others_side.clone(),
                ..default()
            })
            .insert(Restitution::coefficient(0.1)),
    };

    side.insert(CollisionGroups {
        memberships: PLAYER_COLLISION_GROUP,
        filters: Group::all(),
    })
    .insert(side_type);

    side
}

struct SpawnTime(Instant);

impl Default for SpawnTime {
    fn default() -> Self {
        Self(Instant::now())
    }
}

/// Spawns balls
fn spawn_balls(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    mut next_spawn_time: Local<SpawnTime>,
    mut balls_spawned_in_group: Local<u32>,
    audio_assets: Res<AudioAssets>,
    audio: Res<Audio>,
) {
    if Instant::now().duration_since(next_spawn_time.0) > Duration::ZERO {
        spawn_random_ball(commands, meshes, materials);

        audio.play_with_settings(
            audio_assets.launch.clone(),
            PlaybackSettings::ONCE.with_volume(SPAWN_SOUND_VOLUME * MASTER_VOLUME),
        );

        *balls_spawned_in_group += 1;

        if *balls_spawned_in_group >= BALL_GROUP_SIZE {
            *balls_spawned_in_group = 0;
            next_spawn_time.0 = Instant::now() + TIME_BETWEEN_BALL_GROUP_SPAWNS;
        } else {
            next_spawn_time.0 = Instant::now() + TIME_BETWEEN_BALL_SPAWNS;
        }
    }
}

/// Spawns a random ball at a random point with a random initial impulse
fn spawn_random_ball(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut rng = rand::thread_rng();
    //TODO let ball_type = rng.gen::<BallType>();
    let ball_type = BallType::A;
    let spawn_point_x = rng.gen_range(BALL_MIN_START_X..=BALL_MAX_START_X);
    let impulse_x = rng.gen_range(BALL_MIN_START_IMPULSE_X..=BALL_MAX_START_IMPULSE_X);
    let impulse_y = rng.gen_range(BALL_MIN_START_IMPULSE_Y..=BALL_MAX_START_IMPULSE_Y);
    spawn_ball(
        &mut commands,
        Ball {
            ball_type,
            points: 1,
        },
        &mut meshes,
        &mut materials,
    )
    .insert(TransformBundle::from(Transform::from_xyz(
        spawn_point_x,
        PLAY_AREA_RADIUS - BALL_SIZE - 1.0,
        0.0,
    )))
    .insert(ExternalImpulse {
        impulse: Vec2::new(impulse_x, impulse_y),
        ..default()
    });
}

/// Spawns a ball
fn spawn_ball<'w, 's, 'a>(
    commands: &'a mut Commands<'w, 's>,
    ball_component: Ball,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
) -> EntityCommands<'w, 's, 'a> {
    let mut ball = commands.spawn(RigidBody::Dynamic);

    ball.insert(Collider::ball(BALL_SIZE))
        // make balls go through each other
        .insert(CollisionGroups::new(
            BALL_COLLISION_GROUP,
            Group::all().difference(BALL_COLLISION_GROUP),
        ))
        .insert(MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(BALL_SIZE).into()).into(),
            material: materials.add(ColorMaterial::from(ball_component.ball_type.color())),
            ..default()
        })
        .insert(Restitution {
            coefficient: 1.0,
            combine_rule: CoefficientCombineRule::Multiply,
        })
        .insert(Velocity::zero())
        .insert(ActiveEvents::COLLISION_EVENTS)
        .insert(Sleeping::disabled())
        .insert(GameComponent)
        .insert(ball_component);

    ball
}

/// Applies impulses to the player based on pressed keys
fn player_movement(
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

/// Handles collisions between objects
fn collisions(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut score: ResMut<Score>,
    mut entities_to_despawn: ResMut<EntitiesToDespawn>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
    balls_query: Query<&Ball>,
    score_areas_query: Query<&ScoreArea>,
    sides_query: Query<&SideType>,
) {
    for event in collision_events.iter() {
        if let CollisionEvent::Started(a, b, _) = event {
            if let Some((ball, ball_entity)) = get_either(*a, *b, &balls_query) {
                // a ball has hit something
                if entities_to_despawn.0.contains(&ball_entity) {
                    // this ball is going to be despawned, so don't mess with it any more
                    continue;
                }
                unfreeze_entity(ball_entity, &mut commands);
                if let Some((score_area, _)) = get_either(*a, *b, &score_areas_query) {
                    // a ball has hit a score area
                    if ball.ball_type == score_area.0 {
                        score.0 += i32::from(ball.points);
                        audio.play_with_settings(
                            audio_assets.good.clone(),
                            PlaybackSettings::ONCE.with_volume(GOOD_SCORE_VOLUME * MASTER_VOLUME),
                        );
                    } else {
                        score.0 -= i32::from(ball.points);
                        audio.play_with_settings(
                            audio_assets.bad.clone(),
                            PlaybackSettings::ONCE.with_volume(BAD_SCORE_VOLUME * MASTER_VOLUME),
                        );
                    }
                    entities_to_despawn.0.push(ball_entity);
                } else {
                    // a ball has hit something that's not a score area
                    audio.play_with_settings(
                        audio_assets.hit.clone(),
                        PlaybackSettings::ONCE.with_volume(HIT_SOUND_VOLUME * MASTER_VOLUME),
                    );

                    if let Some((side_type, _)) = get_either(*a, *b, &sides_query) {
                        // a ball has hit a side
                        side_type.add_side_effect(ball_entity, &mut commands);
                    }
                }
            }
        }
    }
}

fn get_either<'a, T: Component>(
    a: Entity,
    b: Entity,
    query: &'a Query<&T>,
) -> Option<(&'a T, Entity)> {
    if let Ok(component) = query.get(a) {
        return Some((component, a));
    }

    if let Ok(component) = query.get(b) {
        return Some((component, b));
    }

    None
}

/// Deals with entities that have had the speed up effect added
fn handle_speed_up_effect(
    mut commands: Commands,
    query: Query<Entity, Added<SpeedUpEffect>>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
) {
    for entity in query.iter() {
        audio.play(audio_assets.up.clone());
        commands.entity(entity).remove::<SpeedUpEffect>();
    }
}

/// Deals with entities that have had the slow down effect added
fn handle_slow_down_effect(
    mut commands: Commands,
    query: Query<Entity, Added<SlowDownEffect>>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
) {
    for entity in query.iter() {
        audio.play(audio_assets.down.clone());
        commands.entity(entity).remove::<SlowDownEffect>();
    }
}

/// Deals with entities that have had the freeze others effect added
fn handle_freeze_others_effect(
    mut commands: Commands,
    query: Query<Entity, Added<FreezeOthersEffect>>,
    mut frozen_query: Query<&mut Frozen>,
    balls_query: Query<(Entity, &Velocity), With<Ball>>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
) {
    for entity in query.iter() {
        for (ball_entity, velocity) in balls_query.iter() {
            if ball_entity != entity {
                if let Ok(mut frozen) = frozen_query.get_mut(ball_entity) {
                    // the ball is already frozen, so just update its unfreeze time
                    frozen.unfreeze_at = Instant::now() + FREEZE_DURATION;
                } else {
                    // the ball is not currently frozen, so freeze it
                    commands
                        .entity(ball_entity)
                        .insert(Frozen {
                            unfreeze_at: Instant::now() + FREEZE_DURATION,
                            original_velocity: *velocity,
                        })
                        .insert(RigidBody::Fixed);
                }
            }
        }
        //TODO sound effect
        commands.entity(entity).remove::<FreezeOthersEffect>();
    }
}

/// Handles unfreezing entities
fn unfreeze_entities(
    mut commands: Commands,
    frozen_query: Query<(Entity, &Frozen), With<RigidBody>>,
) {
    for (entity, frozen) in frozen_query.iter() {
        if Instant::now().duration_since(frozen.unfreeze_at) > Duration::ZERO {
            unfreeze_entity(entity, &mut commands);
            commands.entity(entity).insert(frozen.original_velocity);
        }
    }
}

/// Unfreezes the provided entity
fn unfreeze_entity(entity: Entity, commands: &mut Commands) {
    commands
        .entity(entity)
        .insert(RigidBody::Dynamic)
        .insert(Sleeping {
            sleeping: false,
            ..default()
        })
        .remove::<Frozen>();
}

/// Keeps the score display up to date
fn update_score_display(
    score: Res<Score>,
    mut score_text_query: Query<&mut Text, With<ScoreText>>,
) {
    for mut score_text in score_text_query.iter_mut() {
        score_text.sections[0].value = format!("Score: {}", score.0);
    }
}

/// Despawns entities that need to be despawned
fn despawn_entities(mut commands: Commands, mut entities_to_despawn: ResMut<EntitiesToDespawn>) {
    for entity in entities_to_despawn.0.drain(0..) {
        commands.entity(entity).despawn_recursive();
    }
}

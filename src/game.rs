use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    time::{Duration, Instant},
};

use bevy::{
    ecs::{query::ReadOnlyWorldQuery, system::EntityCommands},
    input::mouse::MouseWheel,
    sprite::MaterialMesh2dBundle,
};
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

const INCREASE_ROTATE_SENSITIVITY_KEY: KeyCode = KeyCode::Period;
const DECREASE_ROTATE_SENSITIVITY_KEY: KeyCode = KeyCode::Comma;

const ROTATE_SENSITIVITY_ADJUST_AMOUNT: f32 = 0.2;

const MOVE_SPEED: f32 = 150000.0;
const ROTATE_SPEED: f32 = 65.0;
const SCROLL_ROTATE_SPEED: f32 = 3.0;

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

const SCORE_AREA_SIZE: f32 = 150.0;

pub const PLAYER_SHAPE_SIDES: usize = 4;
const PLAYER_SHAPE_RADIUS: f32 = 60.0;
const PLAYER_COLLISION_GROUP: Group = Group::GROUP_1;

const BALL_SIZE: f32 = 18.0;
const BALL_MIN_START_X: f32 = -PLAY_AREA_RADIUS / 2.0;
const BALL_MAX_START_X: f32 = PLAY_AREA_RADIUS / 2.0;
const BALL_COLLISION_GROUP: Group = Group::GROUP_2;

const FREEZE_DURATION: Duration = Duration::from_secs(2);
const BOUNCE_BACKWARDS_VELOCITY: f32 = 100.0;
const BOUNCE_BACKWARDS_DISTANCE: f32 = BALL_SIZE + 1.0;

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

        app.insert_resource(UnlockedSides(
            [SideType::NothingSpecial, SideType::SpeedUp].into(),
        ))
        .insert_resource(ConfiguredSides(
            [
                (SideId(0), SideType::SpeedUp),
                (SideId(1), SideType::NothingSpecial),
                (SideId(2), SideType::NothingSpecial),
                (SideId(3), SideType::NothingSpecial),
            ]
            .into(),
        ))
        .insert_resource(LevelSettings::first_level())
        .insert_resource(EntitiesToDespawn(Vec::new()))
        .insert_resource(RotateSensitivity(1.0))
        .add_system(update_time_display.run_if(in_state(GameState::Game)))
        .add_system(spawn_balls.run_if(in_state(GameState::Game)))
        .add_system(
            adjust_rotate_sensitivity
                .before(player_movement)
                .run_if(in_state(GameState::Game)),
        )
        .add_system(
            update_rotate_sensitivity_display
                .after(adjust_rotate_sensitivity)
                .run_if(in_state(GameState::Game)),
        )
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
            handle_freeze_others_effect
                .after(collisions)
                .run_if(in_state(GameState::Game)),
        )
        .add_system(
            handle_bounce_backwards_effect
                .after(collisions)
                .run_if(in_state(GameState::Game)),
        )
        .add_system(
            handle_destroy_effect
                .after(collisions)
                .run_if(in_state(GameState::Game)),
        )
        .add_system(
            handle_duplicate_effect
                .after(collisions)
                .run_if(in_state(GameState::Game)),
        )
        .add_system(
            handle_resize_score_areas_effect
                .after(collisions)
                .run_if(in_state(GameState::Game)),
        )
        .add_system(unfreeze_entities.run_if(in_state(GameState::Game)))
        .add_system(
            end_level
                .after(collisions)
                .run_if(in_state(GameState::Game)),
        )
        .add_system(despawn_entities.in_base_set(CoreSet::PostUpdate));
    }
}

#[derive(AssetCollection, Resource)]
pub struct ImageAssets {
    #[asset(path = "images/regular_side.png")]
    regular_side: Handle<Image>,
    #[asset(path = "images/bouncy_side.png")]
    bouncy_side: Handle<Image>,
    #[asset(path = "images/freeze_side.png")]
    freeze_others_side: Handle<Image>,
    #[asset(path = "images/bounce_backwards_side.png")]
    bounce_backwards_side: Handle<Image>,
    #[asset(path = "images/destroy_side.png")]
    destroy_side: Handle<Image>,
    #[asset(path = "images/duplicate_side.png")]
    duplicate_side: Handle<Image>,
    #[asset(path = "images/resize_side.png")]
    resize_side: Handle<Image>,
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

#[derive(Resource)]
struct RotateSensitivity(f32);

#[derive(Resource)]
pub struct LevelSettings {
    /// The ID of the level
    pub id: usize,
    /// Amount of time between spawning groups of balls
    time_between_groups: Duration,
    /// Amount of time between spawning balls in the same group
    time_between_spawns_in_group: Duration,
    /// Number of balls spawned per group
    balls_per_group: u32,
    /// The range of possible initial impulses in the X direction on spawned balls
    start_impulse_range_x: Range<f32>,
    /// The range of possible initial impulses in the Y direction on spawned balls
    start_impulse_range_y: Range<f32>,
    /// The time limit for the level
    duration: Duration,
    /// The minimum score required to complete the level
    pub min_score: i32,
    /// The sides that will be unlocked when the level is completed
    pub sides_to_unlock: Vec<SideType>,
}

impl LevelSettings {
    /// Builds settings for the first level
    fn first_level() -> LevelSettings {
        LevelSettings {
            id: 1,
            time_between_groups: Duration::from_secs(9),
            time_between_spawns_in_group: Duration::from_millis(500),
            balls_per_group: 3,
            start_impulse_range_x: -10.0..10.0,
            start_impulse_range_y: -20.0..-5.0,
            duration: Duration::from_secs(30),
            sides_to_unlock: vec![SideType::FreezeOthers],
            min_score: 1,
        }
    }

    /// Builds settings for the level after this one
    pub fn next_level(&self) -> LevelSettings {
        match self.id {
            1 => LevelSettings {
                id: 2,
                time_between_groups: Duration::from_secs(8),
                time_between_spawns_in_group: Duration::from_millis(500),
                balls_per_group: 3,
                start_impulse_range_x: -10.0..10.0,
                start_impulse_range_y: -21.0..-5.0,
                duration: Duration::from_secs(40),
                sides_to_unlock: vec![SideType::ResizeScoreAreas],
                min_score: 1,
            },
            2 => LevelSettings {
                id: 3,
                time_between_groups: Duration::from_secs(8),
                time_between_spawns_in_group: Duration::from_millis(500),
                balls_per_group: 4,
                start_impulse_range_x: -10.0..10.0,
                start_impulse_range_y: -23.0..-5.0,
                duration: Duration::from_secs(50),
                sides_to_unlock: vec![SideType::BounceBackwards],
                min_score: 1,
            },
            3 => LevelSettings {
                id: 4,
                time_between_groups: Duration::from_secs(7),
                time_between_spawns_in_group: Duration::from_millis(500),
                balls_per_group: 4,
                start_impulse_range_x: -10.0..10.0,
                start_impulse_range_y: -25.0..-6.0,
                duration: Duration::from_secs(60),
                sides_to_unlock: vec![SideType::Destroy],
                min_score: 3,
            },
            4 => LevelSettings {
                id: 5,
                time_between_groups: Duration::from_secs(7),
                time_between_spawns_in_group: Duration::from_millis(500),
                balls_per_group: 5,
                start_impulse_range_x: -10.0..10.0,
                start_impulse_range_y: -27.0..-7.0,
                duration: Duration::from_secs(60),
                sides_to_unlock: vec![SideType::Duplicate],
                min_score: 5,
            },
            _ => LevelSettings {
                id: self.id + 1,
                time_between_groups: self.time_between_groups,
                time_between_spawns_in_group: self.time_between_spawns_in_group,
                balls_per_group: self.balls_per_group + 1,
                start_impulse_range_x: self.start_impulse_range_x.clone(),
                start_impulse_range_y: (self.start_impulse_range_y.start - 2.0)
                    ..self.start_impulse_range_y.end,
                duration: self.duration,
                min_score: self.min_score + 3,
                sides_to_unlock: vec![],
            },
        }
    }
}

#[derive(Resource)]
pub struct UnlockedSides(pub Vec<SideType>);

#[derive(Resource)]
pub struct ConfiguredSides(pub HashMap<SideId, SideType>);

impl ConfiguredSides {
    /// Gets the type of the side with the provided ID. Panics if the side is not configured.
    pub fn get(&self, side_id: &SideId) -> SideType {
        *self
            .0
            .get(side_id)
            .unwrap_or_else(|| panic!("side {side_id:?} should be configured"))
    }
}

#[derive(Component)]
struct LoadingComponent;

#[derive(Component)]
struct LoadingText;

#[derive(Component)]
struct GameComponent;

#[derive(Component)]
struct PlayerShape;

#[derive(Component, Eq, PartialEq, Hash, Clone, Copy, Debug)]
pub struct SideId(pub usize);

impl SideId {
    /// Finds the ID of the opposite side
    fn opposite_side(&self) -> SideId {
        SideId((self.0 + (PLAYER_SHAPE_SIDES / 2)) % PLAYER_SHAPE_SIDES)
    }
}

#[derive(Component, PartialEq, Eq, Hash, Clone, Copy)]
pub enum SideType {
    NothingSpecial,
    SpeedUp,
    FreezeOthers,
    BounceBackwards,
    Destroy,
    Duplicate,
    ResizeScoreAreas,
}

impl SideType {
    /// Adds the effect component that corresponds with this side to the provided entity
    fn add_side_effect(&self, entity: Entity, side_id: SideId, commands: &mut Commands) {
        match self {
            SideType::NothingSpecial => (),
            SideType::SpeedUp => {
                commands.entity(entity).insert(SpeedUpEffect);
            }
            SideType::FreezeOthers => {
                commands.entity(entity).insert(FreezeOthersEffect);
            }
            SideType::BounceBackwards => {
                commands
                    .entity(entity)
                    .insert(BounceBackwardsEffect { side_hit: side_id });
            }
            SideType::Destroy => {
                commands.entity(entity).insert(DestroyEffect);
            }
            SideType::Duplicate => {
                commands.entity(entity).insert(DuplicateEffect);
            }
            SideType::ResizeScoreAreas => {
                commands.entity(entity).insert(ResizeScoreAreasEffect);
            }
        };
    }

    /// Gets the name of this side
    pub fn name(&self) -> &str {
        match self {
            SideType::NothingSpecial => "Regular",
            SideType::SpeedUp => "Bouncy",
            SideType::FreezeOthers => "Freeze",
            SideType::BounceBackwards => "Bounce Backwards",
            SideType::Destroy => "Destroy",
            SideType::Duplicate => "Duplicate",
            SideType::ResizeScoreAreas => "Resize",
        }
    }

    /// Gets the description of this side
    pub fn description(&self) -> &str {
        match self {
            SideType::NothingSpecial => "Balls bounce off of it",
            SideType::SpeedUp => "Bounces balls real fast",
            SideType::FreezeOthers => {
                "Temporarily freezes all balls other than the one that hit it"
            }
            SideType::BounceBackwards => "Bounces balls backwards out the other side of you",
            SideType::Destroy => "Destroys balls that hit it",
            SideType::Duplicate => "Duplicates balls that hit it",
            SideType::ResizeScoreAreas => "Temporarily increases the size of the score area matching the ball that hit it, and decreases the size of other score areas",
        }
    }

    /// Determines whether this side type can appear multiple times on the player
    pub fn multiple_allowed(&self) -> bool {
        matches!(self, SideType::NothingSpecial)
    }
}

#[derive(Component)]
struct SpeedUpEffect;

#[derive(Component)]
struct FreezeOthersEffect;

#[derive(Component)]
struct BounceBackwardsEffect {
    side_hit: SideId,
}

#[derive(Component)]
struct DestroyEffect;

#[derive(Component)]
struct DuplicateEffect;

#[derive(Component)]
struct ResizeScoreAreasEffect;

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
pub struct Score(pub i32);

#[derive(Resource)]
struct LevelEndTime(Instant);

#[derive(Component)]
struct LevelText;

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct TimeText;

#[derive(Component)]
struct RotateSensitivityText;

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
    rotate_sensitivity: Res<RotateSensitivity>,
    level_settings: Res<LevelSettings>,
    configured_sides: Res<ConfiguredSides>,
) {
    spawn_player_shape(
        &mut commands,
        &mut meshes,
        &mut materials,
        &image_assets,
        &configured_sides,
        Transform::from_translation(Vec3::new(0., 0., 0.)),
    )
    .insert(GameComponent);

    // score areas
    let mut score_area_a_color = BallType::A.color();
    score_area_a_color.set_a(0.1);
    commands
        .spawn(MaterialMesh2dBundle {
            mesh: meshes
                .add(shape::Circle::new(SCORE_AREA_SIZE).into())
                .into(),
            material: materials.add(ColorMaterial::from(score_area_a_color)),
            ..default()
        })
        .insert(Collider::ball(SCORE_AREA_SIZE))
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
            mesh: meshes
                .add(shape::Circle::new(SCORE_AREA_SIZE).into())
                .into(),
            material: materials.add(ColorMaterial::from(score_area_b_color)),
            ..default()
        })
        .insert(Collider::ball(SCORE_AREA_SIZE))
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
            mesh: meshes
                .add(shape::Circle::new(SCORE_AREA_SIZE).into())
                .into(),
            material: materials.add(ColorMaterial::from(score_area_c_color)),
            ..default()
        })
        .insert(Collider::ball(SCORE_AREA_SIZE))
        .insert(Sensor)
        .insert(Transform::from_translation(Vec3::new(
            PLAY_AREA_RADIUS,
            -PLAY_AREA_RADIUS,
            0.0,
        )))
        .insert(GameComponent)
        .insert(ScoreArea(BallType::C));

    /* TODO
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
    */

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
        .insert(GameComponent);

    // left sidebar
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(33.3), Val::Percent(100.0)),
                position_type: PositionType::Absolute,
                position: UiRect {
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                margin: UiRect {
                    left: Val::Px(5.0),
                    top: Val::Px(5.0),
                    ..default()
                },
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Start,
                align_items: AlignItems::FlexStart,
                ..default()
            },
            ..default()
        })
        .insert(GameComponent)
        .with_children(|parent| {
            // level display
            parent
                .spawn(
                    TextBundle::from_section(
                        format!("Level {}", level_settings.id),
                        TextStyle {
                            font: asset_server.load(MAIN_FONT),
                            font_size: 30.0,
                            color: Color::Rgba {
                                red: 0.75,
                                green: 0.75,
                                blue: 0.75,
                                alpha: 1.0,
                            },
                        },
                    )
                    .with_text_alignment(TextAlignment::Center),
                )
                .insert(LevelText);

            // minimum score display
            parent.spawn(
                TextBundle::from_section(
                    format!("Score needed: {}", level_settings.min_score),
                    TextStyle {
                        font: asset_server.load(MAIN_FONT),
                        font_size: 25.0,
                        color: Color::Rgba {
                            red: 0.75,
                            green: 0.75,
                            blue: 0.75,
                            alpha: 1.0,
                        },
                    },
                )
                .with_text_alignment(TextAlignment::Center),
            );

            // score display
            parent
                .spawn(
                    TextBundle::from_section(
                        "Score: 0",
                        TextStyle {
                            font: asset_server.load(MAIN_FONT),
                            font_size: 35.0,
                            color: Color::WHITE,
                        },
                    )
                    .with_text_alignment(TextAlignment::Center),
                )
                .insert(ScoreText);
        });

    // timer display
    commands
        .spawn(
            TextBundle::from_section(
                "",
                TextStyle {
                    font: asset_server.load(MAIN_FONT),
                    font_size: 40.0,
                    color: Color::WHITE,
                },
            )
            .with_text_alignment(TextAlignment::Center)
            .with_style(Style {
                position_type: PositionType::Absolute,
                position: UiRect {
                    top: Val::Px(10.0),
                    ..default()
                },
                margin: UiRect {
                    left: Val::Auto,
                    right: Val::Auto,
                    ..default()
                },
                ..default()
            }),
        )
        .insert(GameComponent)
        .insert(TimeText);

    // rotation sensitivity display
    commands
        .spawn(
            TextBundle::from_section(
                format!("Rotation sensitivity: {:.1}", rotate_sensitivity.0),
                TextStyle {
                    font: asset_server.load(MAIN_FONT),
                    font_size: 20.0,
                    color: Color::GRAY,
                },
            )
            .with_text_alignment(TextAlignment::Center)
            .with_style(Style {
                position_type: PositionType::Absolute,
                position: UiRect {
                    right: Val::Px(5.0),
                    bottom: Val::Px(5.0),
                    ..default()
                },
                ..default()
            }),
        )
        .insert(GameComponent)
        .insert(RotateSensitivityText);

    commands.insert_resource(Score(0));
    commands.insert_resource(LevelEndTime(Instant::now() + level_settings.duration));
}

/// Spawns the player at the provided location
pub fn spawn_player_shape<'w, 's, 'a>(
    commands: &'a mut Commands<'w, 's>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    image_assets: &ImageAssets,
    configured_sides: &ConfiguredSides,
    transform: Transform,
) -> EntityCommands<'w, 's, 'a> {
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

    let mut player_shape = commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(shape::RegularPolygon::new(PLAYER_SHAPE_RADIUS, PLAYER_SHAPE_SIDES).into())
            .into(),
        material: materials.add(ColorMaterial::from(Color::Rgba {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            alpha: 0.01,
        })),
        transform,
        ..default()
    });

    player_shape
        .insert(RigidBody::Dynamic)
        .insert(AdditionalMassProperties::MassProperties(MassProperties {
            mass: 100.0,
            principal_inertia: 16000.0,
            ..default()
        }))
        .insert(ExternalForce::default())
        .insert(ExternalImpulse::default())
        .insert(Damping {
            linear_damping: 7.0,
            angular_damping: 10.0,
        })
        .insert(GravityScale(0.0))
        .insert(PlayerShape)
        .with_children(|parent| {
            // side 0
            let side_0_type = configured_sides.get(&SideId(0));
            spawn_side(parent, side_0_type, side_sprite_custom_size, image_assets)
                .insert(SideId(0))
                .insert(side_collider.clone())
                .insert(
                    Transform::from_translation(Vec3::new(
                        -PLAYER_SHAPE_RADIUS / 2.0,
                        PLAYER_SHAPE_RADIUS / 2.0,
                        0.0,
                    ))
                    .with_rotation(Quat::from_rotation_z(45.0_f32.to_radians())),
                );

            // side 1
            let side_1_type = configured_sides.get(&SideId(1));
            spawn_side(parent, side_1_type, side_sprite_custom_size, image_assets)
                .insert(SideId(1))
                .insert(side_collider.clone())
                .insert(
                    Transform::from_translation(Vec3::new(
                        PLAYER_SHAPE_RADIUS / 2.0,
                        PLAYER_SHAPE_RADIUS / 2.0,
                        0.0,
                    ))
                    .with_rotation(Quat::from_rotation_z(-45.0_f32.to_radians())),
                );

            // side 2
            let side_2_type = configured_sides.get(&SideId(2));
            spawn_side(parent, side_2_type, side_sprite_custom_size, image_assets)
                .insert(SideId(2))
                .insert(side_collider.clone())
                .insert(
                    Transform::from_translation(Vec3::new(
                        PLAYER_SHAPE_RADIUS / 2.0,
                        -PLAYER_SHAPE_RADIUS / 2.0,
                        0.0,
                    ))
                    .with_rotation(Quat::from_rotation_z(-135.0_f32.to_radians())),
                );

            // side 3
            let side_3_type = configured_sides.get(&SideId(3));
            spawn_side(parent, side_3_type, side_sprite_custom_size, image_assets)
                .insert(SideId(3))
                .insert(side_collider.clone())
                .insert(
                    Transform::from_translation(Vec3::new(
                        -PLAYER_SHAPE_RADIUS / 2.0,
                        -PLAYER_SHAPE_RADIUS / 2.0,
                        0.0,
                    ))
                    .with_rotation(Quat::from_rotation_z(135.0_f32.to_radians())),
                );
        });

    player_shape
}

/// Spawns a side for the player shape
fn spawn_side<'w, 's, 'a>(
    parent: &'a mut ChildBuilder<'w, 's, '_>,
    side_type: SideType,
    sprite_custom_size: Vec2,
    image_assets: &ImageAssets,
) -> EntityCommands<'w, 's, 'a> {
    let mut side = parent.spawn(ActiveEvents::COLLISION_EVENTS);

    match side_type {
        SideType::NothingSpecial => side
            .insert(SpriteBundle {
                texture: image_assets.regular_side.clone(),
                sprite: Sprite {
                    custom_size: Some(sprite_custom_size),
                    color: Color::rgb(0.8, 0.8, 0.8),
                    ..default()
                },
                ..default()
            })
            .insert(Restitution::coefficient(0.33)),
        SideType::SpeedUp => side
            .insert(SpriteBundle {
                texture: image_assets.bouncy_side.clone(),
                sprite: Sprite {
                    custom_size: Some(sprite_custom_size),
                    color: Color::rgb(0.8, 1.0, 0.8),
                    ..default()
                },
                ..default()
            })
            .insert(Restitution::coefficient(2.0)),
        SideType::FreezeOthers => side
            .insert(SpriteBundle {
                texture: image_assets.freeze_others_side.clone(),
                sprite: Sprite {
                    custom_size: Some(sprite_custom_size),
                    color: Color::rgb(1.0, 1.0, 1.0),
                    ..default()
                },
                ..default()
            })
            .insert(Restitution::coefficient(0.1)),
        SideType::BounceBackwards => side
            .insert(SpriteBundle {
                texture: image_assets.bounce_backwards_side.clone(),
                sprite: Sprite {
                    custom_size: Some(sprite_custom_size),
                    color: Color::rgb(1.0, 1.0, 0.8),
                    ..default()
                },
                ..default()
            })
            .insert(Restitution::coefficient(0.1)),
        SideType::Destroy => side
            .insert(SpriteBundle {
                texture: image_assets.destroy_side.clone(),
                sprite: Sprite {
                    custom_size: Some(sprite_custom_size),
                    color: Color::rgb(1.0, 0.8, 0.8),
                    ..default()
                },
                ..default()
            })
            .insert(Restitution::coefficient(0.0)),
        SideType::Duplicate => side
            .insert(SpriteBundle {
                texture: image_assets.duplicate_side.clone(),
                sprite: Sprite {
                    custom_size: Some(sprite_custom_size),
                    color: Color::rgb(0.8, 0.8, 1.0),
                    ..default()
                },
                ..default()
            })
            .insert(Restitution::coefficient(0.5)),
        SideType::ResizeScoreAreas => side
            .insert(SpriteBundle {
                texture: image_assets.resize_side.clone(),
                sprite: Sprite {
                    custom_size: Some(sprite_custom_size),
                    color: Color::rgb(1.0, 0.8, 1.0),
                    ..default()
                },
                ..default()
            })
            .insert(Restitution::coefficient(0.33)),
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
    level_settings: Res<LevelSettings>,
    mut next_spawn_time: Local<SpawnTime>,
    mut balls_spawned_in_group: Local<u32>,
    audio_assets: Res<AudioAssets>,
    audio: Res<Audio>,
) {
    if Instant::now().duration_since(next_spawn_time.0) > Duration::ZERO {
        spawn_random_ball(commands, meshes, materials, &level_settings);

        audio.play_with_settings(
            audio_assets.launch.clone(),
            PlaybackSettings::ONCE.with_volume(SPAWN_SOUND_VOLUME * MASTER_VOLUME),
        );

        *balls_spawned_in_group += 1;

        if *balls_spawned_in_group >= level_settings.balls_per_group {
            *balls_spawned_in_group = 0;
            next_spawn_time.0 = Instant::now() + level_settings.time_between_groups;
        } else {
            next_spawn_time.0 = Instant::now() + level_settings.time_between_spawns_in_group;
        }
    }
}

/// Spawns a random ball at a random point with a random initial impulse
fn spawn_random_ball(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    level_settings: &Res<LevelSettings>,
) {
    let mut rng = rand::thread_rng();
    let ball_type = rng.gen::<BallType>();
    // TODO add more spawn points
    let spawn_point_x = rng.gen_range(BALL_MIN_START_X..=BALL_MAX_START_X);
    let impulse_x = rng.gen_range(level_settings.start_impulse_range_x.clone());
    let impulse_y = rng.gen_range(level_settings.start_impulse_range_y.clone());
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
    mut player_shape_query: Query<(&mut ExternalForce, &mut ExternalImpulse), With<PlayerShape>>,
    keycode: Res<Input<KeyCode>>,
    mut scroll_events: EventReader<MouseWheel>,
    rotate_sensitivity: Res<RotateSensitivity>,
) {
    for (mut force, mut impulse) in &mut player_shape_query {
        // translation
        if keycode.pressed(MOVE_LEFT_KEY) {
            force.force.x = -MOVE_SPEED;
        } else if keycode.pressed(MOVE_RIGHT_KEY) {
            force.force.x = MOVE_SPEED;
        } else {
            force.force.x = 0.0;
        }

        if keycode.pressed(MOVE_UP_KEY) {
            force.force.y = MOVE_SPEED;
        } else if keycode.pressed(MOVE_DOWN_KEY) {
            force.force.y = -MOVE_SPEED;
        } else {
            force.force.y = 0.0;
        }

        // rotation
        if keycode.pressed(ROTATE_CLOCKWISE_KEY) {
            force.torque = -ROTATE_SPEED * rotate_sensitivity.0;
        } else if keycode.pressed(ROTATE_COUNTERCLOCKWISE_KEY) {
            force.torque = ROTATE_SPEED * rotate_sensitivity.0;
        } else {
            force.torque = 0.0;
        }

        for event in scroll_events.iter() {
            impulse.torque_impulse = event.y * SCROLL_ROTATE_SPEED * rotate_sensitivity.0;
        }
    }
}

fn adjust_rotate_sensitivity(
    keycode: Res<Input<KeyCode>>,
    mut rotate_sensitivity: ResMut<RotateSensitivity>,
) {
    if keycode.just_pressed(INCREASE_ROTATE_SENSITIVITY_KEY) {
        rotate_sensitivity.0 += ROTATE_SENSITIVITY_ADJUST_AMOUNT;
    }

    if keycode.just_pressed(DECREASE_ROTATE_SENSITIVITY_KEY) {
        rotate_sensitivity.0 -= ROTATE_SENSITIVITY_ADJUST_AMOUNT;
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
    sides_query: Query<(&SideType, &SideId)>,
) {
    for event in collision_events.iter() {
        if let CollisionEvent::Started(a, b, _) = event {
            if let Some((ball, ball_entity)) = get_from_either::<Ball, &Ball>(*a, *b, &balls_query)
            {
                // a ball has hit something
                if entities_to_despawn.0.contains(&ball_entity) {
                    // this ball is going to be despawned, so don't mess with it any more
                    continue;
                }
                unfreeze_entity(ball_entity, &mut commands);
                if let Some((score_area, _)) =
                    get_from_either::<ScoreArea, &ScoreArea>(*a, *b, &score_areas_query)
                {
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

                    if let Some((side_type, side_entity)) =
                        get_from_either::<SideType, (&SideType, &SideId)>(*a, *b, &sides_query)
                    {
                        if let Ok(side_id) = sides_query.get_component::<SideId>(side_entity) {
                            // a ball has hit a side
                            side_type.add_side_effect(ball_entity, *side_id, &mut commands);
                        }
                    }
                }
            }
        }
    }
}

fn get_from_either<'a, T: Component, Q: ReadOnlyWorldQuery>(
    a: Entity,
    b: Entity,
    query: &'a Query<Q>,
) -> Option<(&'a T, Entity)> {
    if let Ok(component) = query.get_component::<T>(a) {
        return Some((component, a));
    }

    if let Ok(component) = query.get_component::<T>(b) {
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

/// Deals with entities that have had the bounce backwards effect added
fn handle_bounce_backwards_effect(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &BounceBackwardsEffect,
            &mut Transform,
            &mut Velocity,
        ),
        (Added<BounceBackwardsEffect>, Without<SideId>),
    >,
    sides_query: Query<(&SideId, &GlobalTransform)>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
) {
    let sides = sides_query
        .iter()
        .collect::<HashMap<&SideId, &GlobalTransform>>();
    for (entity, bounce_backwards_effect, mut transform, mut velocity) in query.iter_mut() {
        let hit_side_transform = sides
            .get(&bounce_backwards_effect.side_hit)
            .expect("hit side should have a transform");

        let opposide_side_id = bounce_backwards_effect.side_hit.opposite_side();
        let opposite_side_transform = sides
            .get(&opposide_side_id)
            .expect("opposite side should have a transform");

        let direction =
            (opposite_side_transform.translation() - hit_side_transform.translation()).normalize();
        velocity.linvel = (BOUNCE_BACKWARDS_VELOCITY * direction).truncate();
        transform.translation =
            opposite_side_transform.translation() + (direction * BOUNCE_BACKWARDS_DISTANCE);

        //TODO sound effect
        commands.entity(entity).remove::<BounceBackwardsEffect>();
    }
}

/// Deals with entities that have had the destroy effect added
fn handle_destroy_effect(
    mut commands: Commands,
    query: Query<Entity, Added<DestroyEffect>>,
    mut entities_to_despawn: ResMut<EntitiesToDespawn>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
) {
    for entity in query.iter() {
        entities_to_despawn.0.push(entity);

        //TODO sound effect
        commands.entity(entity).remove::<DestroyEffect>();
    }
}

/// Deals with entities that have had the duplicate effect added
fn handle_duplicate_effect(
    mut commands: Commands,
    query: Query<Entity, Added<DuplicateEffect>>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
) {
    for entity in query.iter() {
        //TODO duplicate ball

        //TODO sound effect
        commands.entity(entity).remove::<DuplicateEffect>();
    }
}

/// Deals with entities that have had the resize score areas effect added
fn handle_resize_score_areas_effect(
    mut commands: Commands,
    query: Query<Entity, Added<ResizeScoreAreasEffect>>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
) {
    for entity in query.iter() {
        //TODO resize score areas

        //TODO sound effect
        commands.entity(entity).remove::<ResizeScoreAreasEffect>();
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
            linear_threshold: -1.0,
            angular_threshold: -1.0,
        })
        .remove::<Frozen>();
}

/// Keeps the score display up to date
fn update_score_display(
    score: Res<Score>,
    mut score_text_query: Query<&mut Text, With<ScoreText>>,
) {
    for mut text in score_text_query.iter_mut() {
        text.sections[0].value = format!("Score: {}", score.0);
    }
}

/// Keeps the remaining time display up to date
fn update_time_display(
    end_time: Res<LevelEndTime>,
    mut time_text_query: Query<&mut Text, With<TimeText>>,
) {
    for mut text in time_text_query.iter_mut() {
        let time_left = end_time.0 - Instant::now();
        text.sections[0].value = format!("{}", time_left.as_secs());
    }
}

/// Keeps the rotation sensitivity display up to date
fn update_rotate_sensitivity_display(
    rotate_sensitivity: Res<RotateSensitivity>,
    mut rotate_sensitivity_text_query: Query<&mut Text, With<RotateSensitivityText>>,
) {
    if rotate_sensitivity.is_changed() {
        for mut text in rotate_sensitivity_text_query.iter_mut() {
            text.sections[0].value = format!("Rotation sensitivity: {:.1}", rotate_sensitivity.0);
        }
    }
}

/// Ends the level when the timer is up
fn end_level(mut next_state: ResMut<NextState<GameState>>, end_time: Res<LevelEndTime>) {
    if Instant::now().duration_since(end_time.0) > Duration::ZERO {
        next_state.set(GameState::BetweenLevels);
    }
}

/// Despawns entities that need to be despawned
fn despawn_entities(mut commands: Commands, mut entities_to_despawn: ResMut<EntitiesToDespawn>) {
    for entity in entities_to_despawn.0.drain(0..) {
        commands.entity(entity).despawn_recursive();
    }
}

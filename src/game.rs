use std::{collections::HashMap, ops::RangeInclusive, time::Duration};

use bevy::{
    ecs::{query::ReadOnlyWorldQuery, system::EntityCommands},
    input::mouse::MouseWheel,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use bevy_asset_loader::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_tweening::Lerp;
use instant::Instant;
use iyes_progress::{ProgressCounter, ProgressPlugin};
use rand::prelude::*;

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

pub const MASTER_VOLUME: f32 = 0.5;
const HIT_SOUND_VOLUME: f32 = 0.4;
const SPAWN_SOUND_VOLUME: f32 = 0.4;
const GOOD_SCORE_VOLUME: f32 = 0.33;
const BAD_SCORE_VOLUME: f32 = 0.4;
const BG_MUSIC_VOLUME: f32 = 0.5;

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
const EXTRA_POINT_BALL_SIZE: f32 = 25.0;
const BALL_COLLISION_GROUP: Group = Group::GROUP_2;

const FREEZE_DURATION: Duration = Duration::from_secs(3);
const BOUNCE_BACKWARDS_VELOCITY: f32 = 100.0;
const BOUNCE_BACKWARDS_DISTANCE: f32 = BALL_SIZE + 1.0;
const SCORE_AREA_RESIZE_DURATION: Duration = Duration::from_secs(5);
const SCORE_AREA_RESIZE_AMOUNT: f32 = 40.0;
const DUPLICATE_COOLDOWN_DURATION: Duration = Duration::from_millis(1000);

const TIMER_FONT_SIZE: f32 = 40.0;

const SCORE_AREA_HIT_ANIMATION_DURATION: Duration = Duration::from_millis(250);

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

        app.add_system(start_backround_music.in_schedule(OnEnter(GameState::Game)))
            .add_system(stop_background_music.in_schedule(OnExit(GameState::Game)));

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
                .before(handle_extra_points_effect)
                .run_if(in_state(GameState::Game)),
        )
        .add_system(
            remove_duplicate_cooldown
                .after(handle_duplicate_effect)
                .run_if(in_state(GameState::Game)),
        )
        .add_system(
            handle_resize_score_areas_effect
                .after(collisions)
                .run_if(in_state(GameState::Game)),
        )
        .add_system(
            handle_extreme_bounce_effect
                .after(collisions)
                .run_if(in_state(GameState::Game)),
        )
        .add_system(
            handle_extra_points_effect
                .after(collisions)
                .run_if(in_state(GameState::Game)),
        )
        .add_system(unfreeze_entities.run_if(in_state(GameState::Game)))
        .add_system(unresize_entities.run_if(in_state(GameState::Game)))
        .add_system(
            end_level
                .after(collisions)
                .run_if(in_state(GameState::Game)),
        )
        .add_system(
            animate_score_area_hit
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
    #[asset(path = "images/extra_bouncy_side.png")]
    extra_bouncy_side: Handle<Image>,
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
    #[asset(path = "images/extra_points_side.png")]
    extra_points_side: Handle<Image>,
}

#[derive(AssetCollection, Resource)]
pub struct AudioAssets {
    #[asset(path = "sounds/hit.ogg")]
    hit: Handle<AudioSource>,
    #[asset(path = "sounds/up.ogg")]
    up: Handle<AudioSource>,
    #[asset(path = "sounds/up_more_2.ogg")]
    up_more: Handle<AudioSource>,
    #[asset(path = "sounds/down.ogg")]
    down: Handle<AudioSource>,
    #[asset(path = "sounds/launch.ogg")]
    launch: Handle<AudioSource>,
    #[asset(path = "sounds/boop.ogg")]
    boop: Handle<AudioSource>,
    #[asset(path = "sounds/duplicate_2.ogg")]
    duplicate: Handle<AudioSource>,
    #[asset(path = "sounds/explode.ogg")]
    explode: Handle<AudioSource>,
    #[asset(path = "sounds/extra_points.ogg")]
    extra_points: Handle<AudioSource>,
    #[asset(path = "sounds/resize.ogg")]
    resize: Handle<AudioSource>,
    #[asset(path = "sounds/good_2.ogg")]
    good: Handle<AudioSource>,
    #[asset(path = "sounds/bad.ogg")]
    bad: Handle<AudioSource>,
    #[asset(path = "sounds/choobcasher2.ogg")]
    game_music: Handle<AudioSource>,
    #[asset(path = "sounds/choobcasher.ogg")]
    pub menu_music: Handle<AudioSource>,
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
    /// Maximum amount of time before a new group gets spawned if there are no balls left on screen
    max_respite_time: Duration,
    /// Amount of time between spawning balls in the same group
    time_between_spawns_in_group: Duration,
    /// Number of balls spawned per group
    balls_per_group: u32,
    /// Whether type B balls will spawn
    type_b_active: bool,
    /// Whether type D balls will spawn
    type_d_active: bool,
    /// Settings for where to spawn balls
    spawn_points: Vec<SpawnPoint>,
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
            time_between_groups: Duration::from_secs(10),
            max_respite_time: Duration::from_secs(2),
            time_between_spawns_in_group: Duration::from_millis(500),
            balls_per_group: 3,
            type_b_active: false,
            type_d_active: false,
            spawn_points: SpawnPoint::four_sides(5.0, 20.0),
            duration: Duration::from_secs(32),
            sides_to_unlock: vec![SideType::FreezeOthers],
            min_score: 1,
        }
    }

    /// Builds settings for the level after this one
    pub fn next_level(&self) -> LevelSettings {
        match self.id {
            1 => LevelSettings {
                id: 2,
                time_between_groups: Duration::from_secs(9),
                max_respite_time: Duration::from_secs(2),
                time_between_spawns_in_group: Duration::from_millis(500),
                balls_per_group: 3,
                type_b_active: true,
                type_d_active: false,
                spawn_points: SpawnPoint::four_sides(5.0, 20.0),
                duration: Duration::from_secs(40),
                sides_to_unlock: vec![SideType::BounceBackwards],
                min_score: 1,
            },
            2 => LevelSettings {
                id: 3,
                time_between_groups: Duration::from_secs(8),
                max_respite_time: Duration::from_secs(2),
                time_between_spawns_in_group: Duration::from_millis(500),
                balls_per_group: 3,
                type_b_active: true,
                type_d_active: true,
                spawn_points: SpawnPoint::four_sides(5.0, 20.0),
                duration: Duration::from_secs(50),
                sides_to_unlock: vec![SideType::ResizeScoreAreas],
                min_score: 1,
            },
            3 => LevelSettings {
                id: 4,
                time_between_groups: Duration::from_secs(7),
                max_respite_time: Duration::from_secs(2),
                time_between_spawns_in_group: Duration::from_millis(500),
                balls_per_group: 4,
                type_b_active: true,
                type_d_active: true,
                spawn_points: SpawnPoint::four_sides(5.0, 22.0),
                duration: Duration::from_secs(64),
                sides_to_unlock: vec![SideType::Destroy, SideType::ExtraPoints],
                min_score: 3,
            },
            4 => LevelSettings {
                id: 5,
                time_between_groups: Duration::from_secs(7),
                max_respite_time: Duration::from_secs(2),
                time_between_spawns_in_group: Duration::from_millis(500),
                balls_per_group: 4,
                type_b_active: true,
                type_d_active: true,
                spawn_points: SpawnPoint::four_sides(5.0, 25.0),
                duration: Duration::from_secs(64),
                sides_to_unlock: vec![SideType::Duplicate, SideType::ExtremeBounce],
                min_score: 5,
            },
            5 => LevelSettings {
                id: 6,
                time_between_groups: Duration::from_secs(7),
                max_respite_time: Duration::from_secs(1),
                time_between_spawns_in_group: Duration::from_millis(500),
                balls_per_group: 5,
                type_b_active: true,
                type_d_active: true,
                spawn_points: SpawnPoint::four_sides(6.0, 27.0),
                duration: Duration::from_secs(64),
                sides_to_unlock: vec![],
                min_score: 7,
            },
            6 => LevelSettings {
                id: 7,
                time_between_groups: Duration::from_secs(7),
                max_respite_time: Duration::from_secs(1),
                time_between_spawns_in_group: Duration::from_millis(500),
                balls_per_group: 5,
                type_b_active: true,
                type_d_active: true,
                spawn_points: SpawnPoint::four_sides(7.0, 30.0),
                duration: Duration::from_secs(64),
                sides_to_unlock: vec![],
                min_score: 10,
            },
            _ => LevelSettings {
                id: self.id + 1,
                time_between_groups: self.time_between_groups,
                max_respite_time: self.max_respite_time,
                time_between_spawns_in_group: self.time_between_spawns_in_group,
                balls_per_group: self.balls_per_group + 1,
                type_b_active: true,
                type_d_active: true,
                spawn_points: self.spawn_points.clone(),
                duration: self.duration,
                sides_to_unlock: vec![],
                min_score: self.min_score + 3,
            },
        }
    }
}

#[derive(Clone)]
struct SpawnPoint {
    /// Range of possible X coordinates
    start_position_range_x: RangeInclusive<f32>,
    /// Range of possible Y coordinates
    start_position_range_y: RangeInclusive<f32>,
    /// The range of possible initial impulses in the X direction on spawned balls
    start_impulse_range_x: RangeInclusive<f32>,
    /// The range of possible initial impulses in the Y direction on spawned balls
    start_impulse_range_y: RangeInclusive<f32>,
}

impl SpawnPoint {
    /// Builds a spawn point next to the top wall
    fn top(min_impulse: f32, max_impulse: f32) -> SpawnPoint {
        SpawnPoint {
            start_position_range_x: (-PLAY_AREA_RADIUS / 3.0)..=(PLAY_AREA_RADIUS / 3.0),
            start_position_range_y: (PLAY_AREA_RADIUS - BALL_SIZE - 1.0)
                ..=(PLAY_AREA_RADIUS - BALL_SIZE - 1.0),
            start_impulse_range_x: -10.0..=10.0,
            start_impulse_range_y: -max_impulse..=-min_impulse,
        }
    }

    /// Builds a spawn point next to the top wall
    fn bottom(min_impulse: f32, max_impulse: f32) -> SpawnPoint {
        SpawnPoint {
            start_position_range_x: (-PLAY_AREA_RADIUS / 3.0)..=(PLAY_AREA_RADIUS / 3.0),
            start_position_range_y: (-PLAY_AREA_RADIUS + BALL_SIZE + 1.0)
                ..=(-PLAY_AREA_RADIUS + BALL_SIZE + 1.0),
            start_impulse_range_x: -10.0..=10.0,
            start_impulse_range_y: min_impulse..=max_impulse,
        }
    }

    /// Builds a spawn point next to the left wall
    fn left(min_impulse: f32, max_impulse: f32) -> SpawnPoint {
        SpawnPoint {
            start_position_range_x: (-PLAY_AREA_RADIUS + BALL_SIZE + 1.0)
                ..=(-PLAY_AREA_RADIUS + BALL_SIZE + 1.0),
            start_position_range_y: (-PLAY_AREA_RADIUS / 3.0)..=(PLAY_AREA_RADIUS / 3.0),
            start_impulse_range_x: min_impulse..=max_impulse,
            start_impulse_range_y: -10.0..=10.0,
        }
    }

    /// Builds a spawn point next to the right wall
    fn right(min_impulse: f32, max_impulse: f32) -> SpawnPoint {
        SpawnPoint {
            start_position_range_x: (PLAY_AREA_RADIUS - BALL_SIZE - 1.0)
                ..=(PLAY_AREA_RADIUS - BALL_SIZE - 1.0),
            start_position_range_y: (-PLAY_AREA_RADIUS / 3.0)..=(PLAY_AREA_RADIUS / 3.0),
            start_impulse_range_x: -max_impulse..=-min_impulse,
            start_impulse_range_y: -10.0..=10.0,
        }
    }

    /// Builds spawn points next to each wall
    fn four_sides(min_impulse: f32, max_impulse: f32) -> Vec<SpawnPoint> {
        vec![
            SpawnPoint::top(min_impulse, max_impulse),
            SpawnPoint::bottom(min_impulse, max_impulse),
            SpawnPoint::left(min_impulse, max_impulse),
            SpawnPoint::right(min_impulse, max_impulse),
        ]
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

#[derive(Resource)]
struct GameMusicController(Handle<AudioSink>);

#[derive(Resource)]
pub struct Score(pub i32);

#[derive(Resource)]
struct LevelEndTime(Instant);

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
    ExtremeBounce,
    ExtraPoints,
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
            SideType::ExtremeBounce => {
                commands.entity(entity).insert(ExtremeBounceEffect);
            }
            SideType::ExtraPoints => {
                commands.entity(entity).insert(ExtraPointsEffect);
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
            SideType::ExtremeBounce => "EXTREME BOUNCE",
            SideType::ExtraPoints => "Importantize",
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
            SideType::BounceBackwards => "Bounces balls backwards out the other side",
            SideType::Destroy => "Destroys balls that hit it",
            SideType::Duplicate => "Duplicates balls that hit it",
            SideType::ResizeScoreAreas => "Temporarily increases the size of the score area matching the ball that hit it, decreases the size of other score areas, and prevents incorrect scores from occurring",
            SideType::ExtremeBounce => "Contains the maximum bounciness allowed by the FDA",
            SideType::ExtraPoints => "Makes balls that hit it worth 1 additional point (don't get too excited, the effect can only be applied once per ball)"
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
struct DuplicateCooldown {
    remove_at: Instant,
}

#[derive(Component)]
struct ResizeScoreAreasEffect;

#[derive(Component)]
struct ExtremeBounceEffect;

#[derive(Component, Clone, Copy)]
struct ExtraPointsEffect;

#[derive(Component)]
struct Frozen {
    unfreeze_at: Instant,
    original_velocity: Velocity,
}

#[derive(Component)]
struct Resized {
    unresize_at: Instant,
    original_mesh: Mesh2dHandle,
    original_collider: Collider,
    penalty_disabled: bool,
}

#[derive(Component, Clone)]
struct Ball {
    ball_type: BallType,
    points: u16,
}

#[derive(PartialEq, Clone, Copy)]
enum BallType {
    A,
    B,
    C,
    D,
}

impl BallType {
    /// Generates a random ball type
    fn random<R: Rng>(level_settings: &LevelSettings, rng: &mut R) -> BallType {
        if level_settings.type_b_active && level_settings.type_d_active {
            match rng.gen_range(0..=3) {
                0 => BallType::A,
                1 => BallType::B,
                2 => BallType::C,
                3 => BallType::D,
                _ => unreachable!(),
            }
        } else if level_settings.type_b_active {
            match rng.gen_range(0..=2) {
                0 => BallType::A,
                1 => BallType::B,
                2 => BallType::C,
                _ => unreachable!(),
            }
        } else if level_settings.type_d_active {
            match rng.gen_range(0..=2) {
                0 => BallType::A,
                1 => BallType::C,
                2 => BallType::D,
                _ => unreachable!(),
            }
        } else {
            match rng.gen_range(0..=1) {
                0 => BallType::A,
                1 => BallType::C,
                _ => unreachable!(),
            }
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
            BallType::D => Color::rgb(0.0, 0.75, 1.0),
        }
    }
}

#[derive(Component)]
struct ScoreArea(BallType);

#[derive(Component)]
struct AnimateScoreAreaHit {
    score_change: i32,
    hit_time: Instant,
}

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
                "loading...\n0%",
                TextStyle {
                    font: asset_server.load(MONO_FONT),
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
                loading_text.sections[0].value = format!("loading...\n{percent_done:.0}%");
            }
        }
    }
}

/// Sets up the game.
#[allow(clippy::too_many_arguments)]
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
    commands
        .spawn(MaterialMesh2dBundle {
            mesh: meshes
                .add(shape::Circle::new(SCORE_AREA_SIZE).into())
                .into(),
            material: materials.add(ColorMaterial::from(color_for_score_area(&ScoreArea(
                BallType::A,
            )))),
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

    if level_settings.type_b_active {
        commands
            .spawn(MaterialMesh2dBundle {
                mesh: meshes
                    .add(shape::Circle::new(SCORE_AREA_SIZE).into())
                    .into(),
                material: materials.add(ColorMaterial::from(color_for_score_area(&ScoreArea(
                    BallType::B,
                )))),
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
    }

    commands
        .spawn(MaterialMesh2dBundle {
            mesh: meshes
                .add(shape::Circle::new(SCORE_AREA_SIZE).into())
                .into(),
            material: materials.add(ColorMaterial::from(color_for_score_area(&ScoreArea(
                BallType::C,
            )))),
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

    if level_settings.type_d_active {
        commands
            .spawn(MaterialMesh2dBundle {
                mesh: meshes
                    .add(shape::Circle::new(SCORE_AREA_SIZE).into())
                    .into(),
                material: materials.add(ColorMaterial::from(color_for_score_area(&ScoreArea(
                    BallType::D,
                )))),
                ..default()
            })
            .insert(Collider::ball(SCORE_AREA_SIZE))
            .insert(Sensor)
            .insert(Transform::from_translation(Vec3::new(
                -PLAY_AREA_RADIUS,
                -PLAY_AREA_RADIUS,
                0.0,
            )))
            .insert(GameComponent)
            .insert(ScoreArea(BallType::D));
    }

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
                        format!("level {}", level_settings.id),
                        TextStyle {
                            font: asset_server.load(MONO_FONT),
                            font_size: 28.0,
                            color: Color::Rgba {
                                red: 0.75,
                                green: 0.75,
                                blue: 0.75,
                                alpha: 1.0,
                            },
                        },
                    )
                    .with_text_alignment(TextAlignment::Center)
                    .with_style(Style {
                        margin: UiRect {
                            bottom: Val::Px(5.0),
                            ..default()
                        },
                        ..default()
                    }),
                )
                .insert(LevelText);

            // minimum score display
            parent.spawn(
                TextBundle::from_section(
                    format!("score needed: {}", level_settings.min_score),
                    TextStyle {
                        font: asset_server.load(MONO_FONT),
                        font_size: 22.0,
                        color: Color::Rgba {
                            red: 0.75,
                            green: 0.75,
                            blue: 0.75,
                            alpha: 1.0,
                        },
                    },
                )
                .with_text_alignment(TextAlignment::Center)
                .with_style(Style {
                    margin: UiRect {
                        bottom: Val::Px(5.0),
                        ..default()
                    },
                    ..default()
                }),
            );

            // score display
            parent
                .spawn(
                    TextBundle::from_section(
                        "score: 0",
                        TextStyle {
                            font: asset_server.load(MONO_FONT),
                            font_size: 33.0,
                            color: Color::WHITE,
                        },
                    )
                    .with_text_alignment(TextAlignment::Center)
                    .with_style(Style {
                        margin: UiRect {
                            bottom: Val::Px(5.0),
                            ..default()
                        },
                        ..default()
                    }),
                )
                .insert(ScoreText);
        });

    // timer display
    commands
        .spawn(
            TextBundle::from_section(
                "",
                TextStyle {
                    font: asset_server.load(MONO_FONT),
                    font_size: TIMER_FONT_SIZE,
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
                format!("rotation sensitivity: {:.1}", rotate_sensitivity.0),
                TextStyle {
                    font: asset_server.load(MONO_FONT),
                    font_size: 14.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
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

/// Determines what color the provided score area should be
fn color_for_score_area(score_area: &ScoreArea) -> Color {
    let mut color = score_area.0.color();
    color.set_a(0.1);

    color
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
            .insert(Restitution::coefficient(0.5)),
        SideType::ExtremeBounce => side
            .insert(SpriteBundle {
                texture: image_assets.extra_bouncy_side.clone(),
                sprite: Sprite {
                    custom_size: Some(sprite_custom_size),
                    color: Color::rgb(0.5, 1.0, 0.5),
                    ..default()
                },
                ..default()
            })
            .insert(Restitution::coefficient(5.0)),
        SideType::ExtraPoints => side
            .insert(SpriteBundle {
                texture: image_assets.extra_points_side.clone(),
                sprite: Sprite {
                    custom_size: Some(sprite_custom_size),
                    color: Color::rgb(0.8, 1.0, 1.0),
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
#[allow(clippy::too_many_arguments)]
fn spawn_balls(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    level_settings: Res<LevelSettings>,
    balls_query: Query<&Ball>,
    mut next_spawn_time: Local<SpawnTime>,
    mut balls_spawned_in_group: Local<u32>,
    audio_assets: Res<AudioAssets>,
    audio: Res<Audio>,
) {
    if balls_query.is_empty()
        && next_spawn_time.0.saturating_duration_since(Instant::now())
            > level_settings.max_respite_time
    {
        // there are no balls left on screen, so reduce time until next group is spawned
        next_spawn_time.0 = Instant::now() + level_settings.max_respite_time;
    } else if Instant::now().saturating_duration_since(next_spawn_time.0) > Duration::ZERO {
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
    level_settings: &LevelSettings,
) {
    let mut rng = rand::thread_rng();
    let ball_type = BallType::random(level_settings, &mut rng);
    let spawn_point = level_settings
        .spawn_points
        .choose(&mut rng)
        .expect("at least one spawn point should be defined");
    let spawn_point_x = rng.gen_range(spawn_point.start_position_range_x.clone());
    let spawn_point_y = rng.gen_range(spawn_point.start_position_range_y.clone());
    let impulse_x = rng.gen_range(spawn_point.start_impulse_range_x.clone());
    let impulse_y = rng.gen_range(spawn_point.start_impulse_range_y.clone());
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
        spawn_point_y,
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
            impulse.torque_impulse =
                event.y.clamp(-1.0, 1.0) * SCROLL_ROTATE_SPEED * rotate_sensitivity.0;
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
#[allow(clippy::too_many_arguments)]
fn collisions(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut score: ResMut<Score>,
    mut entities_to_despawn: ResMut<EntitiesToDespawn>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
    balls_query: Query<&Ball>,
    score_areas_query: Query<(&ScoreArea, Option<&Resized>)>,
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
                if let Some((score_area, score_area_entity)) = get_from_either::<
                    ScoreArea,
                    (&ScoreArea, Option<&Resized>),
                >(
                    *a, *b, &score_areas_query
                ) {
                    // a ball has hit a score area
                    if ball.ball_type == score_area.0 {
                        score.0 += i32::from(ball.points);
                        commands
                            .entity(score_area_entity)
                            .insert(AnimateScoreAreaHit {
                                score_change: i32::from(ball.points),
                                hit_time: Instant::now(),
                            });
                        audio.play_with_settings(
                            audio_assets.good.clone(),
                            PlaybackSettings::ONCE.with_volume(GOOD_SCORE_VOLUME * MASTER_VOLUME),
                        );
                    } else {
                        if let Ok(resized) =
                            score_areas_query.get_component::<Resized>(score_area_entity)
                        {
                            if resized.penalty_disabled {
                                // this score area doesn't penalize incorrect hits right now
                                continue;
                            }
                        }
                        score.0 -= i32::from(ball.points);
                        commands
                            .entity(score_area_entity)
                            .insert(AnimateScoreAreaHit {
                                score_change: -i32::from(ball.points),
                                hit_time: Instant::now(),
                            });
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
        audio.play_with_settings(
            audio_assets.up.clone(),
            PlaybackSettings::ONCE.with_volume(0.75 * MASTER_VOLUME),
        );
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
        audio.play_with_settings(
            audio_assets.down.clone(),
            PlaybackSettings::ONCE.with_volume(1.0 * MASTER_VOLUME),
        );
        commands.entity(entity).remove::<FreezeOthersEffect>();
    }
}

type AddedBounceBackwardsEffectTuple = (Added<BounceBackwardsEffect>, Without<SideId>);

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
        AddedBounceBackwardsEffectTuple,
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

        audio.play_with_settings(
            audio_assets.boop.clone(),
            PlaybackSettings::ONCE.with_volume(0.33 * MASTER_VOLUME),
        );
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

        audio.play_with_settings(
            audio_assets.explode.clone(),
            PlaybackSettings::ONCE.with_volume(0.33 * MASTER_VOLUME),
        );
        commands.entity(entity).remove::<DestroyEffect>();
    }
}

type EntityToDuplicateTuple<'a> = (
    Entity,
    &'a Ball,
    &'a Transform,
    &'a Velocity,
    Option<&'a ExtraPointsEffect>,
    Option<&'a DuplicateCooldown>,
);

/// Deals with entities that have had the duplicate effect added
fn handle_duplicate_effect(
    mut commands: Commands,
    query: Query<EntityToDuplicateTuple, Added<DuplicateEffect>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
) {
    for (entity, ball, transform, velocity, extra_points_effect, duplicate_cooldown) in query.iter()
    {
        if duplicate_cooldown.is_some() {
            commands.entity(entity).remove::<DuplicateEffect>();
            continue;
        }

        let impulse = Vec2::new(5.0, 5.0); //TODO make this parallel to hit side
        let mut new_ball = spawn_ball(
            &mut commands,
            Ball {
                ball_type: ball.ball_type,
                points: 1,
            },
            &mut meshes,
            &mut materials,
        );

        new_ball
            .insert(TransformBundle::from(*transform))
            .insert(*velocity)
            .insert(ExternalImpulse {
                impulse,
                ..default()
            })
            .insert(DuplicateCooldown {
                remove_at: Instant::now() + DUPLICATE_COOLDOWN_DURATION,
            });

        if let Some(extra_points_effect) = extra_points_effect {
            new_ball.insert(*extra_points_effect);
        }

        audio.play_with_settings(
            audio_assets.duplicate.clone(),
            PlaybackSettings::ONCE.with_volume(0.4 * MASTER_VOLUME),
        );

        commands
            .entity(entity)
            .remove::<DuplicateEffect>()
            .insert(DuplicateCooldown {
                remove_at: Instant::now() + DUPLICATE_COOLDOWN_DURATION,
            });
    }
}

/// Removes the duplication cooldown component from entities once the cooldown expires
fn remove_duplicate_cooldown(mut commands: Commands, query: Query<(Entity, &DuplicateCooldown)>) {
    for (entity, cooldown) in query.iter() {
        if Instant::now().saturating_duration_since(cooldown.remove_at) > Duration::ZERO {
            commands.entity(entity).remove::<DuplicateCooldown>();
        }
    }
}

/// Deals with entities that have had the resize score areas effect added
fn handle_resize_score_areas_effect(
    mut commands: Commands,
    query: Query<(Entity, &Ball), Added<ResizeScoreAreasEffect>>,
    mut score_areas_query: Query<(Entity, &ScoreArea, &mut Mesh2dHandle, &mut Collider)>,
    mut meshes: ResMut<Assets<Mesh>>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
) {
    for (ball_entity, ball) in query.iter() {
        for (score_area_entity, score_area, mut mesh, mut collider) in score_areas_query.iter_mut()
        {
            commands.entity(score_area_entity).insert(Resized {
                unresize_at: Instant::now() + SCORE_AREA_RESIZE_DURATION,
                original_mesh: meshes
                    .add(shape::Circle::new(SCORE_AREA_SIZE).into())
                    .into(),
                original_collider: Collider::ball(SCORE_AREA_SIZE),
                penalty_disabled: true,
            });

            if ball.ball_type == score_area.0 {
                *mesh = meshes
                    .add(shape::Circle::new(SCORE_AREA_SIZE + SCORE_AREA_RESIZE_AMOUNT).into())
                    .into();
                *collider = Collider::ball(SCORE_AREA_SIZE + SCORE_AREA_RESIZE_AMOUNT);
            } else {
                *mesh = meshes
                    .add(shape::Circle::new(SCORE_AREA_SIZE - SCORE_AREA_RESIZE_AMOUNT).into())
                    .into();
                *collider = Collider::ball(SCORE_AREA_SIZE - SCORE_AREA_RESIZE_AMOUNT);
            }
        }

        audio.play_with_settings(
            audio_assets.resize.clone(),
            PlaybackSettings::ONCE.with_volume(0.33 * MASTER_VOLUME),
        );

        commands
            .entity(ball_entity)
            .remove::<ResizeScoreAreasEffect>();
    }
}

/// Deals with entities that have had the extreme bounce effect added
fn handle_extreme_bounce_effect(
    mut commands: Commands,
    query: Query<Entity, Added<ExtremeBounceEffect>>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
) {
    for entity in query.iter() {
        audio.play_with_settings(
            audio_assets.up_more.clone(),
            PlaybackSettings::ONCE.with_volume(0.33 * MASTER_VOLUME),
        );
        commands.entity(entity).remove::<ExtremeBounceEffect>();
    }
}

/// Deals with entities that have had the resize score areas effect added
fn handle_extra_points_effect(
    mut query: Query<(&mut Ball, &mut Mesh2dHandle, &mut Collider), Added<ExtraPointsEffect>>,
    mut meshes: ResMut<Assets<Mesh>>,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
) {
    for (mut ball, mut mesh, mut collider) in query.iter_mut() {
        ball.points = 2;
        *mesh = meshes
            .add(shape::Circle::new(EXTRA_POINT_BALL_SIZE).into())
            .into();
        *collider = Collider::ball(EXTRA_POINT_BALL_SIZE);

        audio.play_with_settings(
            audio_assets.extra_points.clone(),
            PlaybackSettings::ONCE.with_volume(0.66 * MASTER_VOLUME),
        );
    }
}

/// Handles unfreezing entities
fn unfreeze_entities(
    mut commands: Commands,
    frozen_query: Query<(Entity, &Frozen), With<RigidBody>>,
) {
    for (entity, frozen) in frozen_query.iter() {
        if Instant::now().saturating_duration_since(frozen.unfreeze_at) > Duration::ZERO {
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

/// Handles un-resizing entities
fn unresize_entities(
    mut commands: Commands,
    mut resized_query: Query<(Entity, &Resized, &mut Mesh2dHandle, &mut Collider)>,
) {
    for (entity, resized, mut mesh, mut collider) in resized_query.iter_mut() {
        if Instant::now().saturating_duration_since(resized.unresize_at) > Duration::ZERO {
            *mesh = resized.original_mesh.clone();
            *collider = resized.original_collider.clone();
            commands.entity(entity).remove::<Resized>();
        }
    }
}

/// Handles animating hit score areas
fn animate_score_area_hit(
    mut commands: Commands,
    query: Query<(
        Entity,
        &ScoreArea,
        &AnimateScoreAreaHit,
        &Handle<ColorMaterial>,
    )>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, score_area, animation, material_handle) in query.iter() {
        let material = materials
            .get_mut(material_handle)
            .expect("material should exist");
        let base_color = color_for_score_area(score_area);
        let start_animation_channel = if animation.score_change > 0 { 1.0 } else { 0.1 };
        let start_animation_alpha_channel = if animation.score_change.abs() == 1 {
            0.5
        } else {
            1.0
        };

        let animation_progress: f32 = Instant::now()
            .saturating_duration_since(animation.hit_time)
            .as_secs_f32()
            / SCORE_AREA_HIT_ANIMATION_DURATION.as_secs_f32();
        if animation_progress >= 1.0 || animation.score_change == 0 {
            material.color = base_color;
            commands.entity(entity).remove::<AnimateScoreAreaHit>();
        } else {
            material.color = Color::Rgba {
                red: start_animation_channel.lerp(&base_color.r(), &animation_progress),
                green: start_animation_channel.lerp(&base_color.g(), &animation_progress),
                blue: start_animation_channel.lerp(&base_color.b(), &animation_progress),
                alpha: start_animation_alpha_channel.lerp(&base_color.a(), &animation_progress),
            };
        }
    }
}

/// Keeps the score display up to date
fn update_score_display(
    score: Res<Score>,
    mut score_text_query: Query<&mut Text, With<ScoreText>>,
) {
    for mut text in score_text_query.iter_mut() {
        text.sections[0].value = format!("score: {}", score.0);
    }
}

/// Keeps the remaining time display up to date
fn update_time_display(
    end_time: Res<LevelEndTime>,
    mut time_text_query: Query<&mut Text, With<TimeText>>,
) {
    for mut text in time_text_query.iter_mut() {
        let time_left = end_time.0.saturating_duration_since(Instant::now());
        let seconds_left = time_left.as_secs();
        if seconds_left <= 5 {
            text.sections[0].value = format!("{:.1}", time_left.as_millis() as f32 / 1000.0);
        } else {
            text.sections[0].value = format!("{seconds_left}");
        }

        if seconds_left == 0 {
            text.sections[0].style.font_size = TIMER_FONT_SIZE + 27.0;
            text.sections[0].style.color = Color::rgb(1.0, 0.0, 0.0);
        } else if seconds_left <= 1 {
            text.sections[0].style.font_size = TIMER_FONT_SIZE + 20.0;
            text.sections[0].style.color = Color::rgb(1.0, 0.1, 0.1);
        } else if seconds_left <= 2 {
            text.sections[0].style.font_size = TIMER_FONT_SIZE + 14.0;
            text.sections[0].style.color = Color::rgb(1.0, 0.3, 0.3);
        } else if seconds_left <= 3 {
            text.sections[0].style.font_size = TIMER_FONT_SIZE + 9.0;
            text.sections[0].style.color = Color::rgb(1.0, 0.5, 0.5);
        } else if seconds_left <= 4 {
            text.sections[0].style.font_size = TIMER_FONT_SIZE + 5.0;
            text.sections[0].style.color = Color::rgb(1.0, 0.7, 0.7);
        } else if seconds_left <= 5 {
            text.sections[0].style.font_size = TIMER_FONT_SIZE + 2.0;
            text.sections[0].style.color = Color::rgb(1.0, 0.9, 0.9);
        }
    }
}

/// Keeps the rotation sensitivity display up to date
fn update_rotate_sensitivity_display(
    rotate_sensitivity: Res<RotateSensitivity>,
    mut rotate_sensitivity_text_query: Query<&mut Text, With<RotateSensitivityText>>,
) {
    if rotate_sensitivity.is_changed() {
        for mut text in rotate_sensitivity_text_query.iter_mut() {
            text.sections[0].value = format!("rotation sensitivity: {:.1}", rotate_sensitivity.0);
        }
    }
}

/// Ends the level when the timer is up
fn end_level(mut next_state: ResMut<NextState<GameState>>, end_time: Res<LevelEndTime>) {
    if Instant::now().saturating_duration_since(end_time.0) > Duration::ZERO {
        next_state.set(GameState::BetweenLevels);
    }
}

/// Starts playing the background music
fn start_backround_music(
    mut commands: Commands,
    audio: Res<Audio>,
    audio_assets: Res<AudioAssets>,
    audio_sinks: Res<Assets<AudioSink>>,
) {
    let handle = audio_sinks.get_handle(audio.play_with_settings(
        audio_assets.game_music.clone(),
        PlaybackSettings::LOOP.with_volume(BG_MUSIC_VOLUME * MASTER_VOLUME),
    ));

    commands.insert_resource(GameMusicController(handle));
}

/// Stops playing the background music
fn stop_background_music(
    music_controller: Res<GameMusicController>,
    audio_sinks: Res<Assets<AudioSink>>,
) {
    if let Some(sink) = audio_sinks.get(&music_controller.0) {
        sink.stop();
    }
}

/// Despawns entities that need to be despawned
fn despawn_entities(mut commands: Commands, mut entities_to_despawn: ResMut<EntitiesToDespawn>) {
    for entity in entities_to_despawn.0.drain(0..) {
        commands.entity(entity).despawn_recursive();
    }
}

use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::common_conditions::input_toggle_active,
    prelude::*,
    window::{WindowResized, WindowResolution},
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::prelude::*;

mod menu;
use menu::*;

mod game;
use game::*;

mod between_levels;
use between_levels::*;

const DEV_MODE: bool = true;

const MAIN_FONT: &str = "fonts/Quicksand-Medium.ttf";

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 720.0;

const NORMAL_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const HOVERED_BUTTON: Color = Color::rgb(0.35, 0.35, 0.35);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameState {
    #[default]
    Menu,
    GameLoading,
    Game,
    BetweenLevels,
}

#[derive(Component)]
pub struct MainCamera;

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Msaa::Sample4)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Side Effects".into(),
                resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                // Tells wasm to resize the window according to the available canvas
                fit_canvas_to_parent: true,
                // Tells wasm not to override default event handling, like F5, Ctrl+R etc.
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        .insert_resource(RapierConfiguration {
            gravity: Vec2::ZERO,
            ..default()
        })
        .add_state::<GameState>()
        .add_startup_system(setup)
        .add_plugin(MenuPlugin)
        .add_plugin(GamePlugin)
        .add_plugin(BetweenLevelsPlugin)
        .add_system(zoom_based_on_window_size)
        .add_system(button_color_system);

    if DEV_MODE {
        app.add_system(bevy::window::close_on_esc)
            .add_plugin(LogDiagnosticsPlugin::default())
            .add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_plugin(
                WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::Equals)),
            )
            .add_plugin(RapierDebugRenderPlugin::default());
    }

    app.run();
}

fn setup(mut commands: Commands) {
    //TODO commands.spawn(Camera2dBundle::default());

    commands
        .spawn((
            Camera2dBundle {
                camera: Camera {
                    hdr: true, // 1. HDR is required for bloom
                    ..default()
                },
                tonemapping: Tonemapping::TonyMcMapface, // 2. Using a tonemapper that desaturates to white is recommended
                ..default()
            },
            BloomSettings::default(), // 3. Enable bloom for the camera
        ))
        .insert(MainCamera);
}

/// Adjusts the camera zoom when the window is resized
fn zoom_based_on_window_size(
    mut camera_query: Query<&mut OrthographicProjection, With<MainCamera>>,
    mut resize_reader: EventReader<WindowResized>,
) {
    let mut projection = camera_query.single_mut();

    for event in resize_reader.iter() {
        projection.scale = (WINDOW_WIDTH / event.width).max(WINDOW_HEIGHT / event.height);
    }
}

type InteractedButtonTuple = (Changed<Interaction>, With<Button>);

/// Handles changing button colors when they're interacted with.
fn button_color_system(
    mut interaction_query: Query<(&Interaction, &mut BackgroundColor), InteractedButtonTuple>,
) {
    for (interaction, mut color) in interaction_query.iter_mut() {
        *color = match *interaction {
            Interaction::Clicked => PRESSED_BUTTON.into(),
            Interaction::Hovered => HOVERED_BUTTON.into(),
            Interaction::None => NORMAL_BUTTON.into(),
        }
    }
}

/// Generic system that takes a component as a parameter, and will despawn all entities with that component
fn despawn_components_system<T: Component>(
    to_despawn: Query<Entity, With<T>>,
    mut commands: Commands,
) {
    despawn_components(to_despawn, &mut commands);
}

fn despawn_components<T: Component>(to_despawn: Query<Entity, With<T>>, commands: &mut Commands) {
    for entity in to_despawn.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

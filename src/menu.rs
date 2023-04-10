use crate::*;

const INTRO_TEXT: &str = include_str!("intro_text.txt");

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(menu_setup.in_schedule(OnEnter(GameState::Menu)))
            .add_system(
                despawn_components_system::<MenuComponent>.in_schedule(OnExit(GameState::Menu)),
            )
            .add_system(start_button_system);
    }
}

#[derive(Component)]
struct MenuComponent;

#[derive(Component)]
struct StartButton;

fn menu_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // title text
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Auto),
                position_type: PositionType::Absolute,
                position: UiRect {
                    top: Val::Px(10.0),
                    ..default()
                },
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .insert(MenuComponent)
        .with_children(|parent| {
            parent.spawn(
                TextBundle::from_section(
                    "Extreme Bounce Party 2000",
                    TextStyle {
                        font: asset_server.load(TITLE_FONT),
                        font_size: 75.0,
                        color: Color::WHITE,
                    },
                )
                .with_text_alignment(TextAlignment::Center)
                .with_style(Style {
                    margin: UiRect::all(Val::Auto),
                    ..default()
                }),
            );
        });

    // intro text
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Auto),
                position_type: PositionType::Absolute,
                position: UiRect {
                    top: Val::Percent(15.0),
                    ..default()
                },
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .insert(MenuComponent)
        .with_children(|parent| {
            parent.spawn(
                TextBundle::from_section(
                    INTRO_TEXT,
                    TextStyle {
                        font: asset_server.load(MAIN_FONT),
                        font_size: 31.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                )
                .with_text_alignment(TextAlignment::Center)
                .with_style(Style {
                    margin: UiRect::all(Val::Auto),
                    max_size: Size {
                        width: Val::Px(WINDOW_WIDTH * 0.8),
                        ..default()
                    },
                    ..default()
                }),
            );
        });

    // start button
    commands
        .spawn(NodeBundle {
            style: Style {
                // center button
                size: Size::new(Val::Percent(100.0), Val::Auto),
                position_type: PositionType::Absolute,
                position: UiRect {
                    bottom: Val::Px(10.0),
                    ..default()
                },
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .insert(MenuComponent)
        .with_children(|parent| {
            parent
                .spawn(ButtonBundle {
                    style: Style {
                        size: Size::new(Val::Auto, Val::Auto),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    background_color: NORMAL_BUTTON.into(),
                    ..default()
                })
                .insert(StartButton)
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "let's bounce",
                        TextStyle {
                            font: asset_server.load(MONO_FONT),
                            font_size: 40.0,
                            color: NORMAL_BUTTON_TEXT_COLOR,
                        },
                    ));
                });
        });
}

type InteractedStartButtonTuple = (Changed<Interaction>, With<StartButton>);

/// Handles interactions with the start button.
fn start_button_system(
    mut next_state: ResMut<NextState<GameState>>,
    interaction_query: Query<&Interaction, InteractedStartButtonTuple>,
) {
    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Clicked {
            next_state.set(GameState::GameLoading);
        }
    }
}

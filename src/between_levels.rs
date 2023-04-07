use crate::*;

pub struct BetweenLevelsPlugin;

impl Plugin for BetweenLevelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(between_levels_setup.in_schedule(OnEnter(GameState::BetweenLevels)))
            .add_system(
                despawn_components_system::<BetweenLevelsComponent>
                    .in_schedule(OnExit(GameState::BetweenLevels)),
            )
            .add_system(next_level_button_system.run_if(in_state(GameState::BetweenLevels)))
            .add_system(restart_level_button_system.run_if(in_state(GameState::BetweenLevels)));
    }
}

#[derive(Component)]
struct BetweenLevelsComponent;

#[derive(Component)]
struct NextLevelButton;

#[derive(Component)]
struct RestartLevelButton;

/// Sets up the between levels screen
fn between_levels_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    score: Res<Score>,
    level_settings: Res<LevelSettings>,
) {
    // score text
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(50.0)),
                position_type: PositionType::Absolute,
                position: UiRect {
                    top: Val::Px(0.0),
                    ..default()
                },
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .insert(BetweenLevelsComponent)
        .with_children(|parent| {
            parent.spawn(
                TextBundle::from_section(
                    format!(
                        "You got {} points on level {}! wow",
                        score.0, level_settings.id
                    ),
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
            );
        });

    if score.0 >= level_settings.min_score {
        // next level button
        commands
            .spawn(NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        bottom: Val::Px(0.0),
                        ..default()
                    },
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            })
            .insert(BetweenLevelsComponent)
            .with_children(|parent| {
                parent
                    .spawn(ButtonBundle {
                        style: Style {
                            size: Size::new(Val::Auto, Val::Px(65.0)),
                            padding: UiRect {
                                left: Val::Px(10.0),
                                right: Val::Px(10.0),
                                ..default()
                            },
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: NORMAL_BUTTON.into(),
                        ..default()
                    })
                    .insert(NextLevelButton)
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Start Next Level",
                            TextStyle {
                                font: asset_server.load(MAIN_FONT),
                                font_size: 40.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                            },
                        ));
                    });
            });
    } else {
        // restart level button
        commands
            .spawn(NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Auto),
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        bottom: Val::Percent(50.0),
                        ..default()
                    },
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ..default()
            })
            .insert(BetweenLevelsComponent)
            .with_children(|parent| {
                parent.spawn(
                    TextBundle::from_section(
                        format!(
                            "You need a score of {} to move on to the next level",
                            level_settings.min_score,
                        ),
                        TextStyle {
                            font: asset_server.load(MAIN_FONT),
                            font_size: 40.0,
                            color: Color::WHITE,
                        },
                    )
                    .with_text_alignment(TextAlignment::Center)
                    .with_style(Style {
                        margin: UiRect::all(Val::Auto),
                        ..default()
                    }),
                );

                parent
                    .spawn(ButtonBundle {
                        style: Style {
                            size: Size::new(Val::Auto, Val::Px(65.0)),
                            margin: UiRect {
                                top: Val::Px(10.0),
                                ..default()
                            },
                            padding: UiRect {
                                left: Val::Px(10.0),
                                right: Val::Px(10.0),
                                ..default()
                            },
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: NORMAL_BUTTON.into(),
                        ..default()
                    })
                    .insert(RestartLevelButton)
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Restart Level",
                            TextStyle {
                                font: asset_server.load(MAIN_FONT),
                                font_size: 40.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                            },
                        ));
                    });
            });
    }
}

type InteractedNextLevelButtonTuple = (Changed<Interaction>, With<NextLevelButton>);

/// Handles interactions with the next level button.
fn next_level_button_system(
    mut level_settings: ResMut<LevelSettings>,
    mut next_state: ResMut<NextState<GameState>>,
    interaction_query: Query<&Interaction, InteractedNextLevelButtonTuple>,
) {
    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Clicked {
            *level_settings = level_settings.next_level();
            next_state.set(GameState::Game);
        }
    }
}

type InteractedRestartLevelButtonTuple = (Changed<Interaction>, With<RestartLevelButton>);

/// Handles interactions with the restart level button.
fn restart_level_button_system(
    mut next_state: ResMut<NextState<GameState>>,
    interaction_query: Query<&Interaction, InteractedRestartLevelButtonTuple>,
) {
    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Clicked {
            next_state.set(GameState::Game);
        }
    }
}

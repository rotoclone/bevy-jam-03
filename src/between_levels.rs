use crate::*;

pub struct BetweenLevelsPlugin;

impl Plugin for BetweenLevelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            unlock_sides
                .before(between_levels_setup)
                .in_schedule(OnEnter(GameState::BetweenLevels)),
        )
        .add_system(between_levels_setup.in_schedule(OnEnter(GameState::BetweenLevels)))
        .add_system(
            despawn_components_system::<BetweenLevelsComponent>
                .in_schedule(OnExit(GameState::BetweenLevels)),
        )
        .add_system(side_selection_buttons_system.run_if(in_state(GameState::BetweenLevels)))
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

#[derive(Component)]
struct SideSelectionButton {
    side_id: SideId,
    side_type: SideType,
}

#[derive(Component)]
struct SideSelectionButtonHighlight(SideSelectionButton);

#[derive(Component)]
struct SideDescription(SideId);

/// Unlocks sides based on the completed level
fn unlock_sides(
    score: Res<Score>,
    level_settings: Res<LevelSettings>,
    mut unlocked_sides: ResMut<UnlockedSides>,
) {
    if score.0 >= level_settings.min_score {
        for unlocked_side in &level_settings.sides_to_unlock {
            unlocked_sides.0.insert(*unlocked_side);
        }
    }
}

/// Sets up the between levels screen
fn between_levels_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    score: Res<Score>,
    level_settings: Res<LevelSettings>,
    unlocked_sides: Res<UnlockedSides>,
    configured_sides: Res<ConfiguredSides>,
) {
    // score text
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Auto),
                position_type: PositionType::Absolute,
                position: UiRect {
                    top: Val::Px(5.0),
                    ..default()
                },
                flex_direction: FlexDirection::Column,
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
                        font_size: 45.0,
                        color: Color::WHITE,
                    },
                )
                .with_text_alignment(TextAlignment::Center)
                .with_style(Style {
                    margin: UiRect::all(Val::Auto),
                    ..default()
                }),
            );

            if score.0 >= level_settings.min_score {
                // unlocked sides text
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            size: Size::new(Val::Percent(100.0), Val::Auto),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ..default()
                    })
                    .insert(BetweenLevelsComponent)
                    .with_children(|parent| {
                        for unlocked_side in &level_settings.sides_to_unlock {
                            parent.spawn(
                                TextBundle::from_section(
                                    format!("You unlocked a new side: {}", unlocked_side.name()),
                                    TextStyle {
                                        font: asset_server.load(MAIN_FONT),
                                        font_size: 35.0,
                                        color: Color::WHITE,
                                    },
                                )
                                .with_text_alignment(TextAlignment::Center)
                                .with_style(Style {
                                    margin: UiRect::all(Val::Auto),
                                    ..default()
                                }),
                            );
                        }
                    });
            }
        });

    // side customization
    for side in 0..PLAYER_SHAPE_SIDES {
        spawn_side_customization_ui(
            SideId(side),
            &mut commands,
            &asset_server,
            &unlocked_sides,
            &configured_sides,
        );
    }

    if score.0 >= level_settings.min_score {
        // next level button
        commands
            .spawn(NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Auto),
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        bottom: Val::Px(5.0),
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
                        bottom: Val::Px(5.0),
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

/// Spawns UI for customizing a side
fn spawn_side_customization_ui(
    side_id: SideId,
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    unlocked_sides: &Res<UnlockedSides>,
    configured_sides: &Res<ConfiguredSides>,
) {
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Auto),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                align_self: AlignSelf::Center,
                ..default()
            },
            ..default()
        })
        .insert(BetweenLevelsComponent)
        .with_children(|parent| {
            // side name
            parent.spawn(
                TextBundle::from_section(
                    format!("Side {}", side_id.0 + 1),
                    TextStyle {
                        font: asset_server.load(MAIN_FONT),
                        font_size: 25.0,
                        color: Color::WHITE,
                    },
                )
                .with_text_alignment(TextAlignment::Center)
                .with_style(Style {
                    margin: UiRect::all(Val::Auto),
                    ..default()
                }),
            );

            // side type selection buttons
            for side_type in &unlocked_sides.0 {
                parent
                    .spawn(ButtonBundle {
                        style: Style {
                            size: Size::new(Val::Auto, Val::Auto),
                            margin: UiRect {
                                top: Val::Px(10.0),
                                ..default()
                            },
                            padding: UiRect {
                                left: Val::Px(10.0),
                                right: Val::Px(10.0),
                                top: Val::Px(5.0),
                                bottom: Val::Px(5.0),
                            },
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: NORMAL_BUTTON.into(),
                        ..default()
                    })
                    .insert(SideSelectionButton {
                        side_id,
                        side_type: *side_type,
                    })
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            side_type.name(),
                            TextStyle {
                                font: asset_server.load(MAIN_FONT),
                                font_size: 20.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                            },
                        ));

                        let visibility = if configured_sides
                            .0
                            .get(&side_id)
                            .expect("side should be configured")
                            == side_type
                        {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        };
                        parent
                            .spawn(NodeBundle {
                                background_color: BackgroundColor(Color::Rgba {
                                    red: 1.0,
                                    green: 1.0,
                                    blue: 1.0,
                                    alpha: 0.1,
                                }),
                                style: Style {
                                    //TODO this is wrong
                                    position_type: PositionType::Absolute,
                                    padding: UiRect {
                                        left: Val::Percent(50.0),
                                        right: Val::Percent(50.0),
                                        top: Val::Percent(0.0),
                                        bottom: Val::Percent(33.33),
                                    },
                                    margin: UiRect::all(Val::Percent(100.0)),
                                    ..default()
                                },
                                visibility,
                                ..default()
                            })
                            .insert(SideSelectionButtonHighlight(SideSelectionButton {
                                side_id,
                                side_type: *side_type,
                            }));
                    });
            }

            // selected side type description
            parent
                .spawn(
                    TextBundle::from_section(
                        configured_sides
                            .0
                            .get(&side_id)
                            .expect("side should be configured")
                            .description(),
                        TextStyle {
                            font: asset_server.load(MAIN_FONT),
                            font_size: 25.0,
                            color: Color::WHITE,
                        },
                    )
                    .with_text_alignment(TextAlignment::Center)
                    .with_style(Style {
                        margin: UiRect {
                            top: Val::Px(5.0),
                            ..default()
                        },
                        max_size: Size {
                            width: Val::Px(300.0),
                            ..default()
                        },
                        ..default()
                    }),
                )
                .insert(SideDescription(side_id));
        });
}

/// Handles interactions with the side selection buttons.
fn side_selection_buttons_system(
    button_query: Query<(&Interaction, &SideSelectionButton), Changed<Interaction>>,
    mut button_highlight_query: Query<(&mut Visibility, &SideSelectionButtonHighlight)>,
    mut side_description_text_query: Query<(&mut Text, &SideDescription)>,
    mut configured_sides: ResMut<ConfiguredSides>,
) {
    for (interaction, button) in button_query.iter() {
        if *interaction == Interaction::Clicked {
            configured_sides.0.insert(button.side_id, button.side_type);
            for (mut visibility, highlight) in button_highlight_query.iter_mut() {
                if highlight.0.side_id == button.side_id {
                    if highlight.0.side_type == button.side_type {
                        *visibility = Visibility::Inherited;
                    } else {
                        *visibility = Visibility::Hidden;
                    }
                }
            }

            for (mut text, description) in side_description_text_query.iter_mut() {
                if description.0 == button.side_id {
                    text.sections[0].value = button.side_type.description().to_string();
                }
            }
        }
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

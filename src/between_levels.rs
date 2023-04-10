use crate::*;

const PLAYER_PREVIEW_TRANSFORM: Transform =
    Transform::from_translation(Vec3::new(0.0, -180.0, 0.0));

const MENU_MUSIC_VOLUME: f32 = 0.25;

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
        .add_system(start_backround_music.in_schedule(OnEnter(GameState::BetweenLevels)))
        .add_system(stop_background_music.in_schedule(OnExit(GameState::BetweenLevels)))
        .add_system(side_selection_buttons_system.run_if(in_state(GameState::BetweenLevels)))
        .add_system(next_level_button_system.run_if(in_state(GameState::BetweenLevels)))
        .add_system(restart_level_button_system.run_if(in_state(GameState::BetweenLevels)));
    }
}

#[derive(Resource)]
struct MenuMusicController(Handle<AudioSink>);

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
struct SideSelectionButtonText(SideSelectionButton);

#[derive(Component)]
struct SideDescription(SideId);

#[derive(Component)]
struct PlayerPreview;

/// Unlocks sides based on the completed level
fn unlock_sides(
    score: Res<Score>,
    level_settings: Res<LevelSettings>,
    mut unlocked_sides: ResMut<UnlockedSides>,
) {
    if score.0 >= level_settings.min_score {
        for unlocked_side in &level_settings.sides_to_unlock {
            unlocked_sides.0.push(*unlocked_side);
        }
    }
}

/// Sets up the between levels screen
#[allow(clippy::too_many_arguments)]
fn between_levels_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    image_assets: Res<ImageAssets>,
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
                    format!("you got {} points on level {}", score.0, level_settings.id),
                    TextStyle {
                        font: asset_server.load(MONO_FONT),
                        font_size: 45.0,
                        color: Color::WHITE,
                    },
                )
                .with_text_alignment(TextAlignment::Center)
                .with_style(Style {
                    margin: UiRect {
                        bottom: Val::Px(10.0),
                        ..default()
                    },
                    ..default()
                }),
            );

            if score.0 >= level_settings.min_score || level_settings.sides_to_unlock.is_empty() {
                // unlocked sides text
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            size: Size::new(Val::Percent(100.0), Val::Auto),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ..default()
                    })
                    .insert(BetweenLevelsComponent)
                    .with_children(|parent| {
                        if level_settings.sides_to_unlock.is_empty() {
                            parent.spawn(
                                TextBundle::from_section(
                                    "all sides have been unlocked",
                                    TextStyle {
                                        font: asset_server.load(MONO_FONT),
                                        font_size: 25.0,
                                        color: Color::rgb(0.8, 0.8, 0.8),
                                    },
                                )
                                .with_text_alignment(TextAlignment::Center)
                                .with_style(Style {
                                    margin: UiRect::all(Val::Auto),
                                    ..default()
                                }),
                            );
                        } else {
                            for unlocked_side in &level_settings.sides_to_unlock {
                                parent.spawn(
                                    TextBundle::from_section(
                                        format!(
                                            "new side unlocked: {}",
                                            unlocked_side.name().to_ascii_lowercase()
                                        ),
                                        TextStyle {
                                            font: asset_server.load(MONO_FONT),
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
                        }
                    });
            }

            // side customization
            parent
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Auto),
                        margin: UiRect {
                            top: Val::Px(25.0),
                            ..default()
                        },
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ..default()
                })
                .insert(BetweenLevelsComponent)
                .with_children(|parent| {
                    for side in 0..PLAYER_SHAPE_SIDES {
                        spawn_side_customization_ui(
                            SideId(side),
                            parent,
                            &asset_server,
                            &unlocked_sides,
                            &configured_sides,
                        );
                    }
                });
        });

    // player preview
    spawn_player_shape(
        &mut commands,
        &mut meshes,
        &mut materials,
        &image_assets,
        &configured_sides,
        PLAYER_PREVIEW_TRANSFORM,
    )
    .insert(BetweenLevelsComponent)
    .insert(PlayerPreview);

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
                            size: Size::new(Val::Auto, Val::Auto),
                            padding: UiRect::all(Val::Px(10.0)),
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
                            "start next level",
                            TextStyle {
                                font: asset_server.load(MONO_FONT),
                                font_size: 40.0,
                                color: NORMAL_BUTTON_TEXT_COLOR,
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
                            "you need a score of {} to move on to the next level",
                            level_settings.min_score,
                        ),
                        TextStyle {
                            font: asset_server.load(MONO_FONT),
                            font_size: 30.0,
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
                            size: Size::new(Val::Auto, Val::Auto),
                            margin: UiRect {
                                top: Val::Px(10.0),
                                ..default()
                            },
                            padding: UiRect::all(Val::Px(10.0)),
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
                            "restart level",
                            TextStyle {
                                font: asset_server.load(MONO_FONT),
                                font_size: 40.0,
                                color: NORMAL_BUTTON_TEXT_COLOR,
                            },
                        ));
                    });
            });
    }
}

/// Spawns UI for customizing a side
fn spawn_side_customization_ui(
    side_id: SideId,
    root_parent: &mut ChildBuilder,
    asset_server: &Res<AssetServer>,
    unlocked_sides: &Res<UnlockedSides>,
    configured_sides: &Res<ConfiguredSides>,
) {
    root_parent
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Auto),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                align_self: AlignSelf::Start,
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
                        font_size: 26.0,
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
                let selected = configured_sides.get(&side_id) == *side_type;
                let enabled = can_side_be_selected(side_type, &side_id, configured_sides);

                let button_color = if enabled {
                    NORMAL_BUTTON
                } else {
                    DISABLED_BUTTON
                };

                let button_text_color = if enabled {
                    NORMAL_BUTTON_TEXT_COLOR
                } else {
                    DISABLED_BUTTON_TEXT_COLOR
                };

                let mut button = parent.spawn(ButtonBundle {
                    style: Style {
                        size: Size::new(Val::Auto, Val::Px(30.0)),
                        margin: UiRect {
                            top: Val::Px(5.0),
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
                    background_color: button_color.into(),
                    ..default()
                });

                button.insert(SideSelectionButton {
                    side_id,
                    side_type: *side_type,
                });

                if !enabled {
                    button.insert(DisabledButton);
                }

                button.with_children(|parent| {
                    parent
                        .spawn(TextBundle::from_section(
                            side_type.name(),
                            TextStyle {
                                font: asset_server.load(MAIN_FONT),
                                font_size: 21.0,
                                color: button_text_color,
                            },
                        ))
                        .insert(SideSelectionButtonText(SideSelectionButton {
                            side_id,
                            side_type: *side_type,
                        }));

                    let visibility = if selected {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                    parent
                        .spawn(NodeBundle {
                            background_color: BackgroundColor(Color::Rgba {
                                red: 0.0,
                                green: 0.8,
                                blue: 1.0,
                                alpha: 0.1,
                            }),
                            style: Style {
                                position_type: PositionType::Absolute,
                                padding: UiRect {
                                    left: Val::Percent(50.0),
                                    right: Val::Percent(50.0),
                                    top: Val::Percent(0.0),
                                    bottom: Val::Px(30.0),
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
                        configured_sides.get(&side_id).description(),
                        TextStyle {
                            font: asset_server.load(MAIN_FONT),
                            font_size: 26.0,
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

/// Determines if the provided side type can be chosen for the provided side ID, given the other configured sides.
fn can_side_be_selected(
    side_type: &SideType,
    side_id: &SideId,
    configured_sides: &ConfiguredSides,
) -> bool {
    if !side_type.multiple_allowed() {
        for i in 0..PLAYER_SHAPE_SIDES {
            if i == side_id.0 {
                continue;
            }

            if configured_sides.get(&SideId(i)) == *side_type {
                // another side is already configured to use this side type
                return false;
            }
        }
    }

    true
}

type InteractedSideSelectionButtonTuple = (Changed<Interaction>, Without<DisabledButton>);

/// Handles interactions with the side selection buttons.
#[allow(clippy::too_many_arguments)]
fn side_selection_buttons_system(
    mut commands: Commands,
    interacted_button_query: Query<
        (&Interaction, &SideSelectionButton),
        InteractedSideSelectionButtonTuple,
    >,
    mut all_buttons_query: Query<(Entity, &SideSelectionButton, &mut BackgroundColor)>,
    mut all_button_text_query: Query<(&mut Text, &SideSelectionButtonText)>,
    mut button_highlight_query: Query<(&mut Visibility, &SideSelectionButtonHighlight)>,
    mut side_description_text_query: Query<
        (&mut Text, &SideDescription),
        Without<SideSelectionButtonText>,
    >,
    player_preview_query: Query<Entity, With<PlayerPreview>>,
    mut configured_sides: ResMut<ConfiguredSides>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    image_assets: Res<ImageAssets>,
) {
    let mut should_spawn_player_preview = false;
    for (interaction, interacted_button) in interacted_button_query.iter() {
        if *interaction == Interaction::Clicked {
            configured_sides
                .0
                .insert(interacted_button.side_id, interacted_button.side_type);

            // update highlighted button
            for (mut visibility, highlight) in button_highlight_query.iter_mut() {
                if highlight.0.side_id == interacted_button.side_id {
                    if highlight.0.side_type == interacted_button.side_type {
                        *visibility = Visibility::Inherited;
                    } else {
                        *visibility = Visibility::Hidden;
                    }
                }
            }

            // update side description text
            for (mut text, description) in side_description_text_query.iter_mut() {
                if description.0 == interacted_button.side_id {
                    text.sections[0].value = interacted_button.side_type.description().to_string();
                }
            }

            // update which buttons are disabled
            for (button_entity, button, mut background_color) in all_buttons_query.iter_mut() {
                if can_side_be_selected(&button.side_type, &button.side_id, &configured_sides) {
                    commands.entity(button_entity).remove::<DisabledButton>();
                    *background_color = NORMAL_BUTTON.into();
                } else {
                    commands.entity(button_entity).insert(DisabledButton);
                    *background_color = DISABLED_BUTTON.into();
                }
            }

            // update button text colors to match their disabledness
            for (mut text, button_text) in all_button_text_query.iter_mut() {
                if can_side_be_selected(
                    &button_text.0.side_type,
                    &button_text.0.side_id,
                    &configured_sides,
                ) {
                    text.sections[0].style.color = NORMAL_BUTTON_TEXT_COLOR;
                } else {
                    text.sections[0].style.color = DISABLED_BUTTON_TEXT_COLOR;
                }
            }

            // update player preview
            for player_preview_entity in player_preview_query.iter() {
                commands.entity(player_preview_entity).despawn_recursive();
            }

            should_spawn_player_preview = true;
        }

        if should_spawn_player_preview {
            spawn_player_shape(
                &mut commands,
                &mut meshes,
                &mut materials,
                &image_assets,
                &configured_sides,
                PLAYER_PREVIEW_TRANSFORM,
            )
            .insert(BetweenLevelsComponent)
            .insert(PlayerPreview);
        }
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
        audio_assets.menu_music.clone(),
        PlaybackSettings::LOOP.with_volume(MENU_MUSIC_VOLUME * MASTER_VOLUME),
    ));

    commands.insert_resource(MenuMusicController(handle));
}

/// Stops playing the background music
fn stop_background_music(
    music_controller: Res<MenuMusicController>,
    audio_sinks: Res<Assets<AudioSink>>,
) {
    if let Some(sink) = audio_sinks.get(&music_controller.0) {
        sink.stop();
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

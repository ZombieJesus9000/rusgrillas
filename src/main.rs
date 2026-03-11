use bevy::prelude::*;
use rand::prelude::*;

/// Logical size of the game world in pixels.
const WORLD_WIDTH: f32 = 800.0;
const WORLD_HEIGHT: f32 = 450.0;

const BUILDING_MIN_WIDTH: f32 = 40.0;
const BUILDING_MAX_WIDTH: f32 = 110.0;
const BUILDING_MIN_HEIGHT: f32 = 80.0;
const BUILDING_MAX_HEIGHT: f32 = 260.0;

const GORILLA_SIZE: Vec2 = Vec2::new(32.0, 48.0);
const BANANA_SIZE: Vec2 = Vec2::new(8.0, 8.0);

/// Gravity in "meters/sec²" game units, default 9.8 similar to GORILLA.BAS.
const DEFAULT_GRAVITY: f32 = 9.8;

/// How strong wind can be, in horizontal units/sec².
const MAX_WIND: f32 = 20.0;

/// How fast the day/night cycle runs (seconds for a full loop).
const DAY_NIGHT_PERIOD: f32 = 40.0;

/// Target frames per second for fixed-step physics.
const PHYSICS_HZ: f32 = 60.0;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, States)]
enum GameState {
    #[default]
    Playing,
}

#[derive(Component)]
struct Building;

#[derive(Component)]
struct Gorilla {
    player: usize, // 1 or 2
}

#[derive(Component)]
struct Banana;

#[derive(Component)]
struct Explosion {
    timer: Timer,
}

#[derive(Component)]
struct Sun;

#[derive(Component)]
struct WindArrow;

#[derive(Component)]
struct UiText;

#[derive(Resource)]
struct PhysicsSettings {
    gravity: f32,
    wind: f32,
}

#[derive(Resource)]
struct TurnState {
    current_player: usize,
    angle_deg: f32,
    power: f32,
    waiting_for_shot_to_finish: bool,
    scores: [u32; 2],
}

#[derive(Resource, Default)]
struct FixedTimeAccumulator {
    acc: f32,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.05, 0.05, 0.12)))
        .insert_resource(PhysicsSettings {
            gravity: DEFAULT_GRAVITY,
            wind: 0.0,
        })
        .insert_resource(TurnState {
            current_player: 1,
            angle_deg: 45.0,
            power: 40.0,
            waiting_for_shot_to_finish: false,
            scores: [0, 0],
        })
        .insert_resource(FixedTimeAccumulator::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rusgrillas - Rust Gorillas Tribute".to_string(),
                resolution: (WORLD_WIDTH, WORLD_HEIGHT).into(),
                resizable: false,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_state::<GameState>()
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(GameState::Playing), (generate_city, spawn_gorillas_and_ui))
        .add_systems(
            Update,
            (
                day_night_cycle,
                keyboard_input_system,
                update_ui_text,
                step_physics,
                update_explosions,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(WORLD_WIDTH / 2.0, WORLD_HEIGHT / 2.0, 999.0),
        ..Default::default()
    });
}

fn generate_city(
    mut commands: Commands,
    mut physics: ResMut<PhysicsSettings>,
    mut clear_color: ResMut<ClearColor>,
) {
    let mut rng = rand::thread_rng();

    // Random day/night starting color
    let t: f32 = rng.gen::<f32>();
    let (r, g, b) = day_night_color(t);
    clear_color.0 = Color::rgb(r, g, b);

    // Random wind similar in spirit to original (+/- range, with possible bias)
    let base = rng.gen_range(-5.0..=5.0);
    let extra = if rng.gen_bool(1.0 / 3.0) {
        if base > 0.0 {
            rng.gen_range(0.0..=MAX_WIND)
        } else {
            -rng.gen_range(0.0..=MAX_WIND)
        }
    } else {
        0.0
    };
    physics.wind = (base + extra).clamp(-MAX_WIND, MAX_WIND);

    // Generate buildings along X axis
    let mut x = 0.0_f32;
    let mut buildings_for_gorillas: Vec<(f32, f32, f32)> = Vec::new(); // (x_center, top_y, width)

    while x < WORLD_WIDTH {
        let width = rng.gen_range(BUILDING_MIN_WIDTH..=BUILDING_MAX_WIDTH);
        if x + width > WORLD_WIDTH {
            break;
        }
        let height = rng.gen_range(BUILDING_MIN_HEIGHT..=BUILDING_MAX_HEIGHT);
        let center_x = x + width * 0.5;
        let center_y = height * 0.5;

        // Save for gorilla placement
        let top_y = height;
        buildings_for_gorillas.push((center_x, top_y, width));

        // Body of building
        let building_color = if rng.gen_bool(0.5) {
            Color::rgb(0.1, 0.1, 0.3)
        } else {
            Color::rgb(0.15, 0.15, 0.4)
        };

        commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    color: building_color,
                    custom_size: Some(Vec2::new(width - 2.0, height)),
                    ..Default::default()
                },
                transform: Transform::from_xyz(center_x, center_y, 0.0),
                ..Default::default()
            })
            .insert(Building);

        // Simple "windows" – small bright rectangles
        let window_color_on = Color::rgb(1.0, 0.9, 0.6);
        let window_color_off = Color::rgb(0.05, 0.05, 0.1);

        let win_w = 6.0;
        let win_h = 8.0;
        let x_start = x + 6.0;
        let x_end = x + width - 6.0;
        let mut wx = x_start;
        while wx < x_end {
            let mut wy = 10.0;
            while wy < height - 10.0 {
                let on = rng.gen_bool(0.5);
                commands.spawn(SpriteBundle {
                    sprite: Sprite {
                        color: if on { window_color_on } else { window_color_off },
                        custom_size: Some(Vec2::new(win_w, win_h)),
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(wx, wy, 1.0),
                    ..Default::default()
                });
                wy += 18.0;
            }
            wx += 18.0;
        }

        x += width + 4.0;
    }

    // Draw sun
    let sun_x = WORLD_WIDTH / 2.0;
    let sun_y = WORLD_HEIGHT - 60.0;
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 0.9, 0.2),
                custom_size: Some(Vec2::new(40.0, 40.0)),
                ..Default::default()
            },
            transform: Transform::from_xyz(sun_x, sun_y, 5.0),
            ..Default::default()
        })
        .insert(Sun);

    // Draw wind arrow at bottom
    if physics.wind.abs() > 0.01 {
        let len = physics.wind / MAX_WIND * (WORLD_WIDTH * 0.25);
        // arrow body
        commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.7, 0.9, 1.0),
                    custom_size: Some(Vec2::new(len.abs(), 4.0)),
                    ..Default::default()
                },
                transform: Transform::from_xyz(
                    WORLD_WIDTH / 2.0 + len / 2.0,
                    16.0,
                    5.0,
                ),
                ..Default::default()
            })
            .insert(WindArrow);

        // arrow head
        let head_size = Vec2::new(10.0, 10.0);
        let head_x = if len > 0.0 {
            WORLD_WIDTH / 2.0 + len
        } else {
            WORLD_WIDTH / 2.0 + len
        };
        commands
            .spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.7, 0.9, 1.0),
                    custom_size: Some(head_size),
                    ..Default::default()
                },
                transform: Transform::from_xyz(head_x, 16.0, 5.0),
                ..Default::default()
            })
            .insert(WindArrow);
    }
}

fn spawn_gorillas_and_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Player 1 on left third, Player 2 on right third, slightly above buildings.
    let gorilla_color_1 = Color::rgb(0.7, 0.6, 0.3);
    let gorilla_color_2 = Color::rgb(0.4, 0.7, 0.5);

    let y_base = BUILDING_MAX_HEIGHT + GORILLA_SIZE.y * 0.5 + 12.0;

    let g1_x = WORLD_WIDTH * 0.15;
    let g2_x = WORLD_WIDTH * 0.85;

    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: gorilla_color_1,
                custom_size: Some(GORILLA_SIZE),
                ..Default::default()
            },
            transform: Transform::from_xyz(g1_x, y_base, 10.0),
            ..Default::default()
        })
        .insert(Gorilla { player: 1 });

    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: gorilla_color_2,
                custom_size: Some(GORILLA_SIZE),
                ..Default::default()
            },
            transform: Transform::from_xyz(g2_x, y_base, 10.0),
            ..Default::default()
        })
        .insert(Gorilla { player: 2 });

    // UI text overlay (angle, power, player, score, controls)
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn(TextBundle {
            text: Text::from_section(
                "",
                TextStyle {
                    font,
                    font_size: 18.0,
                    color: Color::WHITE,
                },
            )
            .with_alignment(TextAlignment::Left),
            style: Style {
                position_type: PositionType::Absolute,
                position: UiRect {
                    left: Val::Px(10.0),
                    top: Val::Px(10.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(UiText);
}

fn day_night_cycle(time: Res<Time>, mut clear_color: ResMut<ClearColor>) {
    let t = (time.elapsed_seconds() / DAY_NIGHT_PERIOD).fract();
    let (r, g, b) = day_night_color(t);
    clear_color.0 = Color::rgb(r, g, b);
}

fn day_night_color(t: f32) -> (f32, f32, f32) {
    // Simple gradient: night (dark blue) -> sunrise -> day -> sunset -> night
    let t = t.clamp(0.0, 1.0);
    if t < 0.25 {
        let k = t / 0.25;
        // night to sunrise
        lerp_color(
            (0.02, 0.02, 0.08),
            (0.4, 0.2, 0.3),
            k,
        )
    } else if t < 0.5 {
        let k = (t - 0.25) / 0.25;
        // sunrise to day
        lerp_color(
            (0.4, 0.2, 0.3),
            (0.35, 0.55, 0.9),
            k,
        )
    } else if t < 0.75 {
        let k = (t - 0.5) / 0.25;
        // day to sunset
        lerp_color(
            (0.35, 0.55, 0.9),
            (0.4, 0.25, 0.25),
            k,
        )
    } else {
        let k = (t - 0.75) / 0.25;
        // sunset to night
        lerp_color(
            (0.4, 0.25, 0.25),
            (0.02, 0.02, 0.08),
            k,
        )
    }
}

fn lerp_color(a: (f32, f32, f32), b: (f32, f32, f32), k: f32) -> (f32, f32, f32) {
    (
        a.0 + (b.0 - a.0) * k,
        a.1 + (b.1 - a.1) * k,
        a.2 + (b.2 - a.2) * k,
    )
}

fn keyboard_input_system(
    kb: Res<ButtonInput<KeyCode>>,
    mut turn: ResMut<TurnState>,
    gorillas: Query<(&Gorilla, &Transform)>,
    physics: Res<PhysicsSettings>,
    mut commands: Commands,
) {
    if turn.waiting_for_shot_to_finish {
        return;
    }

    // Simple controls:
    // Left/Right: adjust angle
    // Up/Down: adjust power
    // Space/Return: fire
    if kb.pressed(KeyCode::ArrowLeft) {
        turn.angle_deg = (turn.angle_deg - 60.0 * 1.0 / PHYSICS_HZ).clamp(5.0, 175.0);
    }
    if kb.pressed(KeyCode::ArrowRight) {
        turn.angle_deg = (turn.angle_deg + 60.0 * 1.0 / PHYSICS_HZ).clamp(5.0, 175.0);
    }
    if kb.pressed(KeyCode::ArrowUp) {
        turn.power = (turn.power + 30.0 * 1.0 / PHYSICS_HZ).clamp(5.0, 100.0);
    }
    if kb.pressed(KeyCode::ArrowDown) {
        turn.power = (turn.power - 30.0 * 1.0 / PHYSICS_HZ).clamp(5.0, 100.0);
    }

    if kb.just_pressed(KeyCode::Space) || kb.just_pressed(KeyCode::Enter) {
        // Spawn banana from active gorilla
        let player = turn.current_player;
        if let Some((_, transform)) = gorillas
            .iter()
            .find(|(g, _)| g.player == player)
        {
            let origin = transform.translation + Vec3::new(
                if player == 1 { GORILLA_SIZE.x * 0.5 } else { -GORILLA_SIZE.x * 0.5 },
                GORILLA_SIZE.y * 0.4,
                0.0,
            );

            let angle_rad = if player == 1 {
                turn.angle_deg.to_radians()
            } else {
                (180.0 - turn.angle_deg).to_radians()
            };

            let vx = angle_rad.cos() * turn.power;
            let vy = angle_rad.sin() * turn.power;

            let mut banana = commands.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::YELLOW,
                    custom_size: Some(BANANA_SIZE),
                    ..Default::default()
                },
                transform: Transform::from_translation(origin),
                ..Default::default()
            });
            banana
                .insert(Banana)
                .insert(Velocity(Vec2::new(vx, vy)))
                .insert(Owner(player));

            turn.waiting_for_shot_to_finish = true;

            // "Throw" sound: simple oscillation using Bevy's audio could be added later.
            let _ = &physics; // silence unused for now if we don't use it yet
        }
    }
}

#[derive(Component)]
struct Owner(usize);

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

fn step_physics(
    time: Res<Time>,
    mut acc: ResMut<FixedTimeAccumulator>,
    mut bananas: Query<(&mut Transform, &mut Velocity, &Owner), With<Banana>>,
    buildings: Query<(&Transform, &Sprite), With<Building>>,
    gorillas: Query<(&Gorilla, &Transform)>,
    mut commands: Commands,
    mut turn: ResMut<TurnState>,
    physics: Res<PhysicsSettings>,
) {
    let dt = time.delta_seconds();
    acc.acc += dt;
    let fixed_dt = 1.0 / PHYSICS_HZ;

    while acc.acc >= fixed_dt {
        acc.acc -= fixed_dt;

        let mut to_despawn: Vec<Entity> = Vec::new();
        let mut hit_player: Option<usize> = None;

        for (entity, (mut transform, mut vel, owner)) in bananas
            .iter_mut()
            .enumerate()
            .map(|(i, (t, v, o))| (Entity::from_raw(i as u32), (t, v, o)))
        {
            // Apply gravity and wind
            vel.y -= physics.gravity * fixed_dt;
            vel.x += physics.wind * fixed_dt * 0.1;

            // Integrate position
            transform.translation.x += vel.x * fixed_dt;
            transform.translation.y += vel.y * fixed_dt;

            let pos = transform.translation;

            // Off-screen?
            if pos.x < -20.0 || pos.x > WORLD_WIDTH + 20.0 || pos.y < 0.0 {
                to_despawn.push(entity);
                continue;
            }

            // Collision with buildings
            for (b_transform, sprite) in buildings.iter() {
                if let Some(size) = sprite.custom_size {
                    if aabb_overlap(
                        pos.truncate(),
                        BANANA_SIZE,
                        b_transform.translation.truncate(),
                        size,
                    ) {
                        spawn_explosion(&mut commands, pos);
                        to_despawn.push(entity);
                        break;
                    }
                }
            }

            // Collision with gorillas
            for (g, g_transform) in gorillas.iter() {
                if aabb_overlap(
                    pos.truncate(),
                    BANANA_SIZE,
                    g_transform.translation.truncate(),
                    GORILLA_SIZE,
                ) {
                    spawn_explosion(&mut commands, pos);
                    hit_player = Some(g.player);
                    to_despawn.push(entity);
                    break;
                }
            }
        }

        // Despawn bananas we tracked (entities by index; safer approach is to iterate with Entity).
        for entity in to_despawn {
            commands.entity(entity).despawn_recursive();
        }

        if !to_despawn.is_empty() || hit_player.is_some() {
            turn.waiting_for_shot_to_finish = false;

            if let Some(player_hit) = hit_player {
                // The owner gets a point unless they hit themselves.
                let shooter = turn.current_player;
                if shooter == player_hit {
                    let other = if shooter == 1 { 2 } else { 1 };
                    turn.scores[other - 1] += 1;
                } else {
                    turn.scores[shooter - 1] += 1;
                }
            }

            turn.current_player = if turn.current_player == 1 { 2 } else { 1 };
        }
    }
}

fn spawn_explosion(commands: &mut Commands, pos: Vec3) {
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::ORANGE_RED,
                custom_size: Some(Vec2::new(32.0, 32.0)),
                ..Default::default()
            },
            transform: Transform::from_translation(pos + Vec3::new(0.0, 0.0, 20.0)),
            ..Default::default()
        })
        .insert(Explosion {
            timer: Timer::from_seconds(0.3, TimerMode::Once),
        });
}

fn update_explosions(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Explosion, &mut Sprite)>,
) {
    for (entity, mut explosion, mut sprite) in q.iter_mut() {
        explosion.timer.tick(time.delta());
        let k = 1.0 - explosion.timer.fraction();
        sprite.color.set_a(k);
        if explosion.timer.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn aabb_overlap(center_a: Vec2, size_a: Vec2, center_b: Vec2, size_b: Vec2) -> bool {
    let half_a = size_a * 0.5;
    let half_b = size_b * 0.5;
    let dx = (center_a.x - center_b.x).abs();
    let dy = (center_a.y - center_b.y).abs();
    dx <= half_a.x + half_b.x && dy <= half_a.y + half_b.y
}

fn update_ui_text(
    mut query: Query<&mut Text, With<UiText>>,
    turn: Res<TurnState>,
    physics: Res<PhysicsSettings>,
) {
    if let Ok(mut text) = query.get_single_mut() {
        let player = turn.current_player;
        text.sections[0].value = format!(
            "Rusgrillas (Rust GORILLA.BAS tribute)\n\
             Player: {}\n\
             Angle: {:.1}°   Power: {:.1}\n\
             Wind: {:.1}\n\
             Score: P1 {} - P2 {}\n\
             Controls: ←/→ angle, ↑/↓ power, Space/Enter to throw",
            player,
            turn.angle_deg,
            turn.power,
            physics.wind,
            turn.scores[0],
            turn.scores[1],
        );
    }
}

//! Rusgrillas: Gorillas port in Bevy 0.18
//! Entry point: adds default plugins and our game plugin.

use bevy::prelude::*;

mod game;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rusgrillas".into(),
                // Bevy 0.18 expects integer resolution here.
                resolution: (960, 540).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(game::GamePlugin)
        .run();
}

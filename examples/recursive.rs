use bevy::{
    color::palettes::{css::BLACK, tailwind::SKY_200},
    input::mouse::MouseMotion,
    prelude::*,
    render::render_resource::Face,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
#[cfg(feature = "gizmos")]
use bevy_easy_portals::gizmos::PortalGizmosPlugin;
use bevy_easy_portals::{Portal, PortalPlugins};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PortalPlugins,
            #[cfg(feature = "gizmos")]
            PortalGizmosPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (handle_camera_look, handle_movement, rotate_objects),
        )
        .run();
}

#[derive(Component)]
struct CameraController {
    sensitivity: f32,
    speed: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            sensitivity: 0.03,
            speed: 5.0,
        }
    }
}

#[derive(Component)]
struct RotatingObject;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Main camera with a cube representation
    let primary_cam = commands
        .spawn((
            CameraController::default(),
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(BLACK.into()),
                ..default()
            },
            Transform::from_xyz(0.0, 1.5, 8.0).looking_at(Vec3::new(0.0, 1.5, 0.0), Vec3::Y),
            AmbientLight {
                brightness: 750.0,
                ..default()
            },
        ))
        .id();

    let rectangle = Rectangle::from_size(Vec2::new(3.0, 3.0));

    // Portal 2 (right side) - looks to the LEFT (forward from its perspective)
    let portal_pos = Vec3::new(0.0, 1.5, 0.0);
    let target_pos = Vec3::new(0.0, 1.5, 10.0); // In front of portal 2 (to the left in world space)

    let target = commands.spawn(Transform::from_translation(target_pos)).id();

    commands.spawn((
        Mesh3d(meshes.add(rectangle)),
        Transform::from_translation(portal_pos),
        Portal::new(primary_cam, target).with_camera_spawn(|camera| {
            camera.insert((
                AmbientLight {
                    brightness: 750.0,
                    ..default()
                },
                Camera {
                    clear_color: ClearColorConfig::Custom(BLACK.into()),
                    ..default()
                },
            ));
        }),
        children![
            ((
                Mesh3d(meshes.add(Rectangle::from_size(vec2(3.05, 3.05)))),
                Transform::from_xyz(0.0, 0.0, -0.01),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    ..default()
                })),
            ))
        ],
    ));

    let shape_transform = Transform::from_xyz(0.0, 1.5, 2.0);

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: SKY_200.into(),
            ..default()
        })),
        shape_transform,
        RotatingObject,
    ));

    commands.spawn((
        PointLight {
            intensity: 10_000_000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 10.0, 0.0).looking_at(shape_transform.translation, Vec3::Y),
    ));
}

fn rotate_objects(time: Res<Time>, mut query: Query<&mut Transform, With<RotatingObject>>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() * 0.7);
        transform.rotate_x(time.delta_secs() * 0.3);
    }
}

fn handle_camera_look(
    mut mouse_motion: MessageReader<MouseMotion>,
    mut camera_query: Query<(&CameraController, &mut Transform)>,
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    cursor_options.visible = false;
    cursor_options.grab_mode = CursorGrabMode::Locked;

    let Ok((camera_controller, mut transform)) = camera_query.single_mut() else {
        return;
    };

    for message in mouse_motion.read() {
        let yaw_delta = Quat::from_rotation_y(
            (-message.delta.x * camera_controller.sensitivity)
                .clamp(-89.0, 89.0)
                .to_radians(),
        );
        let pitch_delta =
            Quat::from_rotation_x((-message.delta.y * camera_controller.sensitivity).to_radians());
        transform.rotation = yaw_delta * transform.rotation.normalize() * pitch_delta;
    }
}

fn handle_movement(
    keys: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<(&CameraController, &mut Transform)>,
    time: Res<Time>,
) {
    let Ok((camera_controller, mut transform)) = camera_query.single_mut() else {
        return;
    };

    let forward = transform.forward().with_y(0.0).normalize();
    let right = transform.right().with_y(0.0).normalize();

    let mut movement = Vec3::ZERO;

    if keys.pressed(KeyCode::KeyW) {
        movement += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        movement -= forward;
    }
    if keys.pressed(KeyCode::KeyA) {
        movement -= right;
    }
    if keys.pressed(KeyCode::KeyD) {
        movement += right;
    }

    transform.translation += movement * camera_controller.speed * time.delta_secs();
}

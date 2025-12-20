use std::f32::consts::PI;

use bevy::{color::palettes::tailwind::ORANGE_600, prelude::*};
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
        .add_systems(Update, rotate_shape)
        .run();
}

#[derive(Component)]
struct Shape;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // It's important we keep track of this entity, since the portal with require it
    let primary_camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            Transform::from_xyz(2.5, 0.0, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        ))
        .id();

    // Spawn a shape so we can see something in the reflection
    let shape_transform = Transform::from_xyz(0.0, 0.0, 4.0);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::from(ORANGE_600))),
        shape_transform,
        Shape,
    ));
    commands.spawn((
        PointLight {
            intensity: 10_000_000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 10.0, 0.0).looking_at(shape_transform.translation, Vec3::Y),
    ));

    let rectangle = Rectangle::from_size(Vec2::splat(5.0));

    let mirror = commands
        .spawn((
            // No need to spawn a material for the mesh here, it will be taken care of by the
            // portal setup
            Mesh3d(meshes.add(rectangle)),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .with_children(|parent| {
            // We can use another mesh for our mirror if we wish
            parent.spawn((
                Mesh3d(meshes.add(rectangle)),
                MeshMaterial3d(materials.add(Color::WHITE.with_alpha(0.2))),
            ));
        })
        .id();

    // The target should be the transform of the mirror itself, but flipped
    let target_transform = Transform::default().with_rotation(Quat::from_rotation_y(PI));
    let target = commands.spawn(target_transform).id();

    commands
        .entity(mirror)
        // Since we're constructing a mirror, let's parent the target to the mirror itself
        .add_child(target)
        // Now let's create the portal!
        .insert(
            Portal::new(primary_camera, target).with_camera_spawn(|camera| {
                // The camera does not inherit from the primary camera.
                camera.insert(Camera {
                    clear_color: ClearColorConfig::Custom(Color::BLACK),
                    ..default()
                });
            }),
        );
}

fn rotate_shape(mut shape_transform: Single<&mut Transform, With<Shape>>, time: Res<Time>) {
    let angle = time.delta_secs() / 2.0;
    shape_transform.rotate(Quat::from_axis_angle(Vec3::new(1.0, 1.0, 0.0), angle));
}

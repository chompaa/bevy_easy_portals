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
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let primary_camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            Transform::from_xyz(-3.5, 0.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
            AmbientLight {
                brightness: 750.0,
                ..default()
            },
        ))
        .id();

    let shape = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(materials.add(Color::from(ORANGE_600))),
            Transform::from_xyz(1.5, 0.0, 0.0),
        ))
        .id();

    let target_transform = Transform::from_xyz(0.0, 0.0, 2.0);
    let target = commands.spawn(target_transform).id();

    // We'll set the target relative to our shape, since that's what we want to look at
    commands.entity(shape).add_child(target);

    let rectangle = Rectangle::from_size(Vec2::splat(2.5));
    let portal_transform = Transform::from_xyz(-1.5, 0.0, 0.0);
    commands
        .spawn((
            // No need to spawn a material for the mesh here, it will be taken care of by the
            // portal setup
            Mesh3d(meshes.add(rectangle)),
            portal_transform,
            Portal::new(primary_camera, target).with_camera_spawn(|camera| {
                camera.insert(AmbientLight {
                    brightness: 750.0,
                    ..Default::default()
                });
            }),
        ))
        .with_children(|parent| {
            // We can use another mesh for our portal if we wish
            parent.spawn((
                Mesh3d(meshes.add(rectangle)),
                MeshMaterial3d(materials.add(Color::WHITE.with_alpha(0.05))),
            ));
        });
}

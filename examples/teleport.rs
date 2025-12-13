//! Demonstrates setting up two bidirectional portals with teleportation between them.
//!
//! Includes basic collision handling for transitioning seamlessly between portals, a simple camera
//! controller for movement and looking around, and a basic scene setup

use std::f32::consts::FRAC_PI_4;

use bevy::{
    camera::visibility::RenderLayers,
    color::palettes::tailwind::{SKY_200, SLATE_200},
    input::mouse::MouseMotion,
    math::bounding::{Aabb3d, IntersectsVolume},
    prelude::*,
    render::render_resource::Face,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
#[cfg(feature = "gizmos")]
use bevy_easy_portals::gizmos::PortalGizmosPlugin;
use bevy_easy_portals::{Portal, PortalPlugins, camera::PortalCameraSystems};

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
            (handle_camera_look, handle_movement, apply_shape_rotation),
        )
        .add_systems(
            PostUpdate,
            handle_portal_collision.before(PortalCameraSystems::UpdateFrusta),
        )
        .run();
}

const CAMERA_START_XYZ: Vec3 = Vec3::new(10.0, 0.5, 5.0);
const FLOOR_MESH_SIZE: f32 = 10.0;
const WALL_MESH_SIZE: f32 = 20.0;
const PORTAL_MESH_SIZE: f32 = 2.5;
const PORTAL_FRAME_SIZES_AND_TRANSLATIONS: [(Vec3, Vec3); 4] = [
    // Left
    (Vec3::new(0.1, 2.5, 0.2), Vec3::new(-1.3, -0.009, 0.0)),
    // Right
    (Vec3::new(0.1, 2.5, 0.2), Vec3::new(1.3, -0.009, 0.0)),
    // Top
    (Vec3::new(2.7, 0.1, 0.2), Vec3::new(0.0, 1.291, 0.0)),
    // Bottom
    (Vec3::new(2.7, 0.1, 0.2), Vec3::new(-0.0, -1.309, 0.0)),
];

// Component used for camera controlling
#[derive(Component)]
struct CameraController {
    // Sensitivity of the camera with respect to mouse movement
    sensitivity: f32,
    // Speed the controller moves in world space
    speed: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            sensitivity: 0.03,
            speed: 3.0,
        }
    }
}

// Component used to track collisions between an entity and portal
#[derive(Component, Clone, Copy)]
#[component(storage = "SparseSet")]
struct Collision {
    // Offset of the entity from the portal
    offset: Vec3,
    // Portal entity that is being collided with
    portal_entity: Entity,
}

// Component used to mark shapes to be rotated
#[derive(Component)]
struct Shape;

fn setup(
    mut commands: Commands,
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    cursor_options.grab_mode = CursorGrabMode::Locked;
    cursor_options.visible = false;

    let primary_camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            // TODO: A not-so-nice hack to help the rendering depth conflicts between the portal's
            // mesh and the near clipping plane of the camera. The value has been "fine-tuned" to
            // visual correctness. PRs that remove this hack are welcome :)
            Projection::Perspective(PerspectiveProjection {
                near: 1e-8,
                ..default()
            }),
            Transform::from_translation(CAMERA_START_XYZ),
            CameraController::default(),
            RenderLayers::from_layers(&[0, 1]),
        ))
        .id();

    commands.insert_resource(AmbientLight {
        brightness: 750.0,
        ..default()
    });

    let target_a = commands.spawn(Transform::IDENTITY).id();
    let portal_a = commands.spawn_empty().add_child(target_a).id();

    let target_b = commands.spawn(Transform::IDENTITY).id();
    let portal_b = commands.spawn_empty().add_child(target_b).id();

    let y_offset = -PORTAL_MESH_SIZE / 2.0 - 0.01;

    let floor_mesh = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(FLOOR_MESH_SIZE)));
    let torus_mesh = meshes.add(Torus::default());
    let portal_mesh = meshes.add(Rectangle::from_size(Vec2::splat(PORTAL_MESH_SIZE)));
    let wall_mesh = meshes.add(Cuboid::from_size(Vec3::splat(WALL_MESH_SIZE)));

    for (sign, color, portal, target) in [
        (-1.0, SKY_200, portal_a, target_b),
        (1.0, SLATE_200, portal_b, target_a),
    ] {
        // Floor
        commands.spawn((
            Mesh3d(floor_mesh.clone()),
            MeshMaterial3d(materials.add(Color::from(color))),
            Transform::from_xyz(10.0 * sign, y_offset, 0.0),
        ));

        // Walls
        let wall_material = StandardMaterial {
            reflectance: 0.0,
            base_color: color.into(),
            cull_mode: Some(Face::Front),
            ..default()
        };
        commands.spawn((
            Mesh3d(wall_mesh.clone()),
            MeshMaterial3d(materials.add(wall_material)),
            Transform::from_xyz(10.0 * sign, y_offset, 0.0),
        ));

        commands.spawn((
            Mesh3d(torus_mesh.clone()),
            MeshMaterial3d(materials.add(Color::BLACK)),
            Transform::from_xyz(15.0 * sign, y_offset + 1.5, 5.0 * sign)
                .with_rotation(Quat::from_axis_angle(Vec3::Z, FRAC_PI_4)),
            Shape,
        ));

        let portal_transform = Transform::from_xyz(10.0 * sign, 0.0, 0.0)
            .with_rotation(Quat::from_axis_angle(Vec3::Y, FRAC_PI_4));
        commands
            .entity(portal)
            .insert((
                Mesh3d(portal_mesh.clone()),
                portal_transform,
                // The mesh is a `Rectangle`, so to allow for the portal to be seen from both
                // sides, don't cull any of its faces.
                //
                // We should also flip the near plane normal when we are looking at the portal's
                // back face.
                Portal::new(primary_camera, target)
                    .with_cull_mode(None)
                    .with_flip_near_plane_normal(true),
                // Stop portals from recursively rendering eachother
                RenderLayers::layer(1),
            ))
            .with_children(|parent| {
                // Portal borders
                for (size, translation) in PORTAL_FRAME_SIZES_AND_TRANSLATIONS {
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::from_size(size.into()))),
                        MeshMaterial3d(materials.add(Color::BLACK)),
                        Transform::from_translation(translation.into()),
                    ));
                }
            });
    }
}

fn handle_portal_collision(
    mut commands: Commands,
    mut camera_query: Query<(Entity, &mut Transform), With<CameraController>>,
    portal_query: Query<(Entity, &Portal), With<Portal>>,
    transform_query: Query<&GlobalTransform, Without<CameraController>>,
    mut stored_collision: Option<Single<&mut Collision>>,
) {
    let (camera_entity, mut camera_transform) = camera_query.single_mut().unwrap();
    let camera_aabb = Aabb3d::new(camera_transform.translation, Vec3::ZERO);

    for (portal_entity, portal) in &portal_query {
        let portal_transform = transform_query.get(portal_entity).unwrap();
        let portal_aabb = Aabb3d::new(
            portal_transform.translation(),
            Vec2::splat(PORTAL_MESH_SIZE).extend(1.0),
        );

        // Are we currently inside a portal?
        if portal_aabb.intersects(&camera_aabb) {
            let offset = camera_transform.translation - portal_transform.translation();

            let Some(ref mut collision) = stored_collision else {
                commands.entity(camera_entity).insert(Collision {
                    offset,
                    portal_entity,
                });
                return;
            };

            if collision.portal_entity == portal_entity {
                let portal_forward = *portal_transform.forward();
                let start_side = collision.offset.dot(portal_forward).signum();
                let end_side = offset.dot(portal_forward).signum();

                // Have we moved to the other side of the portal?
                if start_side != end_side {
                    let target_transform = transform_query.get(portal.target).unwrap();

                    let relative_translation = portal_transform
                        .affine()
                        .inverse()
                        .transform_point3(camera_transform.translation);
                    // Now transform it back to world space using the target's transform
                    let translation = target_transform.transform_point(relative_translation);

                    let relative_rotation =
                        portal_transform.rotation().inverse() * camera_transform.rotation;
                    let rotation = target_transform.rotation() * relative_rotation;

                    camera_transform.translation = translation;
                    camera_transform.rotation = rotation;
                } else {
                    collision.offset = offset;
                }
            }
        } else if stored_collision
            .as_ref()
            .is_some_and(|c| c.portal_entity == portal_entity)
        {
            commands.entity(camera_entity).remove::<Collision>();
        }
    }
}

fn handle_camera_look(
    mut mouse_motion: MessageReader<MouseMotion>,
    mut camera_query: Query<(&CameraController, &mut Transform)>,
) {
    let (camera_controller, mut transform) = camera_query.single_mut().unwrap();

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
    let (camera_controller, mut transform) = camera_query.single_mut().unwrap();

    // Zero the y-vector to only allow lateral movement
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

fn apply_shape_rotation(mut shape_query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut shape_query {
        let angle = time.delta_secs() / 2.0;
        transform.rotate(Quat::from_axis_angle(Vec3::Z, angle));
    }
}

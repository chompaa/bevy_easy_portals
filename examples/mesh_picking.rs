use std::f32::consts::FRAC_PI_4;

use bevy::{
    camera::visibility::RenderLayers,
    color::palettes::tailwind::{SKY_200, VIOLET_400},
    picking::pointer::PointerInteraction,
    prelude::*,
};
#[cfg(feature = "gizmos")]
use bevy_easy_portals::gizmos::{PortalGizmos, PortalGizmosPlugin};
use bevy_easy_portals::{Portal, PortalPlugins, picking::PortalPickingPlugin};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PortalPlugins,
            PortalPickingPlugin,
            #[cfg(feature = "gizmos")]
            PortalGizmosPlugin,
            MeshPickingPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, draw_mesh_intersections)
        .run();
}

#[derive(Component)]
struct Glass;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    #[cfg(feature = "gizmos")] mut config_store: ResMut<GizmoConfigStore>,
) {
    let primary_camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            RenderLayers::from_layers(&[0, 1]),
            AmbientLight {
                brightness: 375.0,
                ..default()
            },
        ))
        .id();

    #[cfg(feature = "gizmos")]
    {
        let (config, _) = config_store.config_mut::<PortalGizmos>();
        config.render_layers = RenderLayers::layer(1)
    }

    let idle_material = materials.add(Color::from(SKY_200));
    let drag_material = materials.add(Color::from(VIOLET_400));

    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(idle_material.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0).with_rotation(Quat::from_rotation_z(-FRAC_PI_4)),
        ))
        .observe(rotate_xy_on_drag)
        .observe(update_material_on::<Pointer<DragStart>>(drag_material))
        .observe(update_material_on::<Pointer<DragEnd>>(idle_material));

    let target = commands.spawn(Transform::from_xyz(0.0, 0.0, 2.0)).id();
    let rectangle = Cuboid::from_size(Vec3::splat(2.0));

    let idle_material = materials.add(Color::WHITE.with_alpha(0.01));
    let hover_material = materials.add(Color::WHITE.with_alpha(0.04));

    for portal_transform in [
        Transform::from_xyz(-2.5, 0.0, -1.0).with_rotation(Quat::from_rotation_y(FRAC_PI_4)),
        Transform::from_xyz(2.5, 0.0, -1.0).with_rotation(Quat::from_rotation_y(-FRAC_PI_4)),
    ] {
        commands
            .spawn((
                Mesh3d(meshes.add(rectangle)),
                portal_transform,
                Portal::new(primary_camera, target).with_camera_spawn(|camera| {
                    camera.insert((
                        Camera {
                            clear_color: ClearColorConfig::Custom(Color::BLACK),
                            ..default()
                        },
                        AmbientLight {
                            brightness: 375.0,
                            ..default()
                        },
                    ));
                }),
                RenderLayers::layer(1),
                // We want to be able to hover the glass
                Pickable {
                    should_block_lower: false,
                    is_hoverable: true,
                },
            ))
            .with_children(|parent| {
                parent
                    .spawn((
                        Mesh3d(meshes.add(rectangle)),
                        MeshMaterial3d(idle_material.clone()),
                        Glass,
                        RenderLayers::layer(1),
                        // Similarly, we want to be able to hover the portal (to pick through)
                        Pickable {
                            should_block_lower: false,
                            is_hoverable: true,
                        },
                    ))
                    .observe(update_material_on::<Pointer<Over>>(hover_material.clone()))
                    .observe(update_material_on::<Pointer<Out>>(idle_material.clone()));
            })
            .observe(rotate_y_on_drag);
    }
}

fn update_material_on<E: EntityEvent>(
    new_material: Handle<StandardMaterial>,
) -> impl Fn(On<E>, Query<&mut MeshMaterial3d<StandardMaterial>>) {
    move |trigger, mut material_query| {
        if let Ok(mut material) = material_query.get_mut(trigger.event_target()) {
            material.0 = new_material.clone();
        }
    }
}

fn draw_mesh_intersections(
    pointers: Query<&PointerInteraction>,
    untargetable: Query<Entity, Or<(With<Portal>, With<Glass>)>>,
    mut gizmos: Gizmos,
) {
    for (position, normal) in pointers
        .iter()
        .flat_map(|interaction| interaction.iter())
        .filter_map(|(entity, hit)| {
            if !untargetable.contains(*entity) {
                hit.position.zip(hit.normal)
            } else {
                None
            }
        })
    {
        gizmos.arrow(position, position + normal.normalize() * 0.5, Color::WHITE);
    }
}

fn rotate_y_on_drag(drag: On<Pointer<Drag>>, mut transform_query: Query<&mut Transform>) {
    if !matches!(drag.button, PointerButton::Secondary) {
        return;
    }
    let mut transform = transform_query.get_mut(drag.event_target()).unwrap();
    transform.rotate_y(drag.delta.x * 0.005);
}

fn rotate_xy_on_drag(drag: On<Pointer<Drag>>, mut transform_query: Query<&mut Transform>) {
    if !matches!(drag.button, PointerButton::Primary) {
        return;
    }
    let mut transform = transform_query.get_mut(drag.event_target()).unwrap();
    transform.rotate_y(drag.delta.x * 0.02);
    transform.rotate_x(drag.delta.y * 0.02);
}

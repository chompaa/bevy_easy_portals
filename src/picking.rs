//! Portal picking functionality for `bevy_picking`.
//!
//! Add the [`PortalPickingPlugin`] to propagate picking messages from backends "through" portals.
//!
//! This module does *not* provide any backend for you. It provides custom inputs that are
//! compatible with any backend. The entity containing the [`Portal`] will need to be picked via a
//! backend, hits will then be sent "through" the target.
//!
//! Some backends support opt-in behavior for picking, where cameras and entities require a marker
//! component to be considered in the backend. This also applies to portal cameras.

use bevy::{
    camera::{CameraProjection, NormalizedRenderTarget},
    picking::{
        PickingSystems,
        backend::ray::{RayId, RayMap},
        hover::HoverMap,
        pointer::{Location, PointerId, PointerInput, PointerLocation},
    },
    platform::collections::HashMap,
    prelude::*,
};
use uuid::Uuid;

use crate::{
    Portal,
    camera::{PortalCamera, PortalImage},
};

/// Enables picking "through" [`Portal`]s.
pub struct PortalPickingPlugin;

impl Plugin for PortalPickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            fix_portal_rays
                .after(RayMap::repopulate)
                .in_set(PickingSystems::ProcessInput),
        )
        .add_systems(First, portal_picking.in_set(PickingSystems::PostInput))
        .add_observer(add_pointer);
    }
}

/// Adds [`PointerId`] and [`PointerLocation`] to entities that have a [`PortalImage`] added.
fn add_pointer(
    trigger: On<Add, PortalImage>,
    mut commands: Commands,
    query: Query<(Entity, &PortalImage)>,
) {
    let (entity, portal_image) = query.get(trigger.event_target()).unwrap();

    let location = Location {
        target: NormalizedRenderTarget::Image(portal_image.0.clone().into()),
        position: Vec2::ZERO,
    };

    commands.entity(entity).insert((
        PointerId::Custom(Uuid::new_v4()),
        PointerLocation::new(location),
    ));
}

/// Fix rays for portal cameras by manually computing them without the custom projection.
///
// TODO: This is a massive hack. Why do we even need this? The ray should start at the near plane
// and be just fine..
fn fix_portal_rays(
    portal_query: Query<(&Portal, &PointerId, &PointerLocation)>,
    camera_query: Query<(&Camera, &GlobalTransform, &Projection), With<PortalCamera>>,
    mut ray_map: ResMut<RayMap>,
) {
    for (portal, portal_pointer_id, pointer_location) in &portal_query {
        // Remove all rays for this portal's pointer ID.
        ray_map
            .map
            .retain(|ray_id, _| &ray_id.pointer != portal_pointer_id);

        let Ok((camera, camera_transform, projection)) = camera_query.get(portal.linked_camera)
        else {
            continue;
        };

        if !camera.is_active {
            continue;
        }

        let Projection::Perspective(projection) = projection else {
            continue;
        };

        // Create clean projection for ray generation
        let mut dummy_projection = projection.clone();
        dummy_projection.near_clip_plane = Vec4::new(0.0, 0.0, -1.0, -dummy_projection.near);
        let clean_clip_from_view = dummy_projection.get_clip_from_view();

        let Some(viewport_rect) = camera.logical_viewport_rect() else {
            continue;
        };

        let Some(pointer_loc_data) = pointer_location.location() else {
            continue;
        };

        let viewport_pos = pointer_loc_data.position;

        if !viewport_rect.contains(viewport_pos) {
            continue;
        }

        let ndc = (viewport_pos - viewport_rect.min) / viewport_rect.size() * 2.0 - Vec2::ONE;
        let ndc_flipped = Vec2::new(ndc.x, -ndc.y);

        let view_from_clip = clean_clip_from_view.inverse();
        let world_from_view = camera_transform.affine();

        let ndc_point_near = ndc_flipped.extend(1.0);
        let ndc_point_far = ndc_flipped.extend(f32::EPSILON);

        let view_point_near = view_from_clip.project_point3a(ndc_point_near.into());
        let view_point_far = view_from_clip.project_point3a(ndc_point_far.into());
        let view_dir = view_point_far - view_point_near;

        let origin: Vec3 = world_from_view.transform_point3a(view_point_near).into();
        let direction: Vec3 = world_from_view.transform_vector3a(view_dir).into();

        let Ok(dir) = Dir3::new(direction) else {
            continue;
        };

        let ray = Ray3d::new(origin, dir);

        ray_map
            .map
            .insert(RayId::new(portal.linked_camera, *portal_pointer_id), ray);
    }
}

/// Handles picking.
///
/// To allow for the [`PointerLocation`] to not lag behind, we raycast against the portal's normal.
/// This comes at the cost of a single frame hit delay.
fn portal_picking(
    mut commands: Commands,
    mut portal_query: Query<(
        Entity,
        &Portal,
        &PortalImage,
        &GlobalTransform,
        &PointerId,
        &mut PointerLocation,
    )>,
    camera_global_transform_query: Query<(&Camera, &GlobalTransform, &Projection)>,
    global_transform_query: Query<&GlobalTransform>,
    hover_map: Res<HoverMap>,
    pointer_state: Res<PointerState>,
    mut pointer_inputs: MessageReader<PointerInput>,
) {
    let mut portal_picks: HashMap<Entity, PointerId> = hover_map
        .iter()
        .flat_map(|(hover_pointer_id, hits)| {
            hits.iter()
                .filter(|(entity, _)| portal_query.contains(**entity))
                .map(|(entity, _)| (*entity, *hover_pointer_id))
        })
        .collect();

    // Handle dragged entities, which need to be considered for dragging in and out of portals.
    for ((pointer_id, _), pointer_state) in pointer_state.pointer_buttons.iter() {
        for &target in pointer_state
            .dragging
            .keys()
            .filter(|&entity| portal_query.contains(*entity))
        {
            portal_picks.insert(target, *pointer_id);
        }
    }

    for (
        portal_entity,
        portal,
        portal_image,
        portal_transform,
        &portal_pointer_id,
        mut portal_pointer_location,
    ) in &mut portal_query
    {
        let Some(&pick_pointer_id) = portal_picks.get(&portal_entity) else {
            // Lift the portal pointer if it's not being used.
            portal_pointer_location.location = None;
            continue;
        };

        let Ok((portal_camera, portal_camera_transform, _)) =
            camera_global_transform_query.get(portal.linked_camera)
        else {
            continue;
        };

        let Ok((primary_camera, primary_camera_transform, _)) =
            camera_global_transform_query.get(portal.primary_camera)
        else {
            continue;
        };

        let target = NormalizedRenderTarget::Image(portal_image.0.clone().into());

        for input in pointer_inputs
            .read()
            .filter(|input| input.pointer_id == pick_pointer_id)
        {
            let Ok(ray) =
                primary_camera.viewport_to_world(primary_camera_transform, input.location.position)
            else {
                continue;
            };

            let Some(distance) = ray.intersect_plane(
                portal_transform.translation(),
                InfinitePlane3d::new(*portal_transform.forward()),
            ) else {
                continue;
            };

            let world_point = ray.get_point(distance);

            let Ok(target_transform) = global_transform_query.get(portal.target) else {
                continue;
            };

            let relative_point = portal_transform
                .affine()
                .inverse()
                .transform_point3(world_point);
            // We need to avoid the clip plane.
            let transformed_point = target_transform.transform_point(relative_point)
                + *target_transform.forward() * 0.01;

            let Ok(position) =
                portal_camera.world_to_viewport(portal_camera_transform, transformed_point)
            else {
                continue;
            };

            commands.write_message(PointerInput {
                location: Location {
                    target: target.clone(),
                    position,
                },
                pointer_id: portal_pointer_id,
                action: input.action,
            });
        }
    }
}

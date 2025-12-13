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
    camera::NormalizedRenderTarget,
    picking::{
        PickingSystems,
        hover::HoverMap,
        pointer::{Location, PointerAction, PointerId, PointerInput, PointerLocation},
    },
    platform::collections::HashSet,
    prelude::*,
};
use uuid::Uuid;

use crate::{Portal, camera::PortalImage};

/// Enables picking "through" [`Portal`]s.
pub struct PortalPickingPlugin;

impl Plugin for PortalPickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PortalInput>()
            .add_systems(
                PreUpdate,
                (
                    portal_inputs.in_set(PickingSystems::Input),
                    portal_picking.in_set(PickingSystems::Last),
                ),
            )
            .add_observer(add_pointer);
    }
}

/// Used to send inputs obtained in [`portal_picking`] in the next frame.
#[derive(Message, Debug)]
struct PortalInput {
    pointer_id: PointerId,
    location: Location,
    action: PointerAction,
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

/// Maps incoming [`PortalInput`]s to [`PointerInput`]s.
fn portal_inputs(
    mut portal_inputs: MessageReader<PortalInput>,
    mut output: MessageWriter<PointerInput>,
) {
    for message in portal_inputs.read() {
        output.write(PointerInput {
            pointer_id: message.pointer_id,
            location: message.location.clone(),
            action: message.action,
        });
    }
}

/// Handles picking.
///
/// To allow for the [`PointerLocation`] to not lag behind, we raycast against the portal's normal.
/// This comes at the cost of a single frame hit delay.
fn portal_picking(
    portal_query: Query<(&Portal, &Transform, &PointerId, &PointerLocation)>,
    camera_global_transform_query: Query<(&Camera, &GlobalTransform)>,
    camera_query: Query<&Camera>,
    hover_map: Res<HoverMap>,
    pointer_state: Res<PointerState>,
    mut pointer_inputs: MessageReader<PointerInput>,
    mut portal_inputs: MessageWriter<PortalInput>,
    mut dragged_last_frame: Local<HashSet<(PointerId, Entity)>>,
) {
    let mut portals: HashSet<(PointerId, Entity)> = dragged_last_frame.drain().collect();

    for (hover_pointer_id, hits) in hover_map.iter() {
        for (entity, _hit_data) in hits.iter() {
            if portal_query.contains(*entity) {
                portals.insert((*hover_pointer_id, *entity));
            }
        }
    }

    // Currently, we have only retrieved portal entities if they are being hovered. However, this
    // does not allow dragging in-and-out of portals.
    for ((pointer_id, _pointer_button), pointer_state) in pointer_state.pointer_buttons.iter() {
        for &target in pointer_state
            .dragging
            .keys()
            .filter(|&entity| portal_query.contains(*entity))
        {
            dragged_last_frame.insert((*pointer_id, target));
            portals.insert((*pointer_id, target));
        }
    }

    for (pointer_id, entity) in portals {
        let Ok((portal, &portal_transform, &portal_pointer_id, portal_pointer_location)) =
            portal_query.get(entity)
        else {
            // This could fail because we store entities from the previous frame in
            // `dragged_last_frame`. There's no guarantee they will still have these components
            // this frame
            continue;
        };

        let Some(portal_camera) = portal
            .linked_camera
            .and_then(|camera| camera_query.get(camera).ok())
        else {
            continue;
        };
        let Ok((primary_camera, primary_camera_transform)) =
            camera_global_transform_query.get(portal.primary_camera)
        else {
            continue;
        };
        // TODO: Having `target` cached here is nice, but shouldn't `PointerLocation::Location` be
        // set to `None` if the portal isn't being hovered?
        let target = portal_pointer_location.location().cloned().unwrap().target;

        for input in pointer_inputs
            .read()
            .filter(|input| input.pointer_id == pointer_id)
        {
            // Manually retrieve the current pointer's position, so that it doesn't lag a frame
            // behind
            //
            // First, shoot a ray forward (w.r.t `primary_camera_transform`)
            let Ok(ray) =
                primary_camera.viewport_to_world(primary_camera_transform, input.location.position)
            else {
                continue;
            };
            // Get the distance from the ray's origin to the portal's normal
            let Some(distance) = ray.intersect_plane(
                portal_transform.translation,
                InfinitePlane3d::new(portal_transform.forward()),
            ) else {
                continue;
            };
            // We can get the world position of the intersection now. Finally, we use it and
            // convert to the portal camera's viewport
            let Ok(position) =
                portal_camera.world_to_viewport(primary_camera_transform, ray.get_point(distance))
            else {
                continue;
            };

            // We could use `Commands::send_message` here, but I'm not sure if it will hurt
            // performance
            portal_inputs.write(PortalInput {
                pointer_id: portal_pointer_id,
                location: Location {
                    target: target.clone(),
                    position,
                },
                action: input.action,
            });
        }
    }
}

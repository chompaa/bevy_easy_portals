use bevy::{
    asset::RenderAssetUsages,
    camera::{CameraUpdateSystems, ImageRenderTarget, RenderTarget, visibility::VisibilitySystems},
    ecs::system::SystemParam,
    log::error,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    window::{PrimaryWindow, WindowRef, WindowResized},
};

use crate::Portal;

/// Plugin that provides [`PortalCamera`] spawning/despawning, transform and frusta updates, and
/// resizing rendered portal images.
pub struct PortalCameraPlugin;

/// Label for systems that update [`Portal`] related cameras.
#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
pub enum PortalCameraSystems {
    /// Resizes [`Portal::cameras`]'s rendered image if any [`WindowResized`] messages are
    /// read.
    ///
    /// Runs in the [`PreUpdate`] schedule.
    ResizeImage,
    /// Updates the [`GlobalTransform`] and [`Transform`] components for [`Portal::linked_camera`]
    /// based on the [`Portal::primary_camera`]s [`GlobalTransform`].
    ///
    /// Runs in the [`PostUpdate`] schedule.
    UpdateTransform,
    /// Updates the near clip plane for each camera.
    ///
    /// Runs in the [`PostUpdate`] schedule.
    UpdateProjection,
    /// Disables cameras for portals that are not in view.
    ///
    /// Runs in the [`PostUpdate`] schedule.
    CheckVisibility,
}

impl Plugin for PortalCameraPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            (
                (
                    PortalCameraSystems::UpdateTransform,
                    PortalCameraSystems::UpdateProjection,
                )
                    .before(CameraUpdateSystems)
                    .chain(),
                PortalCameraSystems::CheckVisibility.after(VisibilitySystems::CheckVisibility),
            ),
        )
        .configure_sets(
            PreUpdate,
            (PortalCameraSystems::ResizeImage.run_if(on_message::<WindowResized>),),
        )
        .add_systems(
            PreUpdate,
            (resize_portal_images.in_set(PortalCameraSystems::ResizeImage),),
        )
        .add_systems(
            PostUpdate,
            (
                update_portal_camera_transform.in_set(PortalCameraSystems::UpdateTransform),
                update_portal_camera_projection.in_set(PortalCameraSystems::UpdateProjection),
                check_portal_visibility.in_set(PortalCameraSystems::CheckVisibility),
            ),
        )
        .add_observer(setup_portal_camera)
        .register_type::<(PortalCameraOf, PortalImage)>();
    }
}

/// Component used to mark a camera associated with a [`Portal`].
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[relationship(relationship_target = PortalCameras)]
pub struct PortalCameraOf(pub Entity);

/// Component that contains all of a [`Portal`]'s cameras.
#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component)]
#[relationship_target(relationship = PortalCameraOf, linked_spawn)]
pub struct PortalCameras(Vec<Entity>);

/// Component used to store a weak reference to a [`PortalCamera`]'s rendered image.
#[derive(Component, Reflect, Debug, Deref, DerefMut)]
#[reflect(Component)]
pub struct PortalImage(pub Handle<Image>);

/// Component that marks which iteration/recursion depth this camera represents.
#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct PortalIteration(pub u32);

/// System that is triggered whenever a [`Portal`] component is added to an entity.
///
/// Creates multiple cameras (one per iteration) to support recursive portal rendering.
/// Each iteration renders deeper into the portal recursion.
fn setup_portal_camera(
    trigger: On<Add, Portal>,
    mut commands: Commands,
    mut portal_query: Query<&mut Portal>,
    primary_camera_query: Query<(&Camera, &RenderTarget)>,
    global_transform_query: Query<&GlobalTransform>,
    mut portal_images: PortalImages,
) {
    let entity = trigger.event_target();
    let mut portal = portal_query.get_mut(entity).unwrap();

    let Ok((primary_cam, render_target)) = primary_camera_query.get(portal.primary_camera) else {
        error!(
            "could not setup portal camera {entity}: primary_camera does not contain a Camera component"
        );
        return;
    };

    let Ok(global_transform) = global_transform_query.get(portal.target).copied() else {
        error!("portal target is missing a GlobalTransform");
        return;
    };

    let Some(image_handle) = portal_images.with_camera(primary_cam, render_target) else {
        return;
    };

    let mut portal_cameras = Vec::new();

    commands
        .entity(entity)
        .insert(PortalImage(image_handle.clone()));

    let max_depth = portal.max_depth.get();
    for iteration in 0..max_depth + 1 {
        let handle = image_handle.clone();

        let mut camera_commands = commands.spawn((
            Camera::default(),
            Camera3d::default(),
            global_transform.compute_transform(),
            global_transform,
            PortalCameraOf(entity),
            PortalIteration(iteration as u32),
            RenderTarget::Image(ImageRenderTarget {
                handle,
                scale_factor: 1.0,
            }),
        ));

        if iteration == max_depth {
            camera_commands.insert(portal.blank_render_layer.clone());
        }

        if let Some(camera_spawn_fn) = &mut portal.camera_spawn {
            (camera_spawn_fn)(&mut camera_commands);
        }

        camera_commands
            .entry::<Camera>()
            .and_modify(move |mut camera| {
                camera.order = camera.order - (iteration as isize + 1);
            });

        let camera_entity = camera_commands.id();
        portal_cameras.push(camera_entity);
    }

    portal.cameras = PortalCameras(portal_cameras);
}

fn check_portal_visibility(
    portal_query: Query<(&Portal, &ViewVisibility)>,
    mut camera_query: Query<&mut Camera, With<PortalCameraOf>>,
) {
    for (portal, visibility) in &portal_query {
        let is_visible = visibility.get();

        for &camera_entity in &portal.cameras.0 {
            if let Ok(mut camera) = camera_query.get_mut(camera_entity) {
                camera.is_active = is_visible;
            }
        }
    }
}

/// System that updates portal camera transforms based on iteration depth.
///
/// Each iteration applies the portal transformation recursively.
pub fn update_portal_camera_transform(
    portal_query: Query<(&Portal, &GlobalTransform), (Without<Camera3d>, Without<PortalCameraOf>)>,
    mut params: ParamSet<(
        TransformHelper,
        Query<(&mut GlobalTransform, &mut Transform, &PortalIteration), With<PortalCameraOf>>,
    )>,
) {
    for (portal, portal_global_transform) in portal_query.iter() {
        let transform_helper = params.p0();

        let Ok(primary_camera_transform) =
            transform_helper.compute_global_transform(portal.primary_camera)
        else {
            continue;
        };
        let Ok(target_transform) = transform_helper.compute_global_transform(portal.target) else {
            continue;
        };

        let mut portal_camera_query = params.p1();
        for &camera_entity in &portal.cameras.0 {
            let Ok((mut cam_global_transform, mut cam_transform, iteration)) =
                portal_camera_query.get_mut(camera_entity)
            else {
                continue;
            };

            let mut position = primary_camera_transform.translation();
            let mut rotation = primary_camera_transform.rotation();

            for _ in 0..=iteration.0 {
                let relative_translation = portal_global_transform
                    .affine()
                    .inverse()
                    .transform_point3(position);
                position = target_transform.transform_point(relative_translation);

                let relative_rotation = portal_global_transform.rotation().inverse() * rotation;
                rotation = target_transform.rotation() * relative_rotation;
            }

            cam_transform.translation = position;
            cam_transform.rotation = rotation;
            *cam_global_transform = GlobalTransform::from(Transform {
                translation: position,
                rotation,
                ..default()
            });
        }
    }
}

/// System that updates [`Projection`]s for portal cameras with oblique near clip planes.
fn update_portal_camera_projection(
    portal_query: Query<(&Portal, &GlobalTransform)>,
    mut projections: Query<&mut Projection, With<PortalCameraOf>>,
    global_transform_query: Query<&GlobalTransform>,
) {
    for (portal, portal_transform) in &portal_query {
        for &camera_entity in &portal.cameras.0 {
            let Ok(mut projection) = projections.get_mut(camera_entity) else {
                continue;
            };

            let Projection::Perspective(projection) = &mut *projection else {
                continue;
            };

            let Ok(
                [
                    portal_camera_transform,
                    primary_camera_transform,
                    target_transform,
                ],
            ) = global_transform_query.get_many([
                camera_entity,
                portal.primary_camera,
                portal.target,
            ])
            else {
                continue;
            };

            let mut world_normal = target_transform.forward().normalize();

            if portal.flip_near_plane_normal {
                let primary_to_portal =
                    portal_transform.translation() - primary_camera_transform.translation();
                let dot = primary_to_portal.dot(*portal_transform.forward());
                if dot <= 0.0 {
                    world_normal = -world_normal;
                }
            }

            let view_from_world = portal_camera_transform.affine().matrix3.inverse();
            let view_space_normal = (view_from_world * world_normal).normalize();
            let view_space_target = portal_camera_transform
                .affine()
                .inverse()
                .transform_point3(target_transform.translation());
            let distance = -view_space_normal.dot(view_space_target);

            projection.near_clip_plane = view_space_normal.extend(distance);
        }
    }
}

/// System that resizes portal camera images when the window is resized.
fn resize_portal_images(
    primary_cameras: Query<(&Camera, &RenderTarget), Without<PortalCameraOf>>,
    mut portal_cameras: Query<&mut RenderTarget, With<PortalCameraOf>>,
    mut portals: Query<(&Portal, &mut PortalImage)>,
    mut portal_images: PortalImages,
) {
    for (portal, mut portal_image) in &mut portals {
        let Ok((primary_camera, render_target)) = primary_cameras.get(portal.primary_camera) else {
            continue;
        };

        let Some(handle) = portal_images.with_camera(primary_camera, render_target) else {
            continue;
        };

        portal_images.images.remove(&portal_image.0);

        for &camera_entity in &portal.cameras.0 {
            let Ok(mut render_target) = portal_cameras.get_mut(camera_entity) else {
                continue;
            };

            *render_target = RenderTarget::Image(ImageRenderTarget {
                handle: handle.clone(),
                scale_factor: 1.0,
            });
        }

        portal_image.0 = handle;
    }
}

#[derive(SystemParam)]
struct PortalImages<'w, 's> {
    primary_window_query: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    window_query: Query<'w, 's, &'static Window>,
    images: ResMut<'w, Assets<Image>>,
    manual_texture_views: Res<'w, ManualTextureViews>,
}

impl PortalImages<'_, '_> {
    /// Creates a new [`Image`] with size matching the given `camera`.
    ///
    /// Returns `None` if no viewport size could be obtained.
    fn with_camera(
        &mut self,
        camera: &Camera,
        render_target: &RenderTarget,
    ) -> Option<Handle<Image>> {
        let size = self.get_viewport_size(camera, render_target)?;
        let mut image = Image::new_uninit(
            size,
            TextureDimension::D2,
            TextureFormat::Bgra8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage |= TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST
            | TextureUsages::RENDER_ATTACHMENT;
        let handle = self.images.add(image);
        Some(handle)
    }

    /// Retrieves the size of the viewport of a given `camera`.
    ///
    /// Returns `None` if no sizing could be obtained.
    fn get_viewport_size(&self, camera: &Camera, render_target: &RenderTarget) -> Option<Extent3d> {
        match camera.viewport.as_ref() {
            Some(viewport) => Some(viewport.physical_size),
            None => match &render_target {
                RenderTarget::Window(window_ref) => (match window_ref {
                    WindowRef::Primary => Some(self.primary_window_query.single().unwrap()),
                    WindowRef::Entity(entity) => Some(self.window_query.get(*entity).unwrap()),
                })
                .map(Window::physical_size),
                RenderTarget::Image(img_render_handle) => {
                    self.images.get(&img_render_handle.handle).map(Image::size)
                }
                RenderTarget::TextureView(handle) => self
                    .manual_texture_views
                    .get(handle)
                    .map(|texture| texture.size),
                RenderTarget::None { size } => Some(*size),
            },
        }
        .map(|size| Extent3d {
            width: size.x,
            height: size.y,
            ..default()
        })
    }
}

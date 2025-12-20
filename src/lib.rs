#![doc = include_str!("../README.md")]

pub mod camera;
#[cfg(feature = "gizmos")]
pub mod gizmos;
pub mod material;
#[cfg(feature = "picking")]
pub mod picking;

use std::num::NonZeroUsize;

use bevy::{
    app::PluginGroupBuilder, camera::visibility::RenderLayers, prelude::*,
    render::render_resource::Face,
};

use crate::camera::PortalCameras;

/// A group of plugins that provides the required systems to make a [`Portal`] work.
pub struct PortalPlugins;

impl PluginGroup for PortalPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(camera::PortalCameraPlugin)
            .add(material::PortalMaterialPlugin)
    }
}

/// Component used to create a portal.
///
/// If [`camera::PortalCameraPlugin`] is enabled, adding this to an entity causes a camera (marked
/// with [`camera::PortalCamera`], and with [`RenderTarget::Image`]) to be spawned, inheriting the
/// the properties of [`Portal::primary_camera`].
///
/// If [`material::PortalMaterialPlugin`] is enabled, a [`material::PortalMaterial`] is inserted on
/// the entity, inherting [`Portal::cull_mode`] for convenience.
///
/// [`RenderTarget::Image`]: bevy::render::camera::RenderTarget
#[non_exhaustive]
#[derive(Component)]
#[require(Transform)]
pub struct Portal {
    /// The entity with the primary render [`Camera`].
    ///
    /// In other words, the camera used to look at this portal.
    pub primary_camera: Entity,
    /// The target entity that should be used to decide the camera's position.
    ///
    /// This entity should contain a [`Transform`] component.
    pub target: Entity,
    /// The cameras that belong to this portal.
    ///
    /// If `max_depth` is set to `0`, this will contain two entities.
    cameras: PortalCameras,
    /// Maximum depth for recursive portals.
    ///
    /// By default, this is set to `1`, meaning no recursive rendering will occur.
    ///
    /// Notes
    ///
    /// * `max_depth` determines the amount of extra cameras to spawn, which can impact performance.
    pub max_depth: NonZeroUsize,
    /// The camera systems spawn a camera with a render layer that nothing else belongs too, so we
    /// can render a "blank" image, so that recursion doesn't look so jarring.
    ///
    /// By default, this is `RenderLayers::layer(42)`.
    pub blank_render_layer: RenderLayers,
    /// Optional callback that is executed when the portals's camera is spawned.
    ///
    /// This lets you insert any components you'd like.
    ///
    /// # Notes
    ///
    /// * If `None`, [`Camera::order`] is set to `-1`.
    /// * [`Camera::target`] is overriden after the callback is executed.
    pub camera_spawn: Option<Box<dyn FnMut(&mut EntityCommands) + Send + Sync>>,
    /// Specifies which side of the portal to cull: "front", "back", or neither.
    ///
    /// If set to `None`, both sides of the portal’s mesh will be rendered.
    ///
    /// Defaults to `Some(Face::Back)`, similar to [`StandardMaterial::cull_mode`].
    ///
    /// # Note
    ///
    /// If you are using `Some(Face::Front)` or `None` here, and your mesh is flat, you should
    /// consider setting [`Portal::flip_near_plane_normal`] to `true`.
    // TODO: Can this be remotely reflected upstream now that #6042 has landed?
    pub cull_mode: Option<Face>,
    /// If set to `true` this will flip the near plane of the [`camera::PortalCamera`]s frustum if
    /// the primary camera is facing the back face of the portal.
    ///
    /// This is particularly useful for portals that are flat meshes and don't have their back face
    /// culled. In other words, set this to `true` if you have a bidirectional portal with a flat
    /// mesh. Otherwise, set it to `false`.
    ///
    /// Set to `false` by default.
    pub flip_near_plane_normal: bool,
}

impl Portal {
    /// Creates a new [`Portal`] from a given `primary_camera` and `target`.
    ///
    /// # See Also
    ///
    /// * [`Portal::primary_camera`]
    /// * [`Portal::target`]
    #[inline]
    #[must_use]
    pub fn new(primary_camera: Entity, target: Entity) -> Self {
        Self {
            primary_camera,
            target,
            max_depth: NonZeroUsize::new(7).unwrap(),
            blank_render_layer: RenderLayers::layer(42),
            camera_spawn: None,
            cull_mode: Some(Face::Back),
            cameras: PortalCameras::default(),
            flip_near_plane_normal: false,
        }
    }

    #[inline]
    #[must_use]
    pub fn with_cull_mode(mut self, cull_mode: Option<Face>) -> Self {
        self.cull_mode = cull_mode;
        self
    }

    #[inline]
    #[must_use]
    pub fn with_flip_near_plane_normal(mut self, with_flip_near_plane_normal: bool) -> Self {
        self.flip_near_plane_normal = with_flip_near_plane_normal;
        self
    }

    #[inline]
    #[must_use]
    pub fn with_camera_spawn<F>(mut self, camera_spawn: F) -> Self
    where
        F: FnMut(&mut EntityCommands) + Send + Sync + 'static,
    {
        self.camera_spawn = Some(Box::new(camera_spawn));
        self
    }
}

use bevy::{
    asset::{AssetPath, embedded_asset, embedded_path},
    core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, CompareFunction, DepthBiasState, DepthStencilState, Face,
        RenderPipelineDescriptor, SpecializedMeshPipelineError, StencilFaceState, StencilState,
    },
    shader::ShaderRef,
    window::WindowResized,
};

use crate::{
    Portal,
    camera::{PortalCameraSystems, PortalImage},
};

pub struct PortalMaterialPlugin;

impl Plugin for PortalMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/portal.wgsl");

        app.add_plugins(MaterialPlugin::<PortalMaterial>::default())
            .add_systems(
                PreUpdate,
                update_materials
                    .run_if(on_message::<WindowResized>)
                    .after(PortalCameraSystems::ResizeImage),
            )
            .add_observer(spawn_material);
    }
}

/// Material used for a [`Portal`]'s mesh.
#[derive(Asset, AsBindGroup, Clone, Reflect)]
#[bind_group_data(PortalMaterialKey)]
pub struct PortalMaterial {
    #[texture(0)]
    #[sampler(1)]
    base_color_texture: Option<Handle<Image>>,
    /// Specifies which side of the portal to cull: "front", "back", or neither.
    ///
    /// If set to `None`, both sides of the portal’s mesh will be rendered.
    ///
    /// This field's value is inherited from what is set on [`Portal`], but not kept in sync.
    ///
    /// Defaults to `Some(Face::Back)`, similar to [`StandardMaterial::cull_mode`] and [`Portal`].
    #[reflect(ignore)]
    pub cull_mode: Option<Face>,
    /// The effect of draw calls on the depth and stencil aspects of the portal.
    ///
    /// You can make use of this field to resolve z-fighting.
    ///
    /// Defaults to the standard mesh [`DepthStencilState`].
    #[reflect(ignore)]
    pub depth_stencil: Option<DepthStencilState>,
}

impl Default for PortalMaterial {
    fn default() -> Self {
        Self {
            base_color_texture: None,
            cull_mode: Some(Face::Back),
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState::default(),
            }),
        }
    }
}

impl Material for PortalMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("shaders/portal.wgsl")).with_source("embedded"),
        )
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = key.bind_group_data.cull_mode;
        descriptor.depth_stencil = key.bind_group_data.depth_stencil;
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PortalMaterialKey {
    cull_mode: Option<Face>,
    depth_stencil: Option<DepthStencilState>,
}

impl From<&PortalMaterial> for PortalMaterialKey {
    fn from(material: &PortalMaterial) -> Self {
        Self {
            cull_mode: material.cull_mode,
            depth_stencil: material.depth_stencil.clone(),
        }
    }
}

fn spawn_material(
    trigger: On<Add, PortalImage>,
    mut commands: Commands,
    portal_query: Query<(&Portal, &PortalImage)>,
    mut materials: ResMut<Assets<PortalMaterial>>,
) {
    let entity = trigger.event_target();
    let Ok((portal, portal_image)) = portal_query.get(entity) else {
        return;
    };
    commands
        .entity(entity)
        .insert_if_new(MeshMaterial3d(materials.add(PortalMaterial {
            base_color_texture: Some(portal_image.0.clone()),
            cull_mode: portal.cull_mode,
            ..default()
        })));
}

fn update_materials(
    portals: Query<(&PortalImage, &MeshMaterial3d<PortalMaterial>), Changed<PortalImage>>,
    mut materials: ResMut<Assets<PortalMaterial>>,
) {
    for (portal_image, material) in &portals {
        let Some(portal_material) = materials.get_mut(&material.0) else {
            continue;
        };
        portal_material.base_color_texture = Some(portal_image.0.clone());
    }
}

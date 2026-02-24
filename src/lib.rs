use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::TextureFormat,
};

pub struct MaterialPreviewPlugin {
    pub render_layers: RenderLayers,
}

impl Plugin for MaterialPreviewPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RenderLayersInUse(self.render_layers.clone()));
        app.add_systems(Startup, setup_studio);
        app.add_systems(Update, render_studio);
    }
}

impl Default for MaterialPreviewPlugin {
    fn default() -> Self {
        Self {
            render_layers: RenderLayers::layer(31),
        }
    }
}

#[derive(Resource)]
struct RenderLayersInUse(RenderLayers);

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct MaterialPreviewToRender {
    pub material: Handle<StandardMaterial>,
    pub size: UVec2,
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct RenderedMaterialPreview {
    pub image: Handle<Image>,
}

#[derive(Component)]
struct StudioObject;

fn setup_studio(
    mut commands: Commands,
    render_layers: Res<RenderLayersInUse>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        render_layers.0.clone(),
        StudioObject,
    ));
    commands.spawn((
        Camera3d::default(),
        Camera {
            is_active: false,
            clear_color: ClearColorConfig::None,
            ..Default::default()
        },
        Transform::from_xyz(0.0, 1.5, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        render_layers.0.clone(),
        StudioObject,
    ));
    commands.spawn((
        PointLight::default(),
        Transform::from_xyz(3.0, 3.0, 2.0),
        render_layers.0.clone(),
        StudioObject,
    ));
}

fn render_studio(
    mut commands: Commands,
    single: Single<(Entity, &MaterialPreviewToRender), Added<MaterialPreviewToRender>>,
    studio_mesh_material: Single<
        &mut MeshMaterial3d<StandardMaterial>,
        (With<StudioObject>, With<Mesh3d>),
    >,
    studio_camera: Single<(&mut Camera, &mut RenderTarget, &mut Projection), With<StudioObject>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (entity, request) = single.into_inner();

    // 更换材质
    let mut material = studio_mesh_material.into_inner();
    *material = MeshMaterial3d(request.material.clone());

    // 准备渲染目标, 激活摄像机并渲染到目标
    let image = images.add(Image::new_target_texture(
        request.size.x,
        request.size.y,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    let (mut camera, mut target, mut projection) = studio_camera.into_inner();
    camera.is_active = true;
    *target = RenderTarget::Image(image.clone().into());
    // 同步宽高比
    if let Projection::Perspective(ref mut perspective) = *projection {
        // 计算纹理的宽高比
        perspective.aspect_ratio = request.size.x as f32 / request.size.y as f32;
    }

    // 返回结果
    commands
        .entity(entity)
        .remove::<MaterialPreviewToRender>()
        .insert(RenderedMaterialPreview { image });
}

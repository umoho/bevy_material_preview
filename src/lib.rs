use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    mesh::VertexAttributeValues,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
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

#[derive(Component, Debug, Clone, PartialEq)]
pub struct MaterialPreviewToRender {
    pub material: Handle<StandardMaterial>,
    pub size: UVec2,
    pub with_plane: bool,
    pub camera_translation: Vec3,
}

impl Default for MaterialPreviewToRender {
    fn default() -> Self {
        Self {
            material: Default::default(),
            size: UVec2::splat(96),
            with_plane: Default::default(),
            camera_translation: (0.0, 1.5, 2.5).into(),
        }
    }
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct RenderedMaterialPreview {
    pub image: Handle<Image>,
}

#[derive(Component)]
struct StudioObject;

#[derive(Component)]
struct StudioMesh;

#[derive(Component)]
struct StudioPlane;

fn setup_studio(
    mut commands: Commands,
    render_layers: Res<RenderLayersInUse>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // 地板
    commands.spawn((
        Mesh3d(meshes.add(new_plane_mesh())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(new_checker_image())),
            perceptual_roughness: 0.8,
            ..Default::default()
        })),
        Transform::from_xyz(0.0, -1.0, 0.0),
        render_layers.0.clone(),
        StudioObject,
        StudioPlane,
    ));
    // 球体
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        render_layers.0.clone(),
        StudioObject,
        StudioMesh,
    ));
    // 摄像机
    commands.spawn((
        Camera3d::default(),
        Camera {
            is_active: false,
            clear_color: ClearColorConfig::None,
            ..Default::default()
        },
        Transform::from_xyz(0.0, 1.5, 2.5).looking_at(Vec3::ZERO, Vec3::Y),
        render_layers.0.clone(),
        StudioObject,
    ));
    // 主灯
    commands.spawn((
        PointLight {
            intensity: 1200000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::from_xyz(4.0, 4.0, 2.0),
        render_layers.0.clone(),
        StudioObject,
    ));
    // 补灯 (背光, 边缘光)
    commands.spawn((
        PointLight {
            intensity: 400000.0,
            ..Default::default()
        },
        Transform::from_xyz(-4.0, 2.0, -2.0),
        render_layers.0.clone(),
        StudioObject,
    ));
}

type CameraComponents = (
    &'static mut Camera,
    &'static mut RenderTarget,
    &'static mut Projection,
    &'static mut Transform,
);

fn render_studio(
    mut commands: Commands,
    single: Single<(Entity, &MaterialPreviewToRender), Added<MaterialPreviewToRender>>,
    studio_mesh_material: Single<&mut MeshMaterial3d<StandardMaterial>, With<StudioMesh>>,
    studio_camera: Single<CameraComponents, With<StudioObject>>,
    studio_plane: Single<&mut Visibility, With<StudioPlane>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (entity, request) = single.into_inner();

    if !request.with_plane {
        let mut visibility = studio_plane.into_inner();
        *visibility = Visibility::Hidden;
    }

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
    let (mut camera, mut target, mut projection, mut transform) = studio_camera.into_inner();
    camera.is_active = true;
    *target = RenderTarget::Image(image.clone().into());
    // 同步宽高比
    if let Projection::Perspective(ref mut perspective) = *projection {
        // 计算纹理的宽高比
        perspective.aspect_ratio = request.size.x as f32 / request.size.y as f32;
    }
    *transform = transform
        .with_translation(request.camera_translation)
        .looking_at(Vec3::ZERO, Vec3::Y);

    // 返回结果
    commands
        .entity(entity)
        .remove::<MaterialPreviewToRender>()
        .insert(RenderedMaterialPreview { image });
}

fn new_checker_image() -> Image {
    let size = Extent3d {
        width: 2,
        height: 2,
        depth_or_array_layers: 1,
    };
    let pixel = [
        40, 40, 40, 255, // 深灰
        100, 100, 100, 255, // 中灰
        100, 100, 100, 255, // 中灰
        40, 40, 40, 255, // 深灰
    ];
    let mut checker = Image::new_fill(
        size,
        TextureDimension::D2,
        &pixel,
        TextureFormat::Rgba8UnormSrgb,
        Default::default(),
    );

    checker.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        ..Default::default()
    });

    checker
}

fn new_plane_mesh() -> Mesh {
    // 缩放地板纹理坐标, 使格子看起来更密
    let mut plane = Plane3d {
        half_size: Vec2::splat(40.0),
        ..Default::default()
    }
    .mesh()
    .build();
    if let Some(VertexAttributeValues::Float32x2(uvs)) = plane.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for uv in uvs.iter_mut() {
            *uv = [uv[0] * 40.0, uv[1] * 40.0];
        }
    }
    plane
}

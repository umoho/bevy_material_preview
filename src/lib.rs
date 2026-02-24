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
        app.add_systems(Update, (spawn_preview_studio, cleanup_studio));
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
    /// 球体的材质.
    pub material: Handle<StandardMaterial>,
    /// 渲染的图片的尺寸, 默认为 96x96.
    pub size: UVec2,
    /// 是否需要地板作为背景?
    pub with_plane: bool,
    /// 使摄像机远离球体, 沿着摄像机看球体的反方向.
    pub distance_offset: f32,
}

impl Default for MaterialPreviewToRender {
    fn default() -> Self {
        Self {
            material: Default::default(),
            size: UVec2::splat(96),
            with_plane: Default::default(),
            distance_offset: Default::default(),
        }
    }
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct RenderedMaterialPreview {
    pub image: Handle<Image>,
}

#[derive(Component)]
struct StudioRoot {
    /// 请求者
    _user_entity: Entity,
    frames_to_live: u8,
}

fn spawn_preview_studio(
    mut commands: Commands,
    query: Query<(Entity, &MaterialPreviewToRender), Added<MaterialPreviewToRender>>,
    render_layers: Res<RenderLayersInUse>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (user_entity, request) in query {
        // 计算唯一的偏移量
        let offset = Vec3::new(user_entity.index_u32() as f32 * 50.0, 0.0, 0.0);

        // 创建渲染目标纹理
        let target_texture = images.add(Image::new_target_texture(
            request.size.x,
            request.size.y,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));

        // 计算摄像机本地变换
        let camera_transform = calculate_camera_transform(request.distance_offset);
        // 计算裁剪面
        let total_dist = camera_transform.translation.length();

        // 产生工作室节点
        commands
            .spawn((
                StudioRoot {
                    _user_entity: user_entity,
                    frames_to_live: 3,
                },
                Transform::from_translation(offset),
                Visibility::default(),
                InheritedVisibility::default(),
            ))
            .with_children(|parent| {
                // 球体
                parent.spawn((
                    Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(32).unwrap())),
                    MeshMaterial3d(request.material.clone()),
                    render_layers.0.clone(),
                ));
                // 地板 (按需生成)
                if request.with_plane {
                    parent.spawn((
                        Mesh3d(meshes.add(new_plane_mesh())),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color_texture: Some(images.add(new_checker_image())),
                            perceptual_roughness: 0.8,
                            ..Default::default()
                        })),
                        Transform::from_xyz(0.0, -1.0, 0.0),
                        render_layers.0.clone(),
                    ));
                }
                // 摄像机
                parent.spawn((
                    Camera3d::default(),
                    Camera {
                        clear_color: ClearColorConfig::Custom(Color::NONE),
                        ..Default::default()
                    },
                    Projection::Perspective(PerspectiveProjection {
                        far: total_dist + 2.0,
                        near: (total_dist - 2.0).max(0.1),
                        aspect_ratio: request.size.x as f32 / request.size.y as f32,
                        ..Default::default()
                    }),
                    RenderTarget::Image(target_texture.clone().into()),
                    camera_transform,
                    render_layers.0.clone(),
                ));
                // 主灯
                parent.spawn((
                    PointLight {
                        intensity: 1200000.0,
                        shadows_enabled: true,
                        ..Default::default()
                    },
                    Transform::from_xyz(4.0, 4.0, 2.0),
                    render_layers.0.clone(),
                ));
                // 补灯 (背光, 边缘光)
                parent.spawn((
                    PointLight {
                        intensity: 400000.0,
                        ..Default::default()
                    },
                    Transform::from_xyz(-4.0, 2.0, -2.0),
                    render_layers.0.clone(),
                ));
            });

        commands
            .entity(user_entity)
            .remove::<MaterialPreviewToRender>()
            .insert(RenderedMaterialPreview {
                image: target_texture,
            });
    }
}

fn cleanup_studio(mut commands: Commands, query: Query<(Entity, &mut StudioRoot)>) {
    for (root_entity, mut studio) in query {
        if studio.frames_to_live <= 0 {
            commands.entity(root_entity).despawn_children().despawn();
        } else {
            studio.frames_to_live -= 1;
        }
    }
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

/// 根据距离偏移量计算摄像机的本地变换,
/// 假设标准视角起点是 (0.0, 1.5, 2.5).
fn calculate_camera_transform(distance_offset: f32) -> Transform {
    let base_local_pos = Vec3::new(0.0, 1.5, 2.5);
    let direction = base_local_pos.normalize_or_zero();
    let final_local_pos = base_local_pos + direction * distance_offset;
    Transform::from_translation(final_local_pos).looking_at(Vec3::ZERO, Vec3::Y)
}

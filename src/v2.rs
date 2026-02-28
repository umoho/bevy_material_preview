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
        app.register_material_preview::<StandardMaterial>();
    }
}

impl Default for MaterialPreviewPlugin {
    fn default() -> Self {
        Self {
            render_layers: RenderLayers::layer(31),
        }
    }
}

pub trait MaterialPreviewAppExt {
    fn register_material_preview<M: Material>(&mut self) -> &mut Self;
}

impl MaterialPreviewAppExt for App {
    fn register_material_preview<M: Material>(&mut self) -> &mut Self {
        self.add_systems(Update, init_sessions::<M>)
    }
}

#[derive(Resource)]
struct RenderLayersInUse(RenderLayers);

/// 材质预览的离屏渲染会话.
///
/// 当实体携带这个组件时, 由插件自动执行离屏渲染, 随后将结果放置于 `result` 字段;
/// 若更改这个组件的值, 则会触发重新渲染.
#[derive(Component)]
pub struct MaterialPreviewSession<M: Material> {
    /// 物体的材质.
    pub material: Handle<M>,
    /// 渲染的图片的尺寸, 默认为 96x96.
    pub size: UVec2,
    /// 是否需要地板作为背景?
    pub with_plane: bool,
    /// 使摄像机沿着摄像机看物体的反方向远离物体.
    pub distance_offset: f32,
    /// 渲染结果, 将由插件自动填充, 用户从这个字段读出结果.
    pub result: Option<Handle<Image>>,
}

/// 存储该会话对应的3D场景实体句柄.
#[derive(Component)]
struct ActiveStudioScene {
    studio_root: Entity,
    object_entity: Entity,
    camera_entity: Entity,
    plane_entity: Entity,
}

fn init_sessions<M: Material>(
    mut commands: Commands,
    new_sessions: Query<(Entity, &mut MaterialPreviewSession<M>), Added<MaterialPreviewSession<M>>>,
    mut images: ResMut<Assets<Image>>,
    render_layers: Res<RenderLayersInUse>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    for (requirer, mut session) in new_sessions {
        // 计算唯一的偏移量
        let offset = Vec3::new(requirer.index_u32() as f32 * 50.0, 0.0, 0.0);

        // 创建渲染目标纹理
        let target_texture = images.add(Image::new_target_texture(
            session.size.x,
            session.size.y,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
        session.result = Some(target_texture.clone());

        // 计算摄像机本地变换
        let camera_transform = calculate_camera_transform(session.distance_offset);
        // 计算裁剪面
        let total_dist = camera_transform.translation.length();

        let mut object_entity = None;
        let mut plane_entity = None;
        let mut camera_entity = None;

        let studio_root = commands
            .spawn((
                Name::new("Preview_Studio"),
                Transform::from_translation(offset),
                Visibility::default(),
                InheritedVisibility::default(),
            ))
            .with_children(|parent| {
                // 球体
                object_entity = Some(
                    parent
                        .spawn((
                            Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(32).unwrap())),
                            MeshMaterial3d(session.material.clone()),
                            render_layers.0.clone(),
                        ))
                        .id(),
                );
                // 地板
                plane_entity = Some(
                    parent
                        .spawn((
                            Mesh3d(meshes.add(new_plane_mesh())),
                            MeshMaterial3d(standard_materials.add(StandardMaterial {
                                base_color_texture: Some(images.add(new_checker_image())),
                                perceptual_roughness: 0.8,
                                ..Default::default()
                            })),
                            Transform::from_xyz(0.0, -1.0, 0.0),
                            render_layers.0.clone(),
                            // 根据 session 初始值决定可见性
                            if session.with_plane {
                                Visibility::Visible
                            } else {
                                Visibility::Hidden
                            },
                        ))
                        .id(),
                );
                // 摄像机
                camera_entity = Some(
                    parent
                        .spawn((
                            Camera3d::default(),
                            Camera {
                                clear_color: ClearColorConfig::Custom(Color::NONE),
                                ..Default::default()
                            },
                            Projection::Perspective(PerspectiveProjection {
                                far: total_dist + 2.0,
                                near: (total_dist - 2.0).max(0.1),
                                aspect_ratio: session.size.x as f32 / session.size.y as f32,
                                ..Default::default()
                            }),
                            RenderTarget::Image(target_texture.into()),
                            camera_transform,
                            render_layers.0.clone(),
                        ))
                        .id(),
                );
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
            })
            .id(); // studio_root

        commands.entity(requirer).insert(ActiveStudioScene {
            studio_root,
            object_entity: object_entity.unwrap(),
            camera_entity: camera_entity.unwrap(),
            plane_entity: plane_entity.unwrap(),
        });
    }
}

/// 根据距离偏移量计算摄像机的本地变换,
/// 假设标准视角起点是 (0.0, 1.5, 2.5).
fn calculate_camera_transform(distance_offset: f32) -> Transform {
    let base_local_pos = Vec3::new(0.0, 1.5, 2.5);
    let direction = base_local_pos.normalize_or_zero();
    let final_local_pos = base_local_pos + direction * distance_offset;
    Transform::from_translation(final_local_pos).looking_at(Vec3::ZERO, Vec3::Y)
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

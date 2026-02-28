use bevy::prelude::*;
use bevy_material_preview::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // 添加插件 (默认会注册 StandardMaterial)
        .add_plugins(MaterialPreviewPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (bind_session_result_to_ui, animate_material_session),
        )
        .run();
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn(Camera2d);

    // 定义四种不同颜色的测试材质
    let configs = [
        (Color::srgb(1.0, 0.1, 0.1), "Red"),
        (Color::srgb(0.1, 1.0, 0.1), "Green"),
        (Color::srgb(0.1, 0.1, 1.0), "Blue"),
        (Color::srgb(1.0, 1.0, 0.1), "Yellow"),
    ];

    // 创建一个全屏容器, 水平排列四个预览框
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::SpaceEvenly,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            for (color, name) in configs {
                let mat_handle = materials.add(StandardMaterial {
                    base_color: color,
                    metallic: 0.5,
                    perceptual_roughness: 0.1,
                    ..default()
                });

                // 每个预览项的容器
                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|node| {
                        // 文字标题
                        node.spawn(Text::new(name));

                        node.spawn((
                            Node {
                                width: Val::Px(200.0),
                                height: Val::Px(200.0),
                                margin: UiRect::all(Val::Px(10.0)),
                                border: UiRect::all(Val::Px(3.0)),
                                ..default()
                            },
                            BorderColor::all(Color::WHITE),
                            ImageNode::default(),
                            // 挂载会话组件
                            MaterialPreviewSession::<StandardMaterial> {
                                material: mat_handle,
                                size: UVec2::splat(512), // 512x512 渲染分辨率
                                with_plane: true,        // 显示棋盘格地板
                                distance_offset: 0.0,
                                target: None, // 初始为空, 由插件填充
                            },
                        ));
                    }); // parent
            } // for
        }); // commands
}

/// 绑定预览纹理给 [ImageNode].
fn bind_session_result_to_ui(
    mut query: Query<
        (&MaterialPreviewSession<StandardMaterial>, &mut ImageNode),
        Changed<MaterialPreviewSession<StandardMaterial>>,
    >,
) {
    for (session, mut image_node) in query.iter_mut() {
        // 一旦插件完成初始化并填充了 target 字段
        if let Some(handle) = &session.target {
            // 将句柄设置给 UI 节点
            if image_node.image.id() != handle.id() {
                image_node.image = handle.clone();
                info!("UI 已成功绑定预览纹理");
            }
        }
    }
}

/// 修改会话数据, 验证丝滑更新渲染结果.
fn animate_material_session(
    time: Res<Time>,
    mut query: Query<&mut MaterialPreviewSession<StandardMaterial>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    for (i, mut session) in query.iter_mut().enumerate() {
        let t = time.elapsed_secs();

        // 动态修改摄像机距离 (在 -1.0 到 1.0 之间往复)
        session.distance_offset = (t + i as f32).sin() * 0.5;

        // 动态修改材质金属度和粗糙度 (0.0 到 1.0)
        if let Some(material) = material_assets.get_mut(session.material.id()) {
            material.metallic = (2.0 * (t + i as f32)).sin().abs();
            material.perceptual_roughness = (2.0 * (t + i as f32)).sin().abs();
        }
    }
}

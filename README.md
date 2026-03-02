# `bevy_material_preview`

这个crate提供了Bevy插件`MaterialPreviewPlugin`, 它实现了当在实体中插入组件`MaterialPreviewSession`时,
在新的渲染层(Render Layers)渲染材质预览图.

## 用法

在实体中插入`MaterialPreviewSession`组件, 然后使用其中的`target`字段的图片句柄`Handle<Image>`.

```rust
fn setup(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn(
        MaterialPreviewSession::<StandardMaterial> {
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.1, 0.1),
                metallic: 0.5,
                perceptual_roughness: 0.1,
                ..default()
            }),
            size: UVec2::splat(512), // 512x512 渲染分辨率
            with_plane: true,        // 显示棋盘格地板
            target: None,            // 如果为`None`, 插件会创建, 否则会使用你提供的`Handle<Image>`.
            ..default()
        },
    );
}
```

请参考示例:

```
cargo r --example preivew
```

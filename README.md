# `bevy_easy_portals`

Easy-to-use portals for Bevy

![screenshot showing a cube being reflected in a mirror using portals](https://raw.githubusercontent.com/chompaa/bevy_easy_portals/main/assets/mirror.png)

## Getting Started

First, add `PortalPlugin` to your app, then use the `Portal` component, et voila!

See [the examples](https://github.com/chompaa/bevy_easy_portals/tree/main/examples) for more references.

<details>

<summary>Example usage</summary>

```rust
use bevy::prelude::*;
use bevy_easy_portals::{Portal, PortalPlugins}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PortalPlugins))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let primary_camera = commands
        .spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 10.0)))
        .id();

    // Spawn something for the portal to look at
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(10.0, 0.0, 0.0),
    ));

    // Where the portal's target camera should be
    let target = commands.spawn(Transform::from_xyz(10.0, 0.0, 10.0)).id();
    // Where the portal should be located
    let portal_transform = Transform::default();
    // Spawn the portal, omit a material since one will be added automatically
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::default())),
        portal_transform,
        Portal::new(primary_camera, target),
    ));
}
```

</details>

## Compatibility

| `bevy_easy_portals` | `bevy` |
| :--                 | :--    |
| `0.6`               | `0.17` |
| `0.5`               | `0.16` |
| `0.3..0.4`          | `0.15` |
| `0.1..0.2`          | `0.14` |

## Features

| Feature                | Description                                                       |
| :--                    | :--                                                               |
| `picking`              | Support picking through portals with using your favorite backend  |
| `gizmos`               | Use gizmos for the portal's aabb and camera transform             |

## Contributing

Feel free to open a PR!

If possible, please try to keep it minimal and scoped.

## Alternatives

- [`bevy_basic_portals`](https://github.com/Selene-Amanita/bevy_basic_portals)

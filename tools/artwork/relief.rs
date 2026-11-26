//! Shared atlas rendering for scenery authored as sampled height profiles.

use scenic_flight::world_mesh::WorldMesh;

const GRID: usize = 22;

pub(super) fn draw(
    mesh: &mut WorldMesh<'_>,
    size: [f32; 2],
    profile: impl Fn([f32; 2]) -> Option<(f32, [u8; 3])>,
) {
    let sample = |[x, z]: [usize; 2]| {
        profile([
            (x as f32 + 0.5) * (2.0 / GRID as f32) - 1.0,
            (z as f32 + 0.5) * (2.0 / GRID as f32) - 1.0,
        ])
    };
    let at = |x, z| {
        [
            (x as f32 / GRID as f32 - 0.5) * size[0],
            (z as f32 / GRID as f32 - 0.5) * size[1],
        ]
    };
    for z in 0..GRID {
        for x in 0..GRID {
            let Some((height, material)) = sample([x, z]) else {
                continue;
            };
            let corners = [at(x, z), at(x + 1, z), at(x + 1, z + 1), at(x, z + 1)];
            let roof = corners.map(|[x, z]| [x, height, z]);
            mesh.faded_quad(roof, material.map(|value| value.saturating_add(16)), 1.0);
            let neighbors = [
                z.checked_sub(1).map(|z| [x, z]),
                (x + 1 < GRID).then_some([x + 1, z]),
                (z + 1 < GRID).then_some([x, z + 1]),
                x.checked_sub(1).map(|x| [x, z]),
            ];
            for (edge, neighbor) in neighbors.into_iter().enumerate() {
                let low = neighbor.and_then(sample).map_or(0.0, |(height, _)| height);
                if low >= height {
                    continue;
                }
                let next = (edge + 1) & 3;
                let [[x0, z0], [x1, z1]] = [corners[edge], corners[next]];
                let color = material.map(|value| value.saturating_sub([9, 22, 26, 12][edge]));
                mesh.faded_quad(
                    [roof[edge], roof[next], [x1, low, z1], [x0, low, z0]],
                    color,
                    1.0,
                );
            }
        }
    }
}

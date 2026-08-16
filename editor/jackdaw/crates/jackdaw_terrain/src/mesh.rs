use crate::heightmap::Heightmap;

/// Raw mesh data for a terrain chunk, ready to be turned into a Bevy `Mesh`.
pub struct ChunkMeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

/// Build mesh data for a single chunk of the terrain.
///
/// `chunk_x` / `chunk_z`: chunk indices (0-based).
/// `chunk_size`: number of cells per chunk edge.
///
/// Positions are in terrain-local space (terrain centered at origin).
pub fn build_chunk_mesh_data(
    heightmap: &Heightmap,
    chunk_x: u32,
    chunk_z: u32,
    chunk_size: u32,
) -> ChunkMeshData {
    let res = heightmap.resolution;
    let cell = heightmap.cell_size();
    let half_size = heightmap.size / 2.0;

    // Grid range for this chunk (in vertex indices)
    let x_start = chunk_x * chunk_size;
    let z_start = chunk_z * chunk_size;
    let x_end = ((chunk_x + 1) * chunk_size + 1).min(res);
    let z_end = ((chunk_z + 1) * chunk_size + 1).min(res);

    let cols = x_end - x_start;
    let rows = z_end - z_start;

    let mut positions = Vec::with_capacity((cols * rows) as usize);
    let mut normals = Vec::with_capacity((cols * rows) as usize);
    let mut uvs = Vec::with_capacity((cols * rows) as usize);

    for lz in 0..rows {
        for lx in 0..cols {
            let gx = x_start + lx;
            let gz = z_start + lz;
            let h = heightmap.get_height(gx, gz);

            let world_x = gx as f32 * cell.x - half_size.x;
            let world_z = gz as f32 * cell.y - half_size.y;

            positions.push([world_x, h, world_z]);

            // UV normalized across full terrain
            let u = gx as f32 / (res - 1) as f32;
            let v = gz as f32 / (res - 1) as f32;
            uvs.push([u, v]);

            // Normal via central differences
            let h_left = if gx > 0 {
                heightmap.get_height(gx - 1, gz)
            } else {
                h
            };
            let h_right = if gx < res - 1 {
                heightmap.get_height(gx + 1, gz)
            } else {
                h
            };
            let h_down = if gz > 0 {
                heightmap.get_height(gx, gz - 1)
            } else {
                h
            };
            let h_up = if gz < res - 1 {
                heightmap.get_height(gx, gz + 1)
            } else {
                h
            };

            let dx = (h_right - h_left) / (2.0 * cell.x);
            let dz = (h_up - h_down) / (2.0 * cell.y);
            let len = (dx * dx + 1.0 + dz * dz).sqrt();
            normals.push([-dx / len, 1.0 / len, -dz / len]);
        }
    }

    // Triangle indices
    let cells_x = cols - 1;
    let cells_z = rows - 1;
    let mut indices = Vec::with_capacity((cells_x * cells_z * 6) as usize);

    for lz in 0..cells_z {
        for lx in 0..cells_x {
            let tl = lz * cols + lx;
            let tr = tl + 1;
            let bl = (lz + 1) * cols + lx;
            let br = bl + 1;

            indices.push(tl);
            indices.push(bl);
            indices.push(tr);

            indices.push(tr);
            indices.push(bl);
            indices.push(br);
        }
    }

    ChunkMeshData {
        positions,
        normals,
        uvs,
        indices,
    }
}

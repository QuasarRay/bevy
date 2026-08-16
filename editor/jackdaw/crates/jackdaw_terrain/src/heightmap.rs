use bevy_math::Vec2;

/// Pure heightmap data structure -- no Bevy ECS dependencies.
#[derive(Clone, Debug)]
pub struct Heightmap {
    /// Vertices per edge.
    pub resolution: u32,
    /// World-space XZ dimensions.
    pub size: Vec2,
    /// Maximum height value for normalization.
    pub max_height: f32,
    /// Row-major height data, length = resolution^2.
    pub heights: Vec<f32>,
}

impl Default for Heightmap {
    fn default() -> Self {
        let resolution = 256;
        Self {
            resolution,
            size: Vec2::new(100.0, 100.0),
            max_height: 50.0,
            heights: vec![0.0; (resolution * resolution) as usize],
        }
    }
}

impl Heightmap {
    pub fn new(resolution: u32, size: Vec2, max_height: f32) -> Self {
        Self {
            resolution,
            size,
            max_height,
            heights: vec![0.0; (resolution * resolution) as usize],
        }
    }

    /// Get height at integer grid coordinates. Returns 0 if out of bounds.
    pub fn get_height(&self, x: u32, z: u32) -> f32 {
        if x >= self.resolution || z >= self.resolution {
            return 0.0;
        }
        self.heights[(z * self.resolution + x) as usize]
    }

    /// Set height at integer grid coordinates.
    pub fn set_height(&mut self, x: u32, z: u32, h: f32) {
        if x < self.resolution && z < self.resolution {
            self.heights[(z * self.resolution + x) as usize] = h;
        }
    }

    /// Convert a local-space position (relative to terrain origin) to fractional grid coordinates.
    pub fn world_to_grid(&self, local_pos: Vec2) -> Vec2 {
        let cell = self.cell_size();
        // Terrain is centered at origin, so offset by half size
        let offset = local_pos + self.size / 2.0;
        Vec2::new(offset.x / cell.x, offset.y / cell.y)
    }

    /// Bilinear interpolation of height at fractional grid coordinates.
    pub fn sample_bilinear(&self, gx: f32, gz: f32) -> f32 {
        let x0 = gx.floor() as i32;
        let z0 = gz.floor() as i32;
        let fx = gx - x0 as f32;
        let fz = gz - z0 as f32;

        let s = |x: i32, z: i32| -> f32 {
            let x = x.clamp(0, self.resolution as i32 - 1) as u32;
            let z = z.clamp(0, self.resolution as i32 - 1) as u32;
            self.get_height(x, z)
        };

        let h00 = s(x0, z0);
        let h10 = s(x0 + 1, z0);
        let h01 = s(x0, z0 + 1);
        let h11 = s(x0 + 1, z0 + 1);

        let h0 = h00 * (1.0 - fx) + h10 * fx;
        let h1 = h01 * (1.0 - fx) + h11 * fx;
        h0 * (1.0 - fz) + h1 * fz
    }

    /// Number of chunks along each axis given a chunk cell size.
    pub fn chunk_count(&self, chunk_size: u32) -> (u32, u32) {
        let cells = self.resolution - 1;
        let cx = cells.div_ceil(chunk_size);
        let cz = cells.div_ceil(chunk_size);
        (cx, cz)
    }

    /// World-space size of one grid cell.
    pub fn cell_size(&self) -> Vec2 {
        Vec2::new(
            self.size.x / (self.resolution - 1) as f32,
            self.size.y / (self.resolution - 1) as f32,
        )
    }
}

use super::Geometry;

/// Shape of [triangular polygon 3D mesh](super::mesh::Mesh) with compiled
/// buffer data.
pub struct BufferGeometry {
    /// Position attribute buffer. Every consecutive triplet specifies the
    /// position of a vertex relative to the geometry's origin.
    pub position: Vec<f32>,
    /// Normal attribute buffer. Every consecutive triplet specifies the normal
    /// vector of the corresponding vertex in [`position`](Self::position).
    pub normal: Vec<f32>,
    /// UV attribute buffer. Every consecutive pair of numbers specifies the
    /// UV coordinates of the corresponding vertex in [`position`](Self::position).
    pub uv: Vec<f32>,
    /// Optional list of indices. Every consecutive triplet defines a triangle
    /// formed by the vertices at the specified indices. If the list is `None`,
    /// every consecutive triplet of vertices defines a triangle.
    pub indices: Option<Vec<u32>>,
}

impl BufferGeometry {
    /// Creates a 3D geometry from the given geometry builder.
    pub fn from_geometry<G: Geometry>(geometry: &G) -> Self {
        let (position, normal, uv) = geometry.vertices();

        Self {
            position,
            normal,
            uv,
            indices: geometry.indices(),
        }
    }
}

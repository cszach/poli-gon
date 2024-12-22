/// Generator of common geometry buffers such as vertices and indices. Should be
/// implemented on all geometry parameter structs.
///
/// There is an emphasis on the word "generator": the methods will create new
/// arrays of data every time they are invoked. To store the data, use
/// [`BufferGeometry::from_geometry`](super::BufferGeometry::from_geometry).
pub trait Geometry {
    /// Generates the vertices for this geometry. The return value is a tuple of
    /// the position buffer, the normal buffer, and the UV buffer in that order.
    fn vertices(&self) -> (Vec<f32>, Vec<f32>, Vec<f32>);
    /// Generates the indices for this geometry if supported, otherwise returns
    /// [`None`](std::option::Option::None).
    fn indices(&self) -> Option<Vec<u32>>;
}

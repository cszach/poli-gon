use std::{cell::RefCell, rc::Rc};

use crate::geometries::BufferGeometry;

/// Triangular polygon mesh 3D object.
///
/// A mesh has a geometry (its shape) and a material (its look) and is the most
/// primitive [kind of 3D object](super::Object3dKind).
pub struct Mesh {
    /// The triangular polygon geometry.
    pub geometry: Rc<BufferGeometry>,
    /// The associated GPU bind group, which contains information such as world
    /// matrix, normal matrix, etc.
    pub bind_group: RefCell<Option<wgpu::BindGroup>>,
}

impl Mesh {
    /// Creates a new 3D mesh with the specified geometry.
    pub fn new(geometry: Rc<BufferGeometry>) -> Self {
        Self {
            geometry,
            bind_group: RefCell::new(None),
        }
    }
}

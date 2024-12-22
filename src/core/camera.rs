use std::{cell::RefCell, f32::consts::PI};

use poli_math::{Matrix4, Vector3};

/// Contains different kinds of cameras.
pub enum CameraKind {
    /// A camera that uses perspective projection, similar to a pinhole camera.
    PerspectiveCamera {
        /// The vertical field-of-view in radians.
        vfov_radians: f32,
        /// The aspect ratio, usually set to the aspect ratio of the drawing
        /// window.
        aspect: f32,
        /// The near plane of the camera frustum. Objects closer to the camera
        /// than this amount in the view space's z axis will not get rendered.
        /// Must be greater than `0.0` and less than [`far`][#far].
        near: f32,
        /// The far plane of the camera frustum. Objects further to the camera
        /// than this amount in the view space's z axis will not get rendered.
        /// Can be set to [`INFINITY`](std::f32::INFINITY).
        far: f32,
    },
}

/// The view point from which 3D scenes are rendered.
pub struct Camera {
    /// The view matrix, which will be used in the rasterization pipeline to
    /// transform world coordinates into view-space coordinates.
    pub view_matrix: Matrix4,
    /// The projection matrix, which will be used in the rasterization pipeline
    /// to transform view-space coordinates into normalized device coordinates
    /// (NDC).
    ///
    /// The NDC is consistent to the [WebGPU NDC][ndc].
    ///
    /// [ndc]: https://gpuweb.github.io/gpuweb/#coordinate-systems
    pub projection_matrix: Matrix4,
    /// The inverse of the [`projection_matrix`][#projection_matrix].
    pub projection_matrix_inverse: Matrix4,
    /// The kind of camera e.g. perspective, orthographic.
    pub kind: RefCell<CameraKind>,
    /// The position of the camera in world space, for writing into the camera
    /// position buffer. Do not modify this property directly, instead use
    /// transformation methods on the encapsulating [`Object3D`](super::Object3d).
    pub position: Vector3,
}

impl Camera {
    /// Creates a new camera of the specified kind.
    pub fn new(kind: CameraKind) -> Self {
        let projection_matrix = match kind {
            CameraKind::PerspectiveCamera {
                vfov_radians,
                aspect,
                near,
                far,
            } => {
                let f = (PI * 0.5 - 0.5 * vfov_radians).tan();

                let (n33, n34) = if far.is_finite() {
                    let range_inv = 1.0 / (near - far);
                    (far * range_inv, far * near * range_inv)
                } else {
                    (-1.0, -near)
                };

                Matrix4 {
                    elements: [
                        f / aspect,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        f,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        n33,
                        -1.0,
                        0.0,
                        0.0,
                        n34,
                        0.0,
                    ],
                }
            }
        };

        Self {
            view_matrix: Matrix4::identity(),
            projection_matrix,
            projection_matrix_inverse: projection_matrix.inverse(),
            kind: RefCell::new(kind),
            position: Vector3::default(),
        }
    }

    /// Updates the projection matrix based on the current camera parameters.
    /// Use this method after you have manually change any of the camera's
    /// parameters e.g. aspect ratio.
    pub fn update_projection_matrix(&mut self) {
        match *self.kind.borrow() {
            CameraKind::PerspectiveCamera {
                vfov_radians,
                aspect,
                near,
                far,
            } => {
                let f = (PI * 0.5 - 0.5 * vfov_radians).tan();

                let (n33, n34) = if far.is_finite() {
                    let range_inv = 1.0 / (near - far);
                    (far * range_inv, far * near * range_inv)
                } else {
                    (-1.0, -near)
                };

                self.projection_matrix.elements[0] = f / aspect;
                self.projection_matrix.elements[5] = f;
                self.projection_matrix.elements[10] = n33;
                self.projection_matrix.elements[14] = n34;
            }
        }

        self.projection_matrix_inverse = self.projection_matrix.inverse();
    }
}

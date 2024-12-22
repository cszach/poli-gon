use std::{
    cell::RefCell,
    collections::VecDeque,
    ops::Mul,
    rc::{Rc, Weak},
};

use poli_math::{Matrix4, Quaternion, Vector3};

use super::{Camera, Mesh};

/// Contains different kinds of 3D objects.
pub enum Object3dKind {
    Mesh(Box<Mesh>),
    Group,
    Camera(Rc<RefCell<Camera>>),
}

/// 3D object, which is anything that can have a transformation in 3D space
/// e.g. position, rotation, scale.
pub struct Object3d {
    /// The name of this 3D object.
    pub name: RefCell<Option<String>>,
    /// The parent of this 3D object in the 3D scene hierarchy.
    pub parent: RefCell<Weak<Object3d>>,
    /// The children of this 3D object in the 3D scene hierarchy.
    pub children: RefCell<Vec<Rc<Object3d>>>,
    /// The local matrix, which encodes the position, rotation, and scale of
    /// this 3D object in model space.
    pub local_matrix: RefCell<Matrix4>,
    /// The world matrix, which encodes the position, rotation, and scale of
    /// this 3D object in world space.
    pub world_matrix: RefCell<Matrix4>,
    /// The kind of 3D object e.g. mesh, group, camera, light.
    pub kind: Object3dKind,
    /// Whether or not this 3D object is visible.
    pub visible: RefCell<bool>,
}

impl From<Mesh> for Object3d {
    fn from(mesh: Mesh) -> Self {
        Self::new(Object3dKind::Mesh(Box::new(mesh)))
    }
}

impl From<Camera> for Object3d {
    fn from(camera: Camera) -> Self {
        Self::new(Object3dKind::Camera(Rc::new(RefCell::new(camera))))
    }
}

/// [Breadth-first traversal][bfs] iterator for the scene graph.
///
/// [bfs]: https://en.wikipedia.org/wiki/Breadth-first_search
pub struct BfsIterator {
    queue: VecDeque<Rc<Object3d>>,
}

impl Iterator for BfsIterator {
    type Item = Rc<Object3d>;

    fn next(&mut self) -> Option<Self::Item> {
        let object = self.queue.pop_front();

        if let Some(ref parent) = object {
            let children = parent.children.borrow();

            self.queue.extend(children.iter().map(Rc::clone));
        }

        object
    }
}

/// [Depth-first traversal][dfs] iterator for the scene graph.
///
/// [dfs]: https://en.wikipedia.org/wiki/Depth-first_search
pub struct DfsIterator {
    stack: Vec<Rc<Object3d>>,
}

impl Iterator for DfsIterator {
    type Item = Rc<Object3d>;

    fn next(&mut self) -> Option<Self::Item> {
        let object = self.stack.pop();

        if let Some(ref parent) = object {
            let children = parent.children.borrow();

            self.stack.extend(children.iter().rev().map(Rc::clone));
        }

        object
    }
}

impl Object3d {
    /// Creates a new 3D object of the specified kind at the world origin.
    pub fn new(kind: Object3dKind) -> Self {
        Self {
            name: RefCell::new(None),
            parent: RefCell::new(Weak::new()),
            children: RefCell::new(Vec::new()),
            local_matrix: RefCell::new(Matrix4::identity()),
            world_matrix: RefCell::new(Matrix4::identity()),
            kind,
            visible: RefCell::new(true),
        }
    }

    /// Returns the [breadth-first traversal][bfs] iterator for the given 3D
    /// object, which traverses the scene graph level-by-level starting with the
    /// given 3D object itself.
    ///
    /// [bfs]: https://en.wikipedia.org/wiki/Breadth-first_search
    pub fn bfs(object: &Rc<Self>) -> BfsIterator {
        BfsIterator {
            queue: VecDeque::from(vec![Rc::clone(object)]),
        }
    }

    /// Returns the [depth-first traversal][dfs] iterator for the given 3D
    /// object, which explores along each branch as far as possible before
    /// backtracking.
    ///
    /// [dfs]: https://en.wikipedia.org/wiki/Depth-first_search
    pub fn dfs(object: &Rc<Self>) -> DfsIterator {
        DfsIterator {
            stack: vec![Rc::clone(object)],
        }
    }

    /// Adds the `child` object to the `parent` object and updates the `child`
    /// object's and its descendants' world matrices.
    pub fn add(parent: &Rc<Self>, child: &Rc<Self>) {
        *child.parent.borrow_mut() = Rc::downgrade(parent);
        parent.children.borrow_mut().push(Rc::clone(child));

        for object in Object3d::bfs(child) {
            if let Some(parent) = object.parent.borrow().upgrade() {
                *object.world_matrix.borrow_mut() =
                    parent.world_matrix.borrow().as_ref() * object.local_matrix.borrow().as_ref()
            }
        }
    }

    /// Translates the given 3D object in 3D space and updates its descendants'
    /// world matrices.
    pub fn translate(object: &Rc<Self>, v: &Vector3) {
        object.local_matrix.borrow_mut().translate(v);
        Self::update_world_matrix(object, true);
    }

    /// Rotates the given 3D object and updates its descendants' world matrices.
    ///
    /// If you have Euler angles, you can use [`Euler::into`].
    pub fn rotate(object: &Rc<Self>, q: &Quaternion) {
        object.local_matrix.borrow_mut().rotate(q);
        Self::update_world_matrix(object, true);
    }

    /// Scales the given 3D object and updates it descendants' world matrices.
    pub fn scale(object: &Rc<Self>, v: &Vector3) {
        object.local_matrix.borrow_mut().scale(v);
        Self::update_world_matrix(object, true);
    }

    /// Updates the world matrix of the given object and (if specified) the
    /// world matrices of its descendants.
    pub fn update_world_matrix(object: &Rc<Self>, update_descendants: bool) {
        if !update_descendants {
            if let Some(parent) = object.parent.borrow().upgrade() {
                *object.world_matrix.borrow_mut() = parent
                    .world_matrix
                    .borrow()
                    .mul(*object.local_matrix.borrow())
            };

            if let Object3dKind::Camera(camera) = &object.kind {
                camera.borrow_mut().view_matrix = object.world_matrix.borrow().inverse();

                camera.borrow_mut().position = object.world_matrix.borrow().translation();
            }
        } else {
            for object in Self::bfs(object) {
                Self::update_world_matrix(&object, false);
            }
        }
    }
}

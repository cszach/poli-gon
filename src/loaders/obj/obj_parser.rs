use std::{cell::RefCell, collections::HashSet, ops::RangeInclusive, rc::Rc, str::Split};

use crate::{
    core::{Mesh, Object3d, Object3dKind::Group},
    geometries::BufferGeometry,
};

/// Successful OBJ file parse result.
pub struct ObjParseResult {
    /// The group of parsed objects from the OBJ file.
    pub group: Rc<Object3d>,
    /// The texture vertex reference numbers that were invalid and for which the
    /// parser performed the default action i.e. added UVs of `0.0`.
    pub default_uvs: HashSet<i32>,
    /// The vertex normal reference numbers that were invalid and for which the
    /// parser performed the default action i.e. added face normals.
    pub default_normals: HashSet<i32>,
}

/// Contains OBJ parse errors.
pub enum ObjParseError {
    /// Invalid syntax error.
    InvalidSyntax {
        /// Line number.
        line_num: usize,
        /// Expected number of arguments (not including the command).
        expected_num_args: RangeInclusive<u32>,
        /// Expected type for the argument(s).
        expected_type: String,
    },
    /// The command is in the OBJ specification, but is not supported by the
    /// parser (yet).
    UnsupportedCommand {
        /// Line number.
        line_num: usize,
        /// Command name.
        command: String,
    },
    /// Invalid reference number used in e.g. `f` command.
    InvalidReferenceNumber {
        /// Line number.
        line_num: usize,
        /// Data type (`v`, `vt`, or `vn`) the reference number was for.
        data_type: String,
        /// Invalid reference number.
        reference_number: i32,
    },
}

/// Subset of [`BufferGeometry`] relevant to OBJ objects.
struct ObjGeometry {
    position: Vec<f32>,
    normal: Vec<f32>,
    uv: Vec<f32>,
}

impl ObjGeometry {
    fn new() -> Self {
        Self {
            position: Vec::new(),
            normal: Vec::new(),
            uv: Vec::new(),
        }
    }
}

/// Subset of [`Object3d`] relevant to OBJ objects.
struct ObjObject {
    name: Option<String>,
    /// `true` if the object is declared through the `g` or `o` command. An OBJ
    /// file may start declaring faces without having the `g` or `o` command
    /// previously, in which case the faces will belong to a default object,
    /// where this value is set to `false`.
    from_declaration: bool,
    geometry: ObjGeometry,
}

impl ObjObject {
    fn new(name: Option<String>, from_declaration: bool) -> Self {
        Self {
            name,
            from_declaration,
            geometry: ObjGeometry::new(),
        }
    }

    fn finalize(&mut self) {}
}

impl Default for ObjObject {
    fn default() -> Self {
        Self {
            name: None,
            from_declaration: false,
            geometry: ObjGeometry::new(),
        }
    }
}

/// Configures the [`ObjParser`]'s behavior.
#[derive(Default)]
pub struct ObjParseOptions {
    /// If `true`, returns an error when an unsupported command is encountered.
    /// If `false`, ignores unsupported commands.
    ///
    /// For a list of supported commands, see the [`ObjParser` documentation](ObjParser).
    pub error_on_unsupported_data_types: bool,
    /// If `true`, invalid texture vertex or vertex normal reference numbers
    /// will throw a parse error. If `false`, they become warnings instead, and
    /// default texture vertices/vertex normals will be used.
    ///
    /// Note that invalid vertex reference numbers will always cause an error.
    pub error_on_invalid_reference_number: bool,
}

/// Parser for ASCII OBJ files.
///
/// ## Supported commands
///
/// - [x] Geometric vertices (`v`)
/// - [x] Texture vertices (`vt`)
/// - [x] Vertex normals (`vn`)
/// - [ ] Parameter space vertices (`vp`)
/// - [ ] Rational or non-rational forms of curve or surface type (`cstype`)
/// - [ ] Degree (`deg`)
/// - [ ] Basis matrix (`bmat`)
/// - [ ] Step size (`step`)
/// - [ ] Point (`p`)
/// - [ ] Line (`l`)
/// - [x] Face (`f`)
/// - [ ] Curve (`curv`)
/// - [ ] 2D curve (`curv2`)
/// - [ ] Surface (`surf`)
/// - [ ] Parameter values (`parm`)
/// - [ ] Outer trimming loop (`trim`)
/// - [ ] Inner trimming loop (`hole`)
/// - [ ] Special curve (`scrv`)
/// - [ ] Special point (`sp`)
/// - [ ] End statement (`end`)
/// - [ ] Connect (`con`)
/// - [x] Group name (`g`)
/// - [ ] Smoothing group (`s`)
/// - [ ] Merging group (`mg`)
/// - [x] Object name (`o`)
/// - [ ] Bevel interpolation (`bevel`)
/// - [ ] Color interpolation (`c_interp`)
/// - [ ] Dissolve interpolation (`d_interp`)
/// - [ ] Level of detail (`lod`)
/// - [ ] Material name (`usemtl`)
/// - [ ] Material library (`mtllib`)
/// - [ ] Shadow casting (`shadow_obj`)
/// - [ ] Ray tracing (`trace_obj`)
/// - [ ] Curve approximation technique (`ctech`)
/// - [ ] Surface approximation technique (`stech`)
pub struct ObjParser {}

struct ObjParseState {
    /// Objects that have been parsed.
    objects: Vec<Rc<RefCell<ObjObject>>>,
    /// Object that the parser is reading data for.
    current_object: Rc<RefCell<ObjObject>>,
    /// Numbers from the vertex command (`v`) added in order.
    vertices: Vec<f32>,
    /// Numbers from the vertex normal command (`vn`) added in order.
    normals: Vec<f32>,
    /// Numbers from the vertex texture command (`vt`) added in order.
    uvs: Vec<f32>,
}

impl ObjParseState {
    /// Creates a new parse state for the parser. Must be created for every file
    /// that will be parsed.
    fn new() -> Self {
        let current_object = Rc::new(RefCell::new(ObjObject::default()));
        let objects = vec![Rc::clone(&current_object)];

        Self {
            objects,
            current_object,
            vertices: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
        }
    }

    /// Creates a new object, adds it to [`objects`](Self::objects), and sets
    /// [`current_object`](Self::current_object) to it.
    fn start_object(&mut self, name: Option<String>, from_declaration: bool) {
        {
            let mut current_object = self.current_object.as_ref().borrow_mut();

            if !current_object.from_declaration {
                current_object.name = name;
                current_object.from_declaration = from_declaration;

                return;
            }

            current_object.finalize();
        }

        let new_object = Rc::new(RefCell::new(ObjObject::new(name, from_declaration)));

        self.objects.push(Rc::clone(&new_object));
        self.current_object = new_object;
    }

    fn finalize(&self) {
        self.current_object.borrow_mut().finalize();
    }

    /// Converts the given vertex reference number to index in
    /// [`vertices`](Self::vertices).
    fn vertex_reference_to_index(&self, reference_number: i32) -> usize {
        3 * if reference_number >= 0 {
            reference_number as usize - 1
        } else {
            (reference_number + self.vertices.len() as i32 / 3) as usize
        }
    }

    /// Converts the given vertex normal reference number to index in
    /// [`normals`](Self::normals).
    fn normal_reference_to_index(&self, reference_number: i32) -> usize {
        3 * if reference_number >= 0 {
            reference_number as usize - 1
        } else {
            (reference_number + self.normals.len() as i32 / 3) as usize
        }
    }

    /// Converts the given texture vertex reference number to index in
    /// [`uvs`](Self::uvs).
    fn uv_reference_to_index(&self, reference_number: i32) -> usize {
        2 * if reference_number >= 0 {
            reference_number as usize - 1
        } else {
            (reference_number + self.uvs.len() as i32 / 2) as usize
        }
    }

    /// Adds three vertices to the current object given their reference numbers.
    ///
    /// ## Returns
    ///
    /// * `Ok(())` if successful.
    /// * `Err(i32)` if there is an invalid reference number, which is included
    ///   in the enum.
    fn add_vertex(&mut self, v1: i32, v2: i32, v3: i32) -> Result<(), i32> {
        let v1_index = self.vertex_reference_to_index(v1);
        let v2_index = self.vertex_reference_to_index(v2);
        let v3_index = self.vertex_reference_to_index(v3);

        let vertices = [
            self.vertices.get(v1_index),
            self.vertices.get(v1_index + 1),
            self.vertices.get(v1_index + 2),
            self.vertices.get(v2_index),
            self.vertices.get(v2_index + 1),
            self.vertices.get(v2_index + 2),
            self.vertices.get(v3_index),
            self.vertices.get(v3_index + 1),
            self.vertices.get(v3_index + 2),
        ];

        let mut current_object = self.current_object.as_ref().borrow_mut();

        for (v, vertex) in vertices.chunks(3).enumerate() {
            if let [Some(x), Some(y), Some(z)] = vertex {
                current_object.geometry.position.extend([*x, *y, *z]);
            } else {
                return match v {
                    0 => Err(v1),
                    1 => Err(v2),
                    _ => Err(v3),
                };
            }
        }

        Ok(())
    }

    /// Adds three vertex normals to the current object given their reference
    /// numbers.
    ///
    /// ## Returns
    ///
    /// * `Ok(())` if successful.
    /// * `Err(i32)` if there is an invalid reference number, which is included
    ///   in the enum.
    fn add_normal(&mut self, vn1: i32, vn2: i32, vn3: i32) -> Result<(), i32> {
        let vn1_index = self.normal_reference_to_index(vn1);
        let vn2_index = self.normal_reference_to_index(vn2);
        let vn3_index = self.normal_reference_to_index(vn3);

        let normals = [
            self.normals.get(vn1_index),
            self.normals.get(vn1_index + 1),
            self.normals.get(vn1_index + 2),
            self.normals.get(vn2_index),
            self.normals.get(vn2_index + 1),
            self.normals.get(vn2_index + 2),
            self.normals.get(vn3_index),
            self.normals.get(vn3_index + 1),
            self.normals.get(vn3_index + 2),
        ];

        let mut new_normals: Vec<f32> = Vec::with_capacity(9);

        for (vn, normal) in normals.chunks(3).enumerate() {
            if let [Some(x), Some(y), Some(z)] = normal {
                new_normals.extend([*x, *y, *z]);
            } else {
                return match vn {
                    0 => Err(vn1),
                    1 => Err(vn2),
                    _ => Err(vn3),
                };
            }
        }

        self.current_object
            .as_ref()
            .borrow_mut()
            .geometry
            .normal
            .extend(new_normals);

        Ok(())
    }

    /// Adds three texture vertices to the current object given their reference
    /// numbers.
    ///
    /// ## Returns
    ///
    /// * `Ok(())` if successful.
    /// * `Err(i32)` if there is an invalid reference number, which is included
    ///   in the enum.
    fn add_uv(&mut self, vt1: i32, vt2: i32, vt3: i32) -> Result<(), i32> {
        let vt1_index = self.uv_reference_to_index(vt1);
        let vt2_index = self.uv_reference_to_index(vt2);
        let vt3_index = self.uv_reference_to_index(vt3);

        let uvs = [
            self.uvs.get(vt1_index),
            self.uvs.get(vt1_index + 1),
            self.uvs.get(vt2_index),
            self.uvs.get(vt2_index + 1),
            self.uvs.get(vt3_index),
            self.uvs.get(vt3_index + 1),
        ];

        let mut new_uvs: Vec<f32> = Vec::with_capacity(6);

        for (vt, uv) in uvs.chunks(2).enumerate() {
            if let [Some(u), Some(v)] = uv {
                new_uvs.extend([*u, *v]);
            } else {
                return match vt {
                    0 => Err(vt1),
                    1 => Err(vt2),
                    _ => Err(vt3),
                };
            }
        }

        self.current_object
            .as_ref()
            .borrow_mut()
            .geometry
            .uv
            .extend(new_uvs);

        Ok(())
    }

    /// Adds default UVs to the current object for 3 vertices.
    fn add_default_uv(&mut self) {
        self.current_object
            .as_ref()
            .borrow_mut()
            .geometry
            .uv
            .extend([0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    }

    /// Adds a triangular face to the current object, given triplets of
    /// vertices, optional texture vertices, and optional vertex normals.
    fn add_face(
        &mut self,
        v: (i32, i32, i32),
        vt: Option<(i32, i32, i32)>,
        vn: Option<(i32, i32, i32)>,
        error_on_invalid_reference_number: bool,
    ) -> Result<(Option<i32>, Option<i32>), (String, i32)> {
        if let Err(reference_number) = self.add_vertex(v.0, v.1, v.2) {
            return Err((String::from("v"), reference_number));
        }

        let mut erroneous_vt = None;
        let mut erroneous_vn = None;

        if let Some(vt) = vt {
            if let Err(reference_number) = self.add_uv(vt.0, vt.1, vt.2) {
                match error_on_invalid_reference_number {
                    true => {
                        return Err((String::from("vt"), reference_number));
                    }
                    false => {
                        erroneous_vt = Some(reference_number);
                        self.add_default_uv();
                    }
                };
            }
        } else {
            self.add_default_uv();
        }

        if let Some(vn) = vn {
            if let Err(reference_number) = self.add_normal(vn.0, vn.1, vn.2) {
                match error_on_invalid_reference_number {
                    true => {
                        return Err((String::from("vn"), reference_number));
                    }
                    false => {
                        erroneous_vn = Some(reference_number);
                        todo!("Add face normal if there is an error parsing vertex normals.");
                    }
                };
            }
        } else {
            todo!("Add face normal if vertex normals are not provided.")
        }

        Ok((erroneous_vt, erroneous_vn))
    }
}

impl ObjParser {
    /// Parses a string slice into an `f32`. Used for parsing vertex data.
    fn parse_f32(s: &str) -> Option<f32> {
        s.parse::<f32>().ok()
    }

    /// Parses a string slice into an `i32`. Used for parsing reference numbers.
    fn parse_i32(s: &str) -> Option<i32> {
        s.parse::<i32>().ok()
    }

    /// Parses the content of an OBJ file. A list of supported commands can be
    /// found in the [`Object3d` documentation](Object3d).
    ///
    /// More specifically, the parser follows the ASCII OBJ 3.0 format, based on
    /// Appendix B1 from the manual for Wavefront's Advanced Visualizer
    /// software. A copy is hosted on [Paul Bourke's page][obj].
    ///
    /// Parsing is optimistic, which means invalid syntax that is recoverable or
    /// does not affect object construction is ignored. For example:
    /// * If a command has more arguments than specified in the OBJ
    ///   specification, the extra arguments will be ignored.
    /// * If an optional value cannot be parsed to a number, its default value
    ///   will be used.
    /// * If one of the `vt` or `vn` values in
    ///   `f v1/vt1/vn1 v2/vt2/vn2 v3/vt3/vn3 ...` could not be parsed, it will
    ///   be treated as if all `vt` values were not specified.
    /// * If an `f` command does not have enough arguments, it will be ignored.
    ///   Note that the same does not apply for `v`, `vt`, or `vn`, because they
    ///   are needed for reference numbers to work.
    ///
    /// ## Parameters
    ///
    /// * `text`: The content of the OBJ file. Can be included using e.g. the
    ///   [`include_str!`] macro.
    /// * `options`: Optional parser configuration. Will use sensible defaults
    ///   if `None`.
    ///
    /// ## Returns
    ///
    /// [`Result`] that contains the [`ObjParseResult`], or the first parse
    /// error.
    ///
    /// [obj]: https://paulbourke.net/dataformats/obj/
    pub fn parse(
        text: &str,
        options: Option<ObjParseOptions>,
    ) -> Result<ObjParseResult, ObjParseError> {
        let options = options.unwrap_or_default();

        let mut state = ObjParseState::new();
        let mut default_uvs = HashSet::new();
        let mut default_normals = HashSet::new();

        for (line_num, line) in text.lines().enumerate() {
            let line_num = line_num + 1;

            let mut parts = line.split_whitespace();

            let Some(command) = parts.next() else {
                continue; // skip empty lines
            };

            match command {
                "v" => {
                    let x = parts.next().and_then(Self::parse_f32);
                    let y = parts.next().and_then(Self::parse_f32);
                    let z = parts.next().and_then(Self::parse_f32);
                    let _w = parts.next().and_then(Self::parse_f32).unwrap_or(1.0);

                    if let (Some(x), Some(y), Some(z)) = (x, y, z) {
                        state.vertices.extend([x, y, z]);
                    } else {
                        return Err(ObjParseError::InvalidSyntax {
                            line_num,
                            expected_num_args: 3..=4,
                            expected_type: String::from("f32"),
                        });
                    }
                }
                "vn" => {
                    let i = parts.next().and_then(Self::parse_f32);
                    let j = parts.next().and_then(Self::parse_f32);
                    let k = parts.next().and_then(Self::parse_f32);

                    if let (Some(i), Some(j), Some(k)) = (i, j, k) {
                        state.normals.extend([i, j, k]);
                    } else {
                        return Err(ObjParseError::InvalidSyntax {
                            line_num,
                            expected_num_args: 3..=3,
                            expected_type: String::from("f32"),
                        });
                    }
                }
                "vt" => {
                    let u = parts.next().and_then(Self::parse_f32);
                    let v = parts.next().and_then(Self::parse_f32);
                    let _w = parts.next().and_then(Self::parse_f32).unwrap_or(0.0);

                    if let (Some(u), Some(v)) = (u, v) {
                        state.uvs.extend([u, v]);
                    } else {
                        return Err(ObjParseError::InvalidSyntax {
                            line_num,
                            expected_num_args: 2..=3,
                            expected_type: String::from("f32"),
                        });
                    }
                }
                "f" => {
                    let triplets: Vec<Split<_>> = parts.map(|vertex| vertex.split('/')).collect();

                    let Some(a) = triplets.first() else {
                        continue; // skip empty face
                    };

                    // A face in OBJ may have multiple vertices, but our library
                    // uses triangular polygon mesh. Here, we are decomposing
                    // the OBJ face into triangles by connecting the first
                    // vertex (`a`) to every adjacent pair of vertices.

                    for window in triplets[1..].windows(2) {
                        let [b, c] = window else {
                            break;
                        };

                        let mut a = a.clone();
                        let mut b = b.clone();
                        let mut c = c.clone();

                        let [v, vt, vn] = [0; 3].map(|_| {
                            a.next()
                                .and_then(Self::parse_i32)
                                .zip(b.next().and_then(Self::parse_i32))
                                .zip(c.next().and_then(Self::parse_i32))
                                .map(|((x, y), z)| (x, y, z))
                        });

                        if let Some(v) = v {
                            match state.add_face(
                                v,
                                vt,
                                vn,
                                options.error_on_invalid_reference_number,
                            ) {
                                Ok(warn) => {
                                    if let Some(vt) = warn.0 {
                                        default_uvs.insert(vt);
                                    }

                                    if let Some(vn) = warn.1 {
                                        default_normals.insert(vn);
                                    }
                                }
                                Err((ty, reference_number)) => {
                                    return Err(ObjParseError::InvalidReferenceNumber {
                                        line_num,
                                        data_type: ty,
                                        reference_number,
                                    });
                                }
                            };
                        };
                    }
                }
                "o" | "g" => {
                    let name = parts.next().map(|s| s.to_string());
                    state.start_object(name, true);
                }
                other => {
                    if options.error_on_unsupported_data_types {
                        return Err(ObjParseError::UnsupportedCommand {
                            line_num,
                            command: other.to_string(),
                        });
                    }
                }
            }
        }

        state.finalize();

        let group = Rc::new(Object3d::new(Group));

        state.objects.iter().for_each(|object| {
            let object = object.take();

            // Skip groups/objects that do not have any faces.
            if object.geometry.position.is_empty() {
                return;
            }

            let buffer_geometry = BufferGeometry {
                position: object.geometry.position,
                normal: object.geometry.normal,
                uv: object.geometry.uv,
                indices: None,
            };

            let mut object_3d: Object3d = Mesh::new(Rc::new(buffer_geometry)).into();
            object_3d.name = RefCell::new(object.name);

            let mesh = Rc::new(object_3d);
            Object3d::add(&group, &mesh);
        });

        Ok(ObjParseResult {
            group,
            default_uvs,
            default_normals,
        })
    }
}

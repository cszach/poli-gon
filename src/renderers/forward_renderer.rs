use std::{mem::size_of, num::NonZero, rc::Rc};

use poli_math::{Color, Matrix3, Matrix4, Vector3};
use wgpu::{include_wgsl, VertexAttribute};
use winit::window::Window;

use crate::{
    core::{Camera, Object3d, Object3dKind},
    wgpual::{Gpu, GpuOptions},
    PowerPreference,
};

/// Forward renderer.
///
/// The forward renderer renders each object in a separate draw call.
pub struct ForwardRenderer {
    /// Contains various GPU objects used by this renderer.
    pub state: Gpu,
    /// The clear color to use for the clear operation.
    pub clear_color: Color,
    /// The clear alpha to use for the clear operation.
    pub clear_alpha: f64,

    position_buffer: wgpu::Buffer,
    normal_buffer: wgpu::Buffer,
    uv_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    depth_texture: wgpu::Texture,
    model_matrix_buffer: wgpu::Buffer,
    model_view_matrix_buffer: wgpu::Buffer,
    projection_matrix_buffer: wgpu::Buffer,
    view_matrix_buffer: wgpu::Buffer,
    normal_matrix_buffer: wgpu::Buffer,
    camera_position_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}

/// Defines the [`ForwardRenderer`](#ForwardRenderer)'s behavior.
pub struct ForwardRendererOptions {
    /// See [`GpuOptions#power_preference`].
    pub power_preference: PowerPreference,
    /// Controls the default clear alpha value. When set to `true`, the value is
    /// `0.0`. Otherwise it's `1.0`. Default is `false`.
    pub alpha: bool,
}

impl Default for ForwardRendererOptions {
    /// Returns the default forward renderer parameters.
    fn default() -> Self {
        Self {
            power_preference: PowerPreference::None,
            alpha: false,
        }
    }
}

impl ForwardRenderer {
    /// The maximum number of meshes that this renderer can render.
    pub const MESH_CAPACITY: u64 = 1024;
    /// The maximum number of vertices that a mesh can have to be rendered properly.
    pub const VERTEX_CAPACITY: u64 = 1 << 19; // 524,288
    /// The maximum number of triangular polygons an indexed mesh can have to be rendered properly.
    pub const POLYGON_CAPACITY: u64 = 1 << 19; // 524,288
    /// The minimum buffer offset alignment as defined in [the WebGPU spec](
    /// https://www.w3.org/TR/webgpu/#dom-supported-limits-minuniformbufferoffsetalignment).
    const OFFSET: u64 = 256;

    /// Creates a new forward renderer.
    ///
    /// * `window`: The window for the renderer to draw on.
    /// * `options`: Parameters for the new renderer.
    pub async fn new(window: &Rc<Window>, options: ForwardRendererOptions) -> Self {
        let state = Gpu::new(GpuOptions {
            window: Rc::clone(window),
            power_preference: options.power_preference,
        })
        .await
        .unwrap();

        let depth_texture = state.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: window.inner_size().width,
                height: window.inner_size().height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                .union(wgpu::TextureUsages::TEXTURE_BINDING),
            view_formats: &[],
        });

        let bind_group_layout =
            state
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        // Model matrix
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Model-view matrix
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Projection matrix
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // View matrix
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Normal matrix
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Camera position
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let position_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: 3 * 4,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 1,
            }],
        };

        let position_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Position buffer"),
            size: Self::OFFSET * (Self::VERTEX_CAPACITY - 1) + (3 * 4),
            usage: wgpu::BufferUsages::VERTEX.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });

        let normal_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: 3 * 4,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 2,
            }],
        };

        let normal_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Normal buffer"),
            size: Self::OFFSET * (Self::VERTEX_CAPACITY - 1) + (2 * 4),
            usage: wgpu::BufferUsages::VERTEX.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });

        let uv_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: 2 * 4,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 3,
            }],
        };

        let uv_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UV buffer"),
            size: Self::OFFSET * (Self::VERTEX_CAPACITY - 1) + (2 * 4),
            usage: wgpu::BufferUsages::VERTEX.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });

        let index_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index buffer"),
            size: Self::OFFSET * (Self::POLYGON_CAPACITY - 1) + 4 * 4,
            usage: wgpu::BufferUsages::INDEX.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });

        let model_matrix_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Model matrix buffer"),
            size: Self::OFFSET * (Self::MESH_CAPACITY - 1) + size_of::<Matrix4>() as u64,
            usage: wgpu::BufferUsages::UNIFORM.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });

        let model_view_matrix_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Model view matrix buffer"),
            size: Self::OFFSET * (Self::MESH_CAPACITY - 1) + size_of::<Matrix4>() as u64,
            usage: wgpu::BufferUsages::UNIFORM.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });

        let projection_matrix_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Projection matrix buffer"),
            size: size_of::<Matrix4>() as u64,
            usage: wgpu::BufferUsages::UNIFORM.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });

        let view_matrix_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("View matrix buffer"),
            size: size_of::<Matrix4>() as u64,
            usage: wgpu::BufferUsages::UNIFORM.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });

        let normal_matrix_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Normal matrix buffer"),
            size: Self::OFFSET * (Self::MESH_CAPACITY - 1) + size_of::<Matrix3>() as u64,
            usage: wgpu::BufferUsages::UNIFORM.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });

        let camera_position_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera position buffer"),
            size: size_of::<Vector3>() as u64,
            usage: wgpu::BufferUsages::UNIFORM.union(wgpu::BufferUsages::COPY_DST),
            mapped_at_creation: false,
        });

        let pipeline_layout =
            state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let module = state
            .device
            .create_shader_module(include_wgsl!("shaders/forward_renderer.wgsl"));

        let pipeline = state
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: "vertexMain",
                    compilation_options: Default::default(),
                    buffers: &[
                        position_buffer_layout,
                        normal_buffer_layout,
                        uv_buffer_layout,
                    ],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: "fragmentMain",
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: state.surface_configuration.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    strip_index_format: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        Self {
            depth_texture,
            state,
            clear_color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            },
            clear_alpha: options.alpha.into(),
            position_buffer,
            normal_buffer,
            uv_buffer,
            index_buffer,
            model_matrix_buffer,
            model_view_matrix_buffer,
            projection_matrix_buffer,
            view_matrix_buffer,
            normal_matrix_buffer,
            camera_position_buffer,
            bind_group_layout,
            pipeline,
        }
    }

    /// Reconfigures the renderer to render to the specified size. Note that
    /// this does not resize the window.
    pub fn set_size(&mut self, width: u32, height: u32) {
        self.state.set_size(width, height);
        self.depth_texture = self.state.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
    }

    /// Renders the specified scene as viewed from the specified camera to the
    /// window.
    pub fn render(
        &mut self,
        scene: Rc<Object3d>,
        camera: &Camera,
    ) -> Result<(), wgpu::SurfaceError> {
        let output = self.state.surface.get_current_texture()?;
        let texture_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.state.queue.write_buffer(
            &self.projection_matrix_buffer,
            0,
            bytemuck::cast_slice(&[camera.projection_matrix]),
        );

        let view_matrix = Matrix4::look_at(
            &camera.position,
            &(0.0, 0.0, 0.0).into(),
            &(0.0, 1.0, 0.0).into(),
        );

        self.state.queue.write_buffer(
            &self.view_matrix_buffer,
            0,
            bytemuck::cast_slice(&[view_matrix]),
        );

        // web_sys::console::log_5(
        //     &"1: ".into(),
        //     &camera.view_matrix.elements[0].into(),
        //     &camera.view_matrix.elements[4].into(),
        //     &camera.view_matrix.elements[8].into(),
        //     &camera.view_matrix.elements[12].into(),
        // );
        //
        // web_sys::console::log_5(
        //     &"2: ".into(),
        //     &camera.view_matrix.elements[1].into(),
        //     &camera.view_matrix.elements[5].into(),
        //     &camera.view_matrix.elements[9].into(),
        //     &camera.view_matrix.elements[13].into(),
        // );
        //
        // web_sys::console::log_5(
        //     &"3: ".into(),
        //     &camera.view_matrix.elements[2].into(),
        //     &camera.view_matrix.elements[6].into(),
        //     &camera.view_matrix.elements[10].into(),
        //     &camera.view_matrix.elements[14].into(),
        // );
        //
        // web_sys::console::log_5(
        //     &"4: ".into(),
        //     &camera.view_matrix.elements[3].into(),
        //     &camera.view_matrix.elements[7].into(),
        //     &camera.view_matrix.elements[11].into(),
        //     &camera.view_matrix.elements[15].into(),
        // );

        self.state.queue.write_buffer(
            &self.camera_position_buffer,
            0,
            bytemuck::cast_slice(&[camera.position]),
        );

        let mut encoder = self
            .state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: self.clear_color.r,
                        g: self.clear_color.g,
                        b: self.clear_color.b,
                        a: self.clear_alpha,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self
                    .depth_texture
                    .create_view(&wgpu::TextureViewDescriptor::default()),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_vertex_buffer(0, self.position_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.normal_buffer.slice(..));
        render_pass.set_vertex_buffer(2, self.uv_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_pipeline(&self.pipeline);

        let mut stack: Vec<Rc<Object3d>> = Vec::new();
        let mut mesh_index = 0;
        let mut index_start = 0;
        let mut base_vertex = 0;

        stack.push(scene);

        while let Some(object) = stack.pop() {
            if !*object.visible.borrow() {
                continue;
            }

            if let Object3dKind::Mesh(mesh) = &object.kind {
                let mut mut_bind_group = mesh.as_ref().bind_group.borrow_mut();

                let bind_group =
                    &*mut_bind_group.get_or_insert(self.state.device.create_bind_group(
                        &wgpu::BindGroupDescriptor {
                            label: None,
                            layout: &self.bind_group_layout,
                            entries: &[
                                // Model matrix
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                        buffer: &self.model_matrix_buffer,
                                        offset: mesh_index * Self::OFFSET,
                                        size: NonZero::new(size_of::<Matrix4>() as u64),
                                    }),
                                },
                                // Model-view matrix
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                        buffer: &self.model_view_matrix_buffer,
                                        offset: mesh_index * Self::OFFSET,
                                        size: NonZero::new(size_of::<Matrix4>() as u64),
                                    }),
                                },
                                // Projection matrix
                                wgpu::BindGroupEntry {
                                    binding: 2,
                                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                        buffer: &self.projection_matrix_buffer,
                                        offset: 0,
                                        size: NonZero::new(size_of::<Matrix4>() as u64),
                                    }),
                                },
                                // View matrix
                                wgpu::BindGroupEntry {
                                    binding: 3,
                                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                        buffer: &self.view_matrix_buffer,
                                        offset: 0,
                                        size: NonZero::new(size_of::<Matrix4>() as u64),
                                    }),
                                },
                                // Normal matrix
                                wgpu::BindGroupEntry {
                                    binding: 4,
                                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                        buffer: &self.normal_matrix_buffer,
                                        offset: mesh_index * Self::OFFSET,
                                        // WebGPU requires 48 bytes minimum.
                                        // TODO: investigate the spec.
                                        size: NonZero::new(size_of::<Matrix4>() as u64),
                                    }),
                                },
                                // Camera position
                                wgpu::BindGroupEntry {
                                    binding: 5,
                                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                        buffer: &self.camera_position_buffer,
                                        offset: 0,
                                        size: NonZero::new(size_of::<Vector3>() as u64),
                                    }),
                                },
                            ],
                        },
                    ));

                self.state.queue.write_buffer(
                    &self.position_buffer,
                    base_vertex * 12,
                    bytemuck::cast_slice(mesh.geometry.position.as_ref()),
                );

                self.state.queue.write_buffer(
                    &self.normal_buffer,
                    base_vertex * 12,
                    bytemuck::cast_slice(mesh.geometry.normal.as_ref()),
                );

                self.state.queue.write_buffer(
                    &self.uv_buffer,
                    base_vertex * 8,
                    bytemuck::cast_slice(mesh.geometry.uv.as_ref()),
                );

                self.state.queue.write_buffer(
                    &self.model_matrix_buffer,
                    mesh_index * Self::OFFSET,
                    bytemuck::cast_slice(&[*object.world_matrix.borrow()]),
                );

                let model_view_matrix = view_matrix * object.world_matrix.borrow().as_ref();
                let normal_matrix: Matrix3 = Matrix3::normal_matrix(&model_view_matrix);

                self.state.queue.write_buffer(
                    &self.model_view_matrix_buffer,
                    mesh_index * Self::OFFSET,
                    bytemuck::cast_slice(&[model_view_matrix]),
                );

                self.state.queue.write_buffer(
                    &self.normal_matrix_buffer,
                    mesh_index * Self::OFFSET,
                    bytemuck::cast_slice(&[normal_matrix]),
                );

                render_pass.set_bind_group(0, bind_group, &[]);

                let num_vertices = mesh.geometry.position.len() as u32 / 3;

                if let Some(indices) = &mesh.geometry.indices {
                    self.state.queue.write_buffer(
                        &self.index_buffer,
                        index_start * size_of::<u32>() as u64,
                        bytemuck::cast_slice(indices.as_ref()),
                    );

                    render_pass.draw_indexed(
                        index_start as u32..index_start as u32 + indices.len() as u32,
                        base_vertex as i32,
                        0..1,
                    );

                    index_start += indices.len() as u64;
                } else {
                    render_pass.draw(base_vertex as u32..base_vertex as u32 + num_vertices, 0..1);
                }

                mesh_index += 1;
                base_vertex += num_vertices as u64;
            }

            for child in object.children.borrow().iter() {
                stack.push(Rc::clone(child));
            }
        }

        drop(render_pass);

        self.state.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

use std::error::Error;

/// Container for several GPU objects used by renderers.
pub struct Gpu<'window> {
    /// The WGPU surface.
    pub surface: wgpu::Surface<'window>,
    /// The WGPU device.
    pub device: wgpu::Device,
    /// The WGPU queue.
    pub queue: wgpu::Queue,
    /// The WGPU surface configuration.
    pub surface_configuration: wgpu::SurfaceConfiguration,
    /// The size of the surface in (logical) pixels.
    pub size: (u32, u32),
}

/// Parameters for when creating a new GPU adapter.
pub struct GpuOptions {
    /// Provides a **hint** to indicate which GPU to use. `LowPower` means to
    /// use an integrated GPU, while `HighPower` means to use a dedicated GPU.
    /// Default is `None` (provides no hint).
    pub power_preference: wgpu::PowerPreference,
}

impl<'window> Gpu<'window> {
    /// Creates a new GPU object with the specified options.
    ///
    /// * `window`: Window to render on.
    /// * `options`: Configuration for the new renderer.
    ///
    /// # Returns
    ///
    /// A [`Future`](std::future::Future) where the new state is returned, or
    /// either a [`CreateSurfaceError`](wgpu::CreateSurfaceError) or
    /// [`RequestDeviceError`](wgpu::RequestDeviceError).
    pub async fn new(
        window: impl Into<wgpu::SurfaceTarget<'window>>,
        options: GpuOptions,
    ) -> Result<Self, Box<dyn Error>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: options.power_preference,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await?;

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(surface_capabilities.formats[0]);

        let size = (300, 150);

        let surface_configuration = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.0,
            height: size.1,
            present_mode: surface_capabilities.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &surface_configuration);

        Ok(Gpu {
            surface,
            device,
            queue,
            surface_configuration,
            size,
        })
    }

    /// Reconfigures the surface to the specified width and height. Note that
    /// this does not resize the surface, and should be called after the surface
    /// has been resized.
    pub fn set_size(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.size.0 = width;
            self.size.1 = height;
            self.surface_configuration.width = width;
            self.surface_configuration.height = height;
            self.surface
                .configure(&self.device, &self.surface_configuration);
        }
    }
}

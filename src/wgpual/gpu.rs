use std::{error::Error, rc::Rc};

use winit::{dpi::LogicalSize, window::Window};

/// Container for several GPU objects used by renderers.
pub struct Gpu {
    /// The WGPU surface.
    pub surface: wgpu::Surface<'static>,
    /// The WGPU device.
    pub device: wgpu::Device,
    /// The WGPU queue.
    pub queue: wgpu::Queue,
    /// The WGPU surface configuration.
    pub surface_configuration: wgpu::SurfaceConfiguration,
    /// The size of the surface in (logical) pixels.
    pub size: winit::dpi::LogicalSize<u32>,
    /// The [`Window`] (which is the application window or the canvas).
    pub window: Rc<Window>,
}

/// Parameters for when creating a new GPU adapter.
pub struct GpuOptions {
    pub window: Rc<Window>,
    /// Provides a **hint** to indicate which GPU to use. `LowPower` means to
    /// use an integrated GPU, while `HighPower` means to use a dedicated GPU.
    /// Default is `None` (provides no hint).
    pub power_preference: wgpu::PowerPreference,
}

impl Gpu {
    /// Creates a new GPU object with the specified options.
    ///
    /// # Returns
    ///
    /// A [`Future`](std::future::Future) where the new state is returned, or
    /// either a [`CreateSurfaceError`](wgpu::CreateSurfaceError) or
    /// [`RequestDeviceError`](wgpu::RequestDeviceError).
    pub async fn new(options: GpuOptions) -> Result<Self, Box<dyn Error>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU.union(wgpu::Backends::GL),
            ..Default::default()
        });

        let surface = instance.create_surface(options.window.clone())?;

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

        let size = LogicalSize::new(300, 150);

        let surface_configuration = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
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
            window: options.window,
        })
    }

    /// Reconfigures the surface to the specified width and height. Note that
    /// this does not resize the surface, and should be called after the surface
    /// has been resized.
    pub fn set_size(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.size.width = width;
            self.size.height = height;
            self.surface_configuration.width = width;
            self.surface_configuration.height = height;
            self.surface
                .configure(&self.device, &self.surface_configuration);
        }
    }
}

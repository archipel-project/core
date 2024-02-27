pub mod camera;
pub mod terrain;
pub mod ui;

use egui_winit::winit;
use tuple_list::{Tuple, TupleList};

pub struct Context {
    pub wgpu_adapter: wgpu::Adapter,
    pub wgpu_device: wgpu::Device,
    pub wgpu_queue: wgpu::Queue,
}

impl Context {
    async fn new(surface: &wgpu::Surface, wgpu_instance: wgpu::Instance) -> anyhow::Result<Self> {
        let adapter = wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(anyhow::anyhow!("No suitable GPU adapters found!"))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;

        Ok(Self {
            wgpu_adapter: adapter,
            wgpu_device: device,
            wgpu_queue: queue,
        })
    }
}

//for now, the depth buffer is in the swapchain object, since it need to be the same size as the swapchain
//this might change in the future...
pub struct Window {
    window: winit::window::Window,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
    depth_buffer: wgpu::Texture,
}

impl Window {
    pub fn new(
        window: winit::window::Window,
        wgpu_instance: wgpu::Instance,
    ) -> anyhow::Result<(Self, Context)> {
        let surface = unsafe { wgpu_instance.create_surface(&window)? };

        let context = pollster::block_on(Context::new(&surface, wgpu_instance))?;
        let window_size = window.inner_size();
        let surface_config = Self::get_surface_configuration(&surface, window_size, &context);
        let depth_buffer = Self::get_depth_buffer(window_size, &context);

        let window = Self {
            window,
            surface,
            surface_config,
            depth_buffer,
        };
        Ok((window, context))
    }

    fn get_surface_configuration(
        surface: &wgpu::Surface,
        size: winit::dpi::PhysicalSize<u32>,
        render_context: &Context,
    ) -> wgpu::SurfaceConfiguration {
        let surface_caps = surface.get_capabilities(&render_context.wgpu_adapter);

        //only using sRGB for now
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: Default::default(),
            view_formats: Vec::new(),
        };
        surface.configure(&render_context.wgpu_device, &config);
        config
    }

    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    fn get_depth_buffer(size: winit::dpi::PhysicalSize<u32>, context: &Context) -> wgpu::Texture {
        let size = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };

        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Buffer"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = context.wgpu_device.create_texture(&desc);
        texture
    }

    pub fn as_winit_window(&self) -> &winit::window::Window {
        &self.window
    }
    pub fn get_surface_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.surface_config
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>, render_context: &Context) {
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        if size.width > 0 && size.height > 0 {
            self.surface
                .configure(&render_context.wgpu_device, &self.surface_config);
            self.depth_buffer = Self::get_depth_buffer(size, render_context);
        }
    }

    pub fn should_be_rendered(&self) -> bool {
        self.surface_config.width > 0 && self.surface_config.height > 0
    }
}

//define a RenderTask
//a RenderJob is a part of the RenderPath
//they should be updated each frame
//they aren't owned by the RenderScheduler,
//the RenderScheduler just calls update draw on them
pub trait RenderJob {
    fn update(&mut self, command_encoder: &mut wgpu::CommandEncoder, render_context: &Context);
    fn draw<'pass>(&'pass mut self, render_pass: &mut wgpu::RenderPass<'pass>);
}

impl RenderJob for () {
    fn update(&mut self, _command_encoder: &mut wgpu::CommandEncoder, _render_context: &Context) {}
    fn draw<'pass>(&'pass mut self, _render_pass: &mut wgpu::RenderPass<'pass>) {}
}

impl<Job> RenderJob for &mut Job
where
    Job: RenderJob,
{
    fn update(&mut self, command_encoder: &mut wgpu::CommandEncoder, render_context: &Context) {
        Job::update(self, command_encoder, render_context);
    }
    fn draw<'pass>(&'pass mut self, render_pass: &mut wgpu::RenderPass<'pass>) {
        Job::draw(self, render_pass);
    }
}

impl<Job, Tail> RenderJob for (Job, Tail)
where
    Self: TupleList,
    Job: RenderJob,
    Tail: RenderJob,
{
    fn update(&mut self, command_encoder: &mut wgpu::CommandEncoder, render_context: &Context) {
        self.0.update(command_encoder, render_context);
        self.1.update(command_encoder, render_context);
    }
    fn draw<'pass>(&'pass mut self, render_pass: &mut wgpu::RenderPass<'pass>) {
        self.0.draw(render_pass);
        self.1.draw(render_pass);
    }
}

//short living renderer,
//update the screen when being dropped
pub struct FrameRenderer<'a> {
    context: &'a Context,
    surface_texture: wgpu::SurfaceTexture,
    output_view: wgpu::TextureView,
    depth_buffer: wgpu::TextureView,
}

impl<'a> FrameRenderer<'a> {
    pub fn new(
        window: &'a Window,
        context: &'a Context,
    ) -> Result<FrameRenderer<'a>, wgpu::SurfaceError> {
        let (surface_texture, output_view) = Self::get_surface_texture(&window.surface)?;
        let depth_buffer = window
            .depth_buffer
            .create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Self {
            context,
            surface_texture,
            output_view,
            depth_buffer,
        })
    }

    fn get_command_encoder(&self) -> wgpu::CommandEncoder {
        self.context
            .wgpu_device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render scheduler encoder"),
            })
    }

    fn get_surface_texture(
        surface: &wgpu::Surface,
    ) -> Result<(wgpu::SurfaceTexture, wgpu::TextureView), wgpu::SurfaceError> {
        let surface_texture = surface.get_current_texture()?;
        let output_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        Ok((surface_texture, output_view))
    }

    pub fn render<T>(self, job_list: T)
    where
        T: Tuple,
        <T as Tuple>::TupleList: RenderJob,
    {
        let mut tuple_list = job_list.into_tuple_list();

        let mut command_encoder = Self::get_command_encoder(&self);
        tuple_list.update(&mut command_encoder, &self.context);

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_buffer,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        tuple_list.draw(&mut render_pass);
        drop(render_pass);

        self.context
            .wgpu_queue
            .submit(std::iter::once(command_encoder.finish()));
        self.surface_texture.present();
    }
}

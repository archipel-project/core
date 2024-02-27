use crate::graphic::Context;
use image::RgbaImage;

//first we need to know all existing textures to create a texture atlas
pub struct TextureAtlasBuilder {
    pub vec: Vec<RgbaImage>,
}

//store all texture blocks in a single texture
//responsible for creating the texture and the bind group
//map block id to texture coordinates //TODO support multiple textures per block
pub struct TextureAtlas {
    _atlas: wgpu::Texture,
    _texture_sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl TextureAtlas {
    fn create_texture(
        block_texture_size: u32,
        block_texture_count: u32,
        context: &Context,
    ) -> wgpu::Texture {
        let texture_size = wgpu::Extent3d {
            width: block_texture_size,
            height: block_texture_size,
            depth_or_array_layers: block_texture_count,
        };

        let texture = context
            .wgpu_device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Texture Atlas"),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb, //because of rgba8
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
        texture
    }

    fn create_sampler(context: &Context) -> wgpu::Sampler {
        context
            .wgpu_device
            .create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Diffuse Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            })
    }

    fn create_bind_group(
        atlas: &wgpu::Texture,
        texture_sampler: &wgpu::Sampler,
        layout: &wgpu::BindGroupLayout,
        context: &Context,
    ) -> wgpu::BindGroup {
        context
            .wgpu_device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Texture Atlas Bind Group"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &atlas.create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(texture_sampler),
                    },
                ],
            })
    }

    pub fn new_exp(
        builder: TextureAtlasBuilder,
        block_texture_size: u32,
        context: &Context,
    ) -> Self {
        let atlas = Self::create_texture(block_texture_size, builder.vec.len() as u32, context);

        let block_texture_size = wgpu::Extent3d {
            width: block_texture_size,
            height: block_texture_size,
            depth_or_array_layers: 1,
        };
        //pos in item to the next texture to copy
        for (i, block_texture) in builder.vec.iter().enumerate() {
            //could be more efficient to use CommandEncoder::write_texture(self) instead, queue create multiple command encoder...
            context.wgpu_queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &atlas,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &block_texture,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * block_texture_size.width),
                    rows_per_image: Some(block_texture_size.height),
                },
                block_texture_size,
            );
        }

        let texture_sampler = Self::create_sampler(context);
        let bind_group_layout = Self::create_bind_group_layout(context);
        let bind_group =
            Self::create_bind_group(&atlas, &texture_sampler, &bind_group_layout, context);

        Self {
            _atlas: atlas,
            _texture_sampler: texture_sampler,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn create_bind_group_layout(context: &Context) -> wgpu::BindGroupLayout {
        context
            .wgpu_device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Atlas Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
    }

    pub fn get_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn get_bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn get_texture_coordinates(&self) -> TextureCoordinates {
        TextureCoordinates {
            x2: 1.0,
            y2: 1.0,
            x1: 0.0,
            y1: 0.0,
        }
    }
}

///x1, y1 is the top left corner, x2, y2 is the bottom right corner
#[derive(Clone, Copy)]
pub struct TextureCoordinates {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

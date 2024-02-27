use egui_winit::winit::event::WindowEvent;
use math::{EulerRot, Mat4, Quat, quat, Vec3};
use wgpu::util::DeviceExt;
use super::Context;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj : [[f32; 4]; 4]
}

pub struct Camera{
    pub pitch : f32,
    pub yaw : f32,
    pub position : Vec3,
    pub fov : f32,
    pub ratio : f32,
    camera_buffer : wgpu::Buffer,
    camera_bind_group : wgpu::BindGroup,
    camera_bind_group_layout : wgpu::BindGroupLayout,
}

impl Camera {
    pub fn new(pitch: f32, yaw: f32, position: Vec3, fov: f32, ratio: f32, context: &Context) -> Self
    {
        let camera_buffer = context.wgpu_device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[CameraUniform {
                    view_proj : [[0.0; 4]; 4]
                }]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout = context.wgpu_device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding : 0,
                        visibility : wgpu::ShaderStages::VERTEX,
                        ty : wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            min_binding_size : None,
                            has_dynamic_offset: false,
                        },
                        count : None,
                    }
                ],
                label: Some("camera_bind_group_layout"),
            }
        );

        let camera_bind_group = context.wgpu_device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &camera_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_buffer.as_entire_binding(),
                    }
                ],
                label: Some("camera_bind_group"),
            }
        );


        Self {
            pitch,
            yaw,
            position,
            fov,
            ratio,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
        }
    }

    pub fn get_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.camera_bind_group_layout
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Resized(size) => {
                self.ratio = size.width as f32 / size.height as f32;
            }
            _ => ()
        }
    }

    fn build_view_proj_matrix(&self) -> CameraUniform {
        //todo: view is really wrong
        let rotation = Quat::from_euler(EulerRot::XYZ, self.pitch, self.yaw, 0.0) * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let view = Mat4::from_quat(rotation)* Mat4::from_translation(-self.position);


        let proj = Mat4::perspective_infinite_rh(self.fov, self.ratio, 0.1);
        let view_proj = proj * view;
        CameraUniform {
            view_proj : view_proj.to_cols_array_2d(),
        }
    }

    pub fn get_bind_group(&self) -> &wgpu::BindGroup {
        &self.camera_bind_group
    }

    pub fn update(&self, render_context: &Context) {
        let camera_buffer = self.build_view_proj_matrix();
        render_context.wgpu_queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera_buffer]));
    }
}
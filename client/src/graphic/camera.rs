use super::Context;
use egui_winit::winit::event::WindowEvent;
use math::aabb::AABB;
use math::positions::EntityPos;
use math::{EulerRot, IVec3, Mat4, Quat, Vec3};
use std::f32::consts::{FRAC_PI_2, PI};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    origin: [i32; 3],
    _padding: i32,
}

pub struct Camera {
    pub pitch: f32,
    pub yaw: f32,
    pub position: EntityPos,
    pub fov: f32,
    pub ratio: f32,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_bind_group_layout: wgpu::BindGroupLayout,
}

impl Camera {
    pub fn new(
        pitch: f32,
        yaw: f32,
        position: EntityPos,
        fov: f32,
        ratio: f32,
        context: &Context,
    ) -> Self {
        let camera_buffer =
            context
                .wgpu_device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Camera Buffer"),
                    contents: bytemuck::cast_slice(&[CameraUniform {
                        view_proj: [[0.0; 4]; 4],
                        origin: [0; 3],
                        _padding: 0,
                    }]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let camera_bind_group_layout =
            context
                .wgpu_device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            min_binding_size: None,
                            has_dynamic_offset: false,
                        },
                        count: None,
                    }],
                    label: Some("camera_bind_group_layout"),
                });

        let camera_bind_group = context
            .wgpu_device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
                label: Some("camera_bind_group"),
            });

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
            _ => (),
        }
    }

    fn build_view_proj_matrix(&self) -> CameraUniform {
        //todo: view is really wrong
        let rotation =
            Quat::from_euler(EulerRot::XYZ, self.pitch, self.yaw, 0.0) * Quat::from_rotation_y(PI);
        let view = Mat4::from_quat(rotation) * Mat4::from_translation(-self.position.relative_pos);

        let proj = Mat4::perspective_infinite_rh(self.fov, self.ratio, 0.1);
        let view_proj = proj * view;
        CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            origin: [
                self.position.chunk_pos.x,
                self.position.chunk_pos.y,
                self.position.chunk_pos.z,
            ],
            _padding: 0,
        }
    }

    pub fn get_bind_group(&self) -> &wgpu::BindGroup {
        &self.camera_bind_group
    }

    pub fn update(&self, render_context: &Context) {
        let camera_buffer = self.build_view_proj_matrix();
        render_context.wgpu_queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_buffer]),
        );
    }

    #[allow(non_snake_case)]
    pub fn get_FOVs(&self) -> (f32, f32) {
        let h_fov = self.fov;
        let v_fov = 2.0 * f32::atan(f32::tan(h_fov * 0.5) * self.ratio);
        (v_fov, h_fov)
    }

    pub fn get_frustum(&self, render_distance: i32) -> CameraFrustum {
        // yaw == 0 <==> looking at z+
        // yaw == -PI/2 <==> looking at x+
        // pitch == PI/2 <==> looking at y-
        // pitch == -PI/2 <==> looking at y

        let rotation = Quat::from_euler(EulerRot::XYZ, -self.pitch, self.yaw, 0.0).inverse();

        let (v_fov, h_fov) = self.get_FOVs();
        const TOLERANCE: f32 = 0.0 * PI / 180.0;

        let x_angle = v_fov * 0.5 + FRAC_PI_2 + TOLERANCE;
        let y_angle = h_fov * 0.5 + FRAC_PI_2 + TOLERANCE;
        let right = Quat::from_rotation_y(-x_angle) * Vec3::Z; //because Z is forward
        let left = Quat::from_rotation_y(x_angle) * Vec3::Z;
        let up = Quat::from_rotation_x(y_angle) * Vec3::Z;
        let down = Quat::from_rotation_x(-y_angle) * Vec3::Z;

        CameraFrustum {
            planes: [
                //todo: get the correct planes and positions
                rotation * right,
                rotation * left,
                rotation * up,
                rotation * down,
            ],
            render_distance,
            origin: self.position,
        }
    }
}

pub struct CameraFrustum {
    planes: [Vec3; 4],
    render_distance: i32,
    origin: EntityPos,
}

impl CameraFrustum {
    pub fn contains(&self, aabb: &AABB) -> bool {
        let is_behind = |normal_plane: Vec3| {
            let corners = aabb.corners();

            for corner in corners {
                let mut vec = corner - self.origin.chunk_pos;
                vec *= 16;
                if normal_plane.dot(vec.as_vec3() - self.origin.relative_pos) <= 0.0 {
                    return true;
                }
            }
            false
        };

        for plane in self.planes {
            if !is_behind(plane) {
                return false;
            }
        }
        return true;
    }

    pub fn totally_contains(&self, aabb: &AABB) -> bool {
        let corners = aabb.corners();
        for corner in corners {
            let vec = corner - self.origin.chunk_pos;
            for plane in self.planes {
                if plane.dot(vec.as_vec3()) < 0.0 {
                    return false;
                }
            }
        }
        return true;
    }

    pub fn get_aabb(&self) -> AABB {
        //todo: improve this to be way more accurate/aggressive
        let min = self.origin.chunk_pos
            - IVec3::new(
                self.render_distance,
                self.render_distance,
                self.render_distance,
            );
        let max = self.origin.chunk_pos
            + IVec3::new(
                self.render_distance,
                self.render_distance,
                self.render_distance,
            );
        AABB::new(min, max)
    }
}

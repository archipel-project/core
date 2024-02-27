use std::time::{Duration, Instant};
use egui_winit::winit::window::WindowBuilder;
use egui_winit::winit::event::{DeviceEvent, ElementState, Event, RawKeyEvent, WindowEvent};
use egui_winit::winit::event_loop::{EventLoop, EventLoopWindowTarget};
use egui_winit::winit::keyboard::{KeyCode, PhysicalKey};
use math::{vec3, Vec3};
use rand::Rng;
use world_core::{ChunkManager, ChunkPos, BlockPos, Chunk, MEMORY_MANAGER};
use crate::graphic;
use crate::graphic::FrameRenderer;
use crate::graphic::ui::GUIWrapper;
use crate::networking::ClientNetworkHandler;

fn main_menu(gui_wrapper: &mut GUIWrapper<GUIData>, ctx: &egui::Context, data: &mut GUIData) {
        egui::Window::new("Tool box").show(ctx, |ui| {
            let fps = 1.0 / data.second_per_frame;

            ui.label(format!("fps: {:.2}", fps));
            ui.label(format!("used memory: {} bytes", data.used_memory));
            ui.label(format!("pre-allocated memory: {} bytes", data.pre_allocated_memory));
            if ui.button("more options").clicked() {
                gui_wrapper.set_gui(other_gui);
            }
        });
}

fn other_gui(gui_wrapper: &mut GUIWrapper<GUIData>, ctx: &egui::Context, guidata: &mut GUIData) {
        egui::Window::new("Options").show(ctx, |ui| {
            ui.label("world options");
            if ui.button("regenerate cube").clicked() {
                guidata.regenerate = true;
            }

            if ui.button("back").clicked() {
                gui_wrapper.set_gui(main_menu);
            }
        });
}

struct GUIData {
    second_per_frame: f32,
    regenerate: bool,
    used_memory: usize,
    pre_allocated_memory: usize,
}

struct CameraController {
    is_front_pressed: bool,
    is_back_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_up_pressed: bool,
    is_down_pressed: bool,
    mouse_x: f64,
    mouse_y: f64,
}

impl CameraController {
    fn new() -> Self {
        Self {
            is_front_pressed: false,
            is_back_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_up_pressed: false,
            is_down_pressed: false,
            mouse_x: 0.0,
            mouse_y: 0.0,
        }
    }

    pub fn process_device_event(&mut self, event : DeviceEvent) {
        match event {
            DeviceEvent::Key(raw_key) => {
                self.input(&raw_key);
            }
            DeviceEvent::MouseMotion{delta} => {
                self.mouse_input(delta);
            }
            _ => (),
        }
    }

    fn input(&mut self, raw_key: &RawKeyEvent) {
        let is_pressed = raw_key.state == ElementState::Pressed;
        match raw_key.physical_key{
            PhysicalKey::Code(keycode) => match keycode {
                KeyCode::KeyW => self.is_front_pressed = is_pressed,
                KeyCode::KeyS => self.is_back_pressed = is_pressed,
                KeyCode::KeyA => self.is_left_pressed = is_pressed,
                KeyCode::KeyD => self.is_right_pressed = is_pressed,
                KeyCode::Space => self.is_up_pressed = is_pressed,
                KeyCode::ShiftLeft => self.is_down_pressed = is_pressed,
                _ => (),
            }
            _ => (),
        }
    }

    fn mouse_input(&mut self, delta: (f64, f64)) {
        self.mouse_x += delta.0;
        self.mouse_y += delta.1;
    }

    fn update_camera(&mut self, camera : &mut graphic::camera::Camera, delta_time: Duration) {
        //update camera yaw and pitch
        camera.yaw += self.mouse_x as f32 * 0.0025;

        camera.pitch += self.mouse_y as f32 * 0.0025;

        camera.pitch =  camera.pitch.clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
        camera.yaw %= std::f32::consts::PI * 2.0;
        self.mouse_x = 0.0;
        self.mouse_y = 0.0;


        let speed = 40.0; //m/s
        let delta_time = delta_time.as_secs_f32();
        let mut direction = Vec3::ZERO;
        if self.is_front_pressed {
            direction += Vec3::new(camera.yaw.cos(), 0.0, camera.yaw.sin());
        }
        if self.is_back_pressed {
            direction += Vec3::new(-camera.yaw.cos(), 0.0, -camera.yaw.sin());
        }
        if self.is_left_pressed {
            direction += Vec3::new(camera.yaw.sin(), 0.0, -camera.yaw.cos());
        }
        if self.is_right_pressed {
            direction += Vec3::new(-camera.yaw.sin(), 0.0, camera.yaw.cos());
        }

        if self.is_up_pressed {
            direction += Vec3::Y;
        }
        if self.is_down_pressed {
            direction -= Vec3::Y;
        }
        camera.position += direction.normalize_or_zero() * speed * delta_time;

    }
}

pub struct App {
    window: graphic::Window,
    graphic_context: graphic::Context,
    client_network_handler: Option<ClientNetworkHandler>,
    last_update: Instant,
    gui_handler: graphic::ui::GuiHandler<GUIData>,
    camera: graphic::camera::Camera,
    terrain_renderer: graphic::terrain::TerrainRenderer,
    camera_controller: CameraController,
    chunk_manager: ChunkManager,
}

impl App {

    fn regenerate_cube(chunk_manager: &mut ChunkManager) {


        //make a platform
        let mut build_platform = |x: i32, z: i32, y:i32, block: u16| {
            let mut rng = rand::thread_rng();
            let mut chunk = Chunk::new(ChunkPos::new(x, y, z));
            for ix in 0..16 {
                for iz in 0..16 {
                    for iy in 0..3 + rng.gen_range(0..3) {
                        chunk.set_block(BlockPos::new(ix, iy, iz), block);
                    }
                }
            }
            chunk_manager.insert_chunk(chunk);
        };


        for x in -20..20 {
            for z in -20..20 {
                for y in 0..1 {
                    let mut rng = rand::thread_rng();
                    build_platform(x, z, y, rng.gen_range(1..10));
                }
            }
        }
    }
    pub fn new() -> anyhow::Result<(Self, EventLoop<()>)>  {

        let event_loop = EventLoop::new()?;
        let window = WindowBuilder::new()
            .with_title("my super minecraft a bit empty")
            .build(&event_loop)?;

        let ratio = window.inner_size().width as f32 / window.inner_size().height as f32;

        let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let (window, graphic_context) = graphic::Window::new(window, wgpu_instance)?;

        let mut gui_handler = graphic::ui::GuiHandler::new(&window, &graphic_context);
        gui_handler.set_gui(main_menu);


        let camera = graphic::camera::Camera::new(0.0,
                                                  0.0,
                                                  vec3(0.0, 0.0, 2.0),
                                                  90.0 * std::f32::consts::PI / 180.0,
                                                  ratio,
                                                  &graphic_context);


        //todo: move this to a better place, when the network will be implemented
        let mut chunk_manager = ChunkManager::new();
        Self::regenerate_cube(&mut chunk_manager);

        let terrain_renderer = graphic::terrain::TerrainRenderer::new(&camera, 8, &chunk_manager, &graphic_context);

        Ok((Self {
            window,
            graphic_context,
            client_network_handler: None,
            last_update: Instant::now(),
            gui_handler,
            camera,
            terrain_renderer,
            camera_controller: CameraController::new(),
            chunk_manager,
        },
        event_loop))
    }

    pub fn run(mut self, event_loop: EventLoop<()>) -> anyhow::Result<()> {

        event_loop.run(|event, elwt| {
            match event {
                Event::WindowEvent{event, ..} => self.process_window_event(event, &elwt),
                Event::DeviceEvent{event, ..} => self.camera_controller.process_device_event(event),
                Event::AboutToWait => {
                    self.window.as_winit_window().request_redraw()
                },
                Event::LoopExiting => self.exit(),
                _ => (),
            }
        })?;

        Ok(())
    }

    fn process_window_event(&mut self, event: WindowEvent, elwt: &EventLoopWindowTarget<()>) {
        self.camera.handle_window_event(&event);
        if self.gui_handler.handle_window_event(&event, &self.window) { return; }

        match event {
            WindowEvent::CloseRequested => {
                elwt.exit();
            },
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta_time = now - self.last_update;
                self.last_update = now;

                self.tick(delta_time).unwrap_or_else(|e| {
                        println!("error while ticking: {:?}", e.to_string());
                        elwt.exit();
                });
            },
            WindowEvent::Resized(size) => {
                self.window.resize(size, &self.graphic_context);
            },
            _ => (),
        }
    }

    fn exit(&mut self) {
        println!("exiting");
        if self.client_network_handler.is_some() {
            self.client_network_handler.as_mut().unwrap().exit();
        }
    }

    fn tick(&mut self, delta_time: Duration) -> anyhow::Result<()> {
        if self.client_network_handler.is_some() {
            self.client_network_handler.as_mut().unwrap().tick(delta_time)?;
        }

        let used_memory = MEMORY_MANAGER.stats();
        let mut gui_data = GUIData {
            second_per_frame: delta_time.as_secs_f32(),
            regenerate: false,
            used_memory: used_memory.0,
            pre_allocated_memory: used_memory.1,
        };

        self.camera_controller.update_camera(&mut self.camera, delta_time);
        self.gui_handler.update_gui(&self.window, &self.graphic_context, &mut gui_data);

        if gui_data.regenerate {
            Self::regenerate_cube(&mut self.chunk_manager);
        }

        if self.window.should_be_rendered() {
            self.redraw()?;
        }
        Ok(())
    }

    fn redraw(&mut self) -> anyhow::Result<()>
    {
        self.camera.update(&self.graphic_context);
        let renderer = FrameRenderer::new(&self.window, &self.graphic_context)?;
        let render_jobs = (self.terrain_renderer.build_render_job(&mut self.chunk_manager, &self.camera, &self.graphic_context), &mut self.gui_handler);
        renderer.render(render_jobs);
        Ok(())
    }
}
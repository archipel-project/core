use crate::graphic::RenderJob;
use egui::{ClippedPrimitive, ViewportInfo};
use egui_wgpu::renderer::ScreenDescriptor;
use egui_winit::winit::event::WindowEvent;

type GUIPointer<DataObject> = fn(&mut GUIWrapper<DataObject>, &egui::Context, &mut DataObject);

pub struct GUIWrapper<DataObject> {
    current_gui: GUIPointer<DataObject>,
}

impl<DataObject> From<GUIPointer<DataObject>> for GUIWrapper<DataObject> {
    fn from(value: GUIPointer<DataObject>) -> Self {
        Self { current_gui: value }
    }
}

impl<DataObject> Default for GUIWrapper<DataObject> {
    fn default() -> Self {
        Self {
            current_gui: |_, _, _| {},
        }
    }
}

impl<DataObject> GUIWrapper<DataObject> {
    pub fn set_gui(&mut self, gui: GUIPointer<DataObject>) {
        self.current_gui = gui;
    }

    fn current_gui(&mut self, ctx: &egui::Context, data_object: &mut DataObject) {
        (self.current_gui)(self, ctx, data_object);
    }
}

//could be improved by drawing the GUI into a texture when the gui is updated and then drawing the texture on the screen
//this would allow to draw the GUI only when it is updated
//but this mean using a new texture and resizing it when the window is resized

///DataObject is an object modified by the GUI, it must be used to get the GUI entries, can be the main app as reference
pub struct GuiHandler<DataObject> {
    context: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    gui_pointer: GUIWrapper<DataObject>,
    draw_data: Option<DrawData>,
}

pub struct DrawData {
    clipped_primitives: Vec<ClippedPrimitive>,
    screen_descriptor: ScreenDescriptor,
}

impl<DataObject> GuiHandler<DataObject> {
    pub fn new(window: &super::Window, graphic_context: &super::Context) -> Self {
        let context = egui::Context::default();
        let state = egui_winit::State::new(
            context.clone(),
            context.viewport_id(),
            window.as_winit_window(),
            None,
            None,
        );
        let renderer = egui_wgpu::Renderer::new(
            &graphic_context.wgpu_device,
            window.get_surface_config().format,
            Some(super::Window::DEPTH_FORMAT),
            1,
        );

        Self {
            context,
            state,
            renderer,
            gui_pointer: GUIWrapper::default(),
            draw_data: None,
        }
    }

    pub fn set_gui(&mut self, gui: GUIPointer<DataObject>) {
        self.gui_pointer.set_gui(gui);
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent, window: &super::Window) -> bool {
        let response = self.state.on_window_event(window.as_winit_window(), event);
        response.consumed
    }

    pub fn update_gui(
        &mut self,
        window: &super::Window,
        graphic_context: &super::Context,
        data_object: &mut DataObject,
    ) {
        let surface_config = window.get_surface_config();
        let window = window.as_winit_window();

        let mut viewport_info = ViewportInfo::default();
        egui_winit::update_viewport_info(&mut viewport_info, &self.context, window);

        let raw_input = self.state.take_egui_input(&window);

        let egui::FullOutput {
            shapes,
            platform_output,
            textures_delta,
            pixels_per_point,
            ..
        } = self.context.run(raw_input, |ctx| {
            self.gui_pointer.current_gui(ctx, data_object)
        });
        self.state.handle_platform_output(window, platform_output);

        for (id, image_delta) in &textures_delta.set {
            self.renderer.update_texture(
                &graphic_context.wgpu_device,
                &graphic_context.wgpu_queue,
                *id,
                image_delta,
            );
        }
        let clipped_primitives = self.context.tessellate(shapes, pixels_per_point);

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [surface_config.width, surface_config.height],
            pixels_per_point,
        };

        self.draw_data = Some(DrawData {
            clipped_primitives,
            screen_descriptor,
        });
    }
}

impl<DataObject> RenderJob for GuiHandler<DataObject> {
    fn update(
        &mut self,
        command_encoder: &mut wgpu::CommandEncoder,
        graphic_context: &super::Context,
    ) {
        let draw_data = self.draw_data.as_ref().unwrap();
        self.renderer.update_buffers(
            &graphic_context.wgpu_device,
            &graphic_context.wgpu_queue,
            command_encoder,
            &draw_data.clipped_primitives,
            &draw_data.screen_descriptor,
        );
    }

    fn draw<'pass>(&'pass mut self, render_pass: &mut wgpu::RenderPass<'pass>) {
        let draw_data = self.draw_data.as_ref().unwrap();
        self.renderer.render(
            render_pass,
            &draw_data.clipped_primitives,
            &draw_data.screen_descriptor,
        );
    }
}

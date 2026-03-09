use glam::Vec2;
use glow::HasContext;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use inox2d::formats::inp::parse_inp;
use inox2d::puppet::Puppet;
use inox2d::render::InoxRendererExt;
use inox2d_opengl::OpenglRenderer;

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}

#[wasm_bindgen]
pub struct CompanionPuppet {
    renderer: OpenglRenderer,
    puppet: Puppet,
    last_time: f64,
    dt: f32,
}

#[wasm_bindgen]
impl CompanionPuppet {
    /// Load a puppet from raw .inp/.inx bytes and initialize it on the given canvas.
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str, model_bytes: &[u8]) -> Result<CompanionPuppet, JsValue> {
        let document = web_sys::window()
            .ok_or("no window")?
            .document()
            .ok_or("no document")?;

        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or(format!("no element with id '{}'", canvas_id))?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| "element is not a canvas")?;

        // Create WebGL2 context with stencil buffer
        let context_options = js_sys::Object::new();
        js_sys::Reflect::set(&context_options, &"stencil".into(), &true.into())?;
        js_sys::Reflect::set(&context_options, &"alpha".into(), &true.into())?;
        js_sys::Reflect::set(&context_options, &"premultipliedAlpha".into(), &true.into())?;

        let webgl2_context = canvas
            .get_context_with_context_options("webgl2", &context_options)
            .map_err(|e| format!("get_context error: {:?}", e))?
            .ok_or("no webgl2 context")?
            .dyn_into::<web_sys::WebGl2RenderingContext>()
            .map_err(|_| "not a WebGl2RenderingContext")?;

        let gl = glow::Context::from_webgl2_context(webgl2_context);

        // Set clear color to transparent immediately
        unsafe {
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
        }

        // Parse model
        let mut model = parse_inp(model_bytes)
            .map_err(|e| format!("Failed to parse model: {:?}", e))?;

        model.puppet.init_transforms();
        model.puppet.init_rendering();
        model.puppet.init_params();
        model.puppet.init_physics();

        // Create renderer
        let mut renderer = OpenglRenderer::new(gl, &model)
            .map_err(|e| format!("Failed to create renderer: {:?}", e))?;

        // Set initial camera scale
        renderer.camera.scale = Vec2::splat(0.15);

        Ok(CompanionPuppet {
            renderer,
            puppet: model.puppet,
            last_time: 0.0,
            dt: 0.0,
        })
    }

    /// Resize the renderer viewport.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    /// Set a parameter value. For 1D params, only `x` is used.
    pub fn set_param(&mut self, name: &str, x: f32, y: f32) -> Result<(), JsValue> {
        if let Some(ctx) = self.puppet.param_ctx.as_mut() {
            ctx.set(name, Vec2::new(x, y))
                .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
        }
        Ok(())
    }

    /// Get all parameter names as a JSON array string.
    pub fn get_param_names(&self) -> String {
        let names: Vec<&String> = self.puppet.params.keys().collect();
        serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string())
    }

    /// Begin a new frame. Call set_param() after this, then call end_and_draw().
    pub fn begin_frame(&mut self, timestamp: f64) {
        self.puppet.begin_frame();

        let dt = if self.last_time == 0.0 {
            0.0
        } else {
            ((timestamp - self.last_time) / 1000.0) as f32
        };
        self.last_time = timestamp;
        self.dt = dt.min(0.1);
    }

    /// Finalize params, run physics, and draw. Call after set_param().
    pub fn end_and_draw(&mut self) {
        self.puppet.end_frame(self.dt);

        self.renderer.clear();
        self.renderer.on_begin_draw(&self.puppet);
        self.renderer.draw(&self.puppet);
        self.renderer.on_end_draw(&self.puppet);
    }

    /// Set camera scale (zoom level).
    pub fn set_camera_scale(&mut self, scale: f32) {
        self.renderer.camera.scale = Vec2::splat(scale);
    }

    /// Set camera position.
    pub fn set_camera_position(&mut self, x: f32, y: f32) {
        self.renderer.camera.position = Vec2::new(x, y);
    }
}

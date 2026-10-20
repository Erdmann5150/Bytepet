//! A transparent OpenGL surface that shows one RGBA image (the rendered device).
//! Uses GL 2.1 or GLES 2, which every desktop GPU driver (and software renderer) supports.

use std::error::Error;
use std::num::NonZeroU32;

use glow::HasContext;
use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, Version};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, Surface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Window, WindowAttributes};

pub struct Gfx {
    pub window: Window,
    surface: Surface<WindowSurface>,
    context: PossiblyCurrentContext,
    gl: glow::Context,
    program: glow::Program,
    texture: glow::Texture,
    tex_size: (u32, u32),
    pub transparent: bool,
}

const VS_120: &str = "#version 120\nattribute vec2 pos;\nvarying vec2 uv;\nvoid main() {\n  uv = vec2((pos.x + 1.0) * 0.5, (1.0 - pos.y) * 0.5);\n  gl_Position = vec4(pos, 0.0, 1.0);\n}\n";
const FS_120: &str =
    "#version 120\nvarying vec2 uv;\nuniform sampler2D tex;\nvoid main() { gl_FragColor = texture2D(tex, uv); }\n";
const VS_100: &str = "#version 100\nattribute vec2 pos;\nvarying vec2 uv;\nvoid main() {\n  uv = vec2((pos.x + 1.0) * 0.5, (1.0 - pos.y) * 0.5);\n  gl_Position = vec4(pos, 0.0, 1.0);\n}\n";
const FS_100: &str = "#version 100\nprecision mediump float;\nvarying vec2 uv;\nuniform sampler2D tex;\nvoid main() { gl_FragColor = texture2D(tex, uv); }\n";

fn pick_config(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
    // prefer configs that can show through to the desktop, then fewer MSAA samples
    configs
        .reduce(|best, c| {
            let ct = c.supports_transparency().unwrap_or(false);
            let bt = best.supports_transparency().unwrap_or(false);
            if (ct && !bt)
                || (ct == bt && c.alpha_size() > best.alpha_size())
                || (ct == bt && c.num_samples() < best.num_samples() && c.alpha_size() == best.alpha_size())
            {
                c
            } else {
                best
            }
        })
        .expect("no OpenGL configuration available")
}

impl Gfx {
    pub fn new(event_loop: &ActiveEventLoop, attrs: WindowAttributes) -> Result<Gfx, Box<dyn Error>> {
        let template = ConfigTemplateBuilder::new().with_alpha_size(8).with_transparency(true);
        let (window, config) =
            DisplayBuilder::new().with_window_attributes(Some(attrs)).build(event_loop, template, pick_config)?;
        let window = window.ok_or("could not create the window")?;
        let transparent = config.supports_transparency().unwrap_or(false);
        let raw = window.window_handle().ok().map(|h| h.as_raw());
        let display = config.display();
        let gl21 =
            ContextAttributesBuilder::new().with_context_api(ContextApi::OpenGl(Some(Version::new(2, 1)))).build(raw);
        let gles =
            ContextAttributesBuilder::new().with_context_api(ContextApi::Gles(Some(Version::new(2, 0)))).build(raw);
        let not_current = unsafe {
            display
                .create_context(&config, &gl21)
                .or_else(|_| display.create_context(&config, &gles))
                .or_else(|_| display.create_context(&config, &ContextAttributesBuilder::new().build(raw)))?
        };
        let surface_attrs = window.build_surface_attributes(Default::default())?;
        let surface = unsafe { display.create_window_surface(&config, &surface_attrs)? };
        let context = not_current.make_current(&surface)?;
        let _ = surface.set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()));
        let gl = unsafe { glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s) as *const _) };

        let (vs, fs) = if gl.version().is_embedded { (VS_100, FS_100) } else { (VS_120, FS_120) };
        let (program, texture) = unsafe {
            let program = gl.create_program()?;
            for (kind, src) in [(glow::VERTEX_SHADER, vs), (glow::FRAGMENT_SHADER, fs)] {
                let sh = gl.create_shader(kind)?;
                gl.shader_source(sh, src);
                gl.compile_shader(sh);
                if !gl.get_shader_compile_status(sh) {
                    return Err(format!("shader: {}", gl.get_shader_info_log(sh)).into());
                }
                gl.attach_shader(program, sh);
            }
            gl.bind_attrib_location(program, 0, "pos");
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                return Err(format!("program: {}", gl.get_program_info_log(program)).into());
            }
            gl.use_program(Some(program));
            let quad: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
            let bytes: Vec<u8> = quad.iter().flat_map(|f| f.to_ne_bytes()).collect();
            let vbo = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);
            gl.enable_vertex_attrib_array(0);
            let texture = gl.create_texture()?;
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            for (p, v) in [
                (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
                (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, p, v as i32);
            }
            let loc = gl.get_uniform_location(program, "tex");
            gl.uniform_1_i32(loc.as_ref(), 0);
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.disable(glow::BLEND);
            (program, texture)
        };
        Ok(Gfx { window, surface, context, gl, program, texture, tex_size: (0, 0), transparent })
    }

    pub fn resize(&self, w: u32, h: u32) {
        if let (Some(w), Some(h)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
            self.surface.resize(&self.context, w, h);
        }
    }

    /// Show a premultiplied RGBA image covering the whole window.
    pub fn present(&mut self, rgba: &[u8], w: u32, h: u32) {
        let size = self.window.inner_size();
        unsafe {
            let gl = &self.gl;
            gl.use_program(Some(self.program));
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            if self.tex_size != (w, h) {
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA as i32,
                    w as i32,
                    h as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(rgba)),
                );
                self.tex_size = (w, h);
            } else {
                gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    0,
                    0,
                    w as i32,
                    h as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(rgba)),
                );
            }
            gl.viewport(0, 0, size.width as i32, size.height as i32);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
        let _ = self.surface.swap_buffers(&self.context);
    }
}

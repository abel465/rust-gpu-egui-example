use std::{num::NonZeroU64, sync::Arc};

use eframe::{
    egui_wgpu::wgpu::util::DeviceExt,
    egui_wgpu::{self, wgpu},
};

use shared::ShaderConstants;
use std::borrow::Cow;

struct CompiledShaderModules {
    named_spv_modules: Vec<(Option<String>, wgpu::ShaderModuleDescriptorSpirV<'static>)>,
}

impl CompiledShaderModules {
    fn spv_module_for_entry_point<'a>(
        &'a self,
        wanted_entry: &str,
    ) -> wgpu::ShaderModuleDescriptorSpirV<'a> {
        for (name, spv_module) in &self.named_spv_modules {
            match name {
                Some(name) if name != wanted_entry => continue,
                _ => {
                    return wgpu::ShaderModuleDescriptorSpirV {
                        label: name.as_deref(),
                        source: Cow::Borrowed(&spv_module.source),
                    };
                }
            }
        }
        unreachable!(
            "{wanted_entry:?} not found in modules {:?}",
            self.named_spv_modules
                .iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>()
        );
    }
}

fn maybe_watch(
    // options: &Options,
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))] on_watch: Option<
        Box<dyn FnMut(CompiledShaderModules) + Send + 'static>,
    >,
) -> CompiledShaderModules {
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    {
        use spirv_builder::{CompileResult, MetadataPrintout, SpirvBuilder};
        use std::path::PathBuf;
        // Hack: spirv_builder builds into a custom directory if running under cargo, to not
        // deadlock, and the default target directory if not. However, packages like `proc-macro2`
        // have different configurations when being built here vs. when building
        // rustc_codegen_spirv normally, so we *want* to build into a separate target directory, to
        // not have to rebuild half the crate graph every time we run. So, pretend we're running
        // under cargo by setting these environment variables.
        std::env::set_var("OUT_DIR", env!("OUT_DIR"));
        std::env::set_var("PROFILE", env!("PROFILE"));
        let crate_name = "mandelbrot";
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let crate_path = [manifest_dir, "..", "shaders", crate_name]
            .iter()
            .copied()
            .collect::<PathBuf>();

        // let has_debug_printf = options.force_spirv_passthru;
        let has_debug_printf = false;

        let builder = SpirvBuilder::new(crate_path, "spirv-unknown-vulkan1.1")
            .print_metadata(MetadataPrintout::None)
            .shader_panic_strategy(if has_debug_printf {
                spirv_builder::ShaderPanicStrategy::DebugPrintfThenExit {
                    print_inputs: true,
                    print_backtrace: true,
                }
            } else {
                spirv_builder::ShaderPanicStrategy::SilentExit
            })
            // HACK(eddyb) needed because of `debugPrintf` instrumentation limitations
            // (see https://github.com/KhronosGroup/SPIRV-Tools/issues/4892).
            .multimodule(has_debug_printf);
        let initial_result = if let Some(mut f) = on_watch {
            builder
                .watch(move |compile_result| f(handle_compile_result(compile_result)))
                .expect("Configuration is correct for watching")
        } else {
            builder.build().unwrap()
        };
        fn handle_compile_result(compile_result: CompileResult) -> CompiledShaderModules {
            let load_spv_module = |path| {
                let data = std::fs::read(path).unwrap();
                // FIXME(eddyb) this reallocates all the data pointlessly, there is
                // not a good reason to use `ShaderModuleDescriptorSpirV` specifically.
                let spirv = Cow::Owned(wgpu::util::make_spirv_raw(&data).into_owned());
                wgpu::ShaderModuleDescriptorSpirV {
                    label: None,
                    source: spirv,
                }
            };
            CompiledShaderModules {
                named_spv_modules: match compile_result.module {
                    spirv_builder::ModuleResult::SingleModule(path) => {
                        vec![(None, load_spv_module(path))]
                    }
                    spirv_builder::ModuleResult::MultiModule(modules) => modules
                        .into_iter()
                        .map(|(name, path)| (Some(name), load_spv_module(path)))
                        .collect(),
                },
            }
        }
        handle_compile_result(initial_result)
    }
}

pub struct Custom3d {
    shader_constants: ShaderConstants,
}

impl Custom3d {
    pub fn new<'a>(cc: &'a eframe::CreationContext<'a>) -> Option<Self> {
        // Get the WGPU render state from the eframe creation context. This can also be retrieved
        // from `eframe::Frame` when you don't have a `CreationContext` available.
        let wgpu_render_state = cc.wgpu_render_state.as_ref()?;

        let device = &wgpu_render_state.device;

        let create_shader = |module| {
            let wgpu::ShaderModuleDescriptorSpirV { label, source } = module;
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("custom3d"),
                source: wgpu::ShaderSource::SpirV(source).into(),
            })
        };
        let compiled_shader_modules = maybe_watch(
            // options,
            #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
            {
                // let proxy = event_loop.create_proxy();
                // Some(Box::new(move |res| match proxy.send_event(res) {
                //     Ok(it) => it,
                //     // ShaderModuleDescriptor is not `Debug`, so can't use unwrap/expect
                //     Err(_err) => panic!("Event loop dead"),
                // }))
                Some(Box::new(move |_| ()))
            },
        );
        let vs_module_descr = compiled_shader_modules.spv_module_for_entry_point("main_vs");
        let fs_module_descr = compiled_shader_modules.spv_module_for_entry_point("main_fs");
        let vs_module = &create_shader(vs_module_descr);
        let fs_module = &create_shader(fs_module_descr);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("custom3d"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(16),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("custom3d"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                range: 0..std::mem::size_of::<ShaderConstants>() as u32,
            }],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("custom3d"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: vs_module,
                entry_point: "main_vs",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: fs_module,
                entry_point: "main_fs",
                targets: &[Some(wgpu_render_state.target_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("custom3d"),
            contents: bytemuck::cast_slice(&[0.0_f32; 4]), // 16 bytes aligned!
            // Mapping at creation (as done by the create_buffer_init utility) doesn't require us to to add the MAP_WRITE usage
            // (this *happens* to workaround this bug )
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("custom3d"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Because the graphics pipeline must have the same lifetime as the egui render pass,
        // instead of storing the pipeline in our `Custom3D` struct, we insert it into the
        // `paint_callback_resources` type map, which is stored alongside the render pass.
        wgpu_render_state
            .renderer
            .write()
            .paint_callback_resources
            .insert(TriangleRenderResources {
                pipeline,
                bind_group,
                uniform_buffer,
            });

        Some(Self {
            shader_constants: ShaderConstants {
                width: 400,
                height: 400,
                time: 0.0,
                cursor_x: 0.0,
                cursor_y: 0.0,
                drag_start_x: 0.0,
                drag_start_y: 0.0,
                drag_end_x: 0.0,
                drag_end_y: 0.0,
                zoom: 1.0,
                translate_x: 0.0,
                translate_y: 0.0,
                mouse_button_pressed: 0,
                mouse_button_press_time: [0.0, 0.0, 0.0],
            },
        })
    }
}

impl eframe::App for Custom3d {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::Frame::canvas(ui.style()).show(ui, |ui| {
                        self.custom_painting(ui);
                    });
                });
        });
    }
}

impl Custom3d {
    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let available_size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(available_size, egui::Sense::drag());

        self.shader_constants.width = available_size.x as u32;
        self.shader_constants.height = available_size.y as u32;
        self.shader_constants.translate_x -= response.drag_delta().x;
        self.shader_constants.translate_y -= response.drag_delta().y;

        // Clone locals so we can move them into the paint callback:
        let shader_constants = self.shader_constants;

        // The callback function for WGPU is in two stages: prepare, and paint.
        //
        // The prepare callback is called every frame before paint and is given access to the wgpu
        // Device and Queue, which can be used, for instance, to update buffers and uniforms before
        // rendering.
        //
        // You can use the main `CommandEncoder` that is passed-in, return an arbitrary number
        // of user-defined `CommandBuffer`s, or both.
        // The main command buffer, as well as all user-defined ones, will be submitted together
        // to the GPU in a single call.
        //
        // The paint callback is called after prepare and is given access to the render pass, which
        // can be used to issue draw commands.
        let cb = egui_wgpu::CallbackFn::new().paint(
            move |_info, render_pass, paint_callback_resources| {
                let resources: &TriangleRenderResources = paint_callback_resources.get().unwrap();
                resources.paint(render_pass, shader_constants);
            },
        );

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };

        ui.painter().add(callback);
    }
}

struct TriangleRenderResources {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl TriangleRenderResources {
    fn paint<'rp>(
        &'rp self,
        render_pass: &mut wgpu::RenderPass<'rp>,
        push_constants: ShaderConstants,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_push_constants(
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            0,
            bytemuck::bytes_of(&push_constants),
        );
        render_pass.draw(0..3, 0..1);
    }
}

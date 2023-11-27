use egui::Widget;

/// How often we repaint the demo app by default
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RunMode {
    /// This is the default for the demo.
    ///
    /// If this is selected, egui is only updated if are input events
    /// (like mouse movements) or there are some animations in the GUI.
    ///
    /// Reactive mode saves CPU.
    ///
    /// The downside is that the UI can become out-of-date if something it is supposed to monitor changes.
    /// For instance, a GUI for a thermostat need to repaint each time the temperature changes.
    /// To ensure the UI is up to date you need to call `egui::Context::request_repaint()` each
    /// time such an event happens. You can also chose to call `request_repaint()` once every second
    /// or after every single frame - this is called [`Continuous`](RunMode::Continuous) mode,
    /// and for games and interactive tools that need repainting every frame anyway, this should be the default.
    Reactive,

    /// This will call `egui::Context::request_repaint()` at the end of each frame
    /// to request the backend to repaint as soon as possible.
    ///
    /// On most platforms this will mean that egui will run at the display refresh rate of e.g. 60 Hz.
    ///
    /// For this demo it is not any reason to do so except to
    /// demonstrate how quickly egui runs.
    ///
    /// For games or other interactive apps, this is probably what you want to do.
    /// It will guarantee that egui is always up-to-date.
    Continuous,
}

/// Default for demo is Reactive since
/// 1) We want to use minimal CPU
/// 2) There are no external events that could invalidate the UI
///    so there are no events to miss.
impl Default for RunMode {
    fn default() -> Self {
        RunMode::Reactive
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct BackendPanel {
    pub open: bool,

    #[cfg_attr(feature = "serde", serde(skip))]
    // go back to [`RunMode::Reactive`] mode each time we start
    run_mode: RunMode,

    #[cfg_attr(feature = "serde", serde(skip))]
    repaint_after_seconds: f32,

    #[cfg_attr(feature = "serde", serde(skip))]
    frame_history: crate::frame_history::FrameHistory,

    egui_windows: EguiWindows,
}

impl Default for BackendPanel {
    fn default() -> Self {
        Self {
            open: false,
            run_mode: Default::default(),
            repaint_after_seconds: 1.0,
            frame_history: Default::default(),
            egui_windows: Default::default(),
        }
    }
}

impl BackendPanel {
    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.frame_history
            .on_new_frame(ctx.input(|i| i.time), frame.info().cpu_usage);

        match self.run_mode {
            RunMode::Continuous => {
                // Tell the backend to repaint as soon as possible
                ctx.request_repaint();
            }
            RunMode::Reactive => {
                // let the computer rest for a bit
                ctx.request_repaint_after(std::time::Duration::from_secs_f32(
                    self.repaint_after_seconds,
                ));
            }
        }
    }

    pub fn end_of_frame(&mut self, ctx: &egui::Context) {
        self.egui_windows.windows(ctx);
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::trace!(ui);

        self.run_mode_ui(ui);

        ui.separator();

        self.frame_history.ui(ui);

        ui.separator();

        ui.label("egui windows:");
        self.egui_windows.checkboxes(ui);

        ui.separator();

        {
            let mut debug_on_hover = ui.ctx().debug_on_hover();
            ui.checkbox(&mut debug_on_hover, "🐛 Debug on hover")
                .on_hover_text("Show structure of the ui when you hover with the mouse");
            ui.ctx().set_debug_on_hover(debug_on_hover);
        }

        ui.separator();

        #[cfg(not(target_arch = "wasm32"))]
        {
            ui.separator();
            if ui.button("Quit").clicked() {
                frame.close();
            }
        }
    }

    fn run_mode_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let run_mode = &mut self.run_mode;
            ui.label("Mode:");
            ui.radio_value(run_mode, RunMode::Reactive, "Reactive")
                .on_hover_text("Repaint when there are animations or input (e.g. mouse movement)");
            ui.radio_value(run_mode, RunMode::Continuous, "Continuous")
                .on_hover_text("Repaint everything each frame");
        });

        if self.run_mode == RunMode::Continuous {
            ui.label(format!(
                "Repainting the UI each frame. FPS: {:.1}",
                self.frame_history.fps()
            ));
        } else {
            ui.label("Only running UI code when there are animations or input.");

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("(but at least every ");
                egui::DragValue::new(&mut self.repaint_after_seconds)
                    .clamp_range(0.1..=10.0)
                    .speed(0.1)
                    .suffix(" s")
                    .ui(ui)
                    .on_hover_text("Repaint this often, even if there is no input.");
                ui.label(")");
            });
        }
    }
}

// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct EguiWindows {
    // egui stuff:
    settings: bool,
    inspection: bool,
}

impl Default for EguiWindows {
    fn default() -> Self {
        EguiWindows::none()
    }
}

impl EguiWindows {
    fn none() -> Self {
        Self {
            settings: false,
            inspection: false,
        }
    }

    fn checkboxes(&mut self, ui: &mut egui::Ui) {
        let Self {
            settings,
            inspection,
        } = self;

        ui.checkbox(settings, "🔧 Settings");
        ui.checkbox(inspection, "🔍 Inspection");
    }

    fn windows(&mut self, ctx: &egui::Context) {
        let Self {
            settings,
            inspection,
        } = self;

        egui::Window::new("🔧 Settings")
            .open(settings)
            .vscroll(true)
            .show(ctx, |ui| {
                ctx.settings_ui(ui);
            });

        egui::Window::new("🔍 Inspection")
            .open(inspection)
            .vscroll(true)
            .show(ctx, |ui| {
                ctx.inspection_ui(ui);
            });
    }
}

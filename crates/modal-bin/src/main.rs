use eframe::egui::{self, Vec2};
use modal_lib::{
    editor::{GraphEditor, GraphEditorState, ModalApp},
    remote::{rodio_out::RodioOut, RuntimeRemote},
};

fn main() {
    let options = eframe::NativeOptions {
        window_builder: Some(Box::new(|viewport| {
            viewport.with_inner_size(Vec2::new(1600.0, 1200.0))
        })),
        ..Default::default()
    };

    eframe::run_native(
        "Modal",
        options,
        Box::new(|cc| Ok(Box::new(ModalStandalone::with_context(cc)))),
    )
    .unwrap();
}

struct ModalStandalone {
    app: ModalApp,
}

impl ModalStandalone {
    fn with_context(cc: &eframe::CreationContext) -> Self {
        cc.egui_ctx
            .all_styles_mut(|style| style.interaction.selectable_labels = false);

        let state: Option<GraphEditorState> = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, "synth-app"));

        let remote = RuntimeRemote::start(Box::new(RodioOut::default()));
        let mut editor = GraphEditor::new(remote);
        if let Some(state) = state {
            editor.replace(state);
        }

        ModalStandalone {
            app: ModalApp::new(editor),
        }
    }
}

impl eframe::App for ModalStandalone {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "synth-app", &self.app.serializable_state());
        println!("state saved");
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.app.on_exit();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.main_app(ctx);
    }
}

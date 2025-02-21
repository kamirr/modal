mod rodio_out;

use eframe::egui::{self, Vec2};
use modal_editor::{ModalEditor, ModalEditorState};

use rodio_out::RodioOut;

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
        Box::new(|cc| Ok(Box::new(ModalApp::with_context(cc)))),
    )
    .unwrap();
}

struct ModalApp {
    editor: ModalEditor,
}

impl ModalApp {
    fn with_context(cc: &eframe::CreationContext) -> Self {
        cc.egui_ctx
            .all_styles_mut(|style| style.interaction.selectable_labels = false);

        let state: Option<ModalEditorState> = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, "synth-app"));

        let mut editor = ModalEditor::new(Box::new(RodioOut::default()));
        if let Some(state) = state {
            editor.replace(state);
        }

        ModalApp { editor }
    }
}

impl eframe::App for ModalApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "synth-app", &self.editor.serializable_state());
        println!("state saved");
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.editor.shutdown();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.editor.update(ctx);
    }
}

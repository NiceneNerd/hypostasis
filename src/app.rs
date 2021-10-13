use std::path::{Path, PathBuf};

use anyhow::Result;
use eframe::{egui, egui::Vec2, epi};
use glob::glob;

pub struct App {
    folder: String,
    objects: Vec<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            folder: String::new(),
            objects: vec![],
        }
    }
}

impl epi::App for App {
    fn name(&self) -> &str {
        "Hypostasis"
    }

    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        _storage: Option<&dyn epi::Storage>,
    ) {
        ()
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        let Self { folder, objects } = self;
        let mut tasks: Vec<&dyn Fn(&mut Self) -> Result<()>> = vec![];

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2 { x: 0.0, y: 8.0 };
            ui.label("Select a mod project:");
            ui.text_edit_singleline(folder);
            if ui.small_button("Browse").clicked() {
                if let Some(picked) = rfd::FileDialog::new().pick_folder() {
                    *folder = picked.to_str().unwrap().to_owned()
                }
            }
            if objects.len() > 0 {
                ui.add(egui::Label::new(objects.join("\n")).code());
            }
            if ui.button("Process").clicked() {
                tasks.push(&Self::process_maps)
            }
        });
        for task in tasks {
            task(self).unwrap();
        }
    }
}

impl App {
    fn process_maps(&mut self) -> Result<()> {
        for file in glob(
            Path::new(&self.folder)
                .join("**/MainField/**/?-?_*.smubin")
                .to_str()
                .unwrap(),
        )
        .unwrap()
        .filter_map(|f| f.ok())
        {
            println!("{}", file.display());
        }
        Ok(())
    }
}

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use eframe::{egui, egui::Vec2, epi};
use glob::glob;
use roead::{byml::Byml, yaz0::decompress};

static HASHES: &str = include_str!("../data/hashes.txt");

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
            match task(self) {
                Ok(_) => (),
                Err(e) => {
                    let mut open = true;
                    egui::Window::new("Error").show(ctx, |ui| {
                        ui.label(&e.to_string());
                        if ui.button("OK").clicked() {
                            open = false;
                        }
                    });
                }
            }
        }
    }
}

impl App {
    fn process_maps(&mut self) -> Result<()> {
        let hashes: HashSet<u32> = HASHES
            .split(',')
            .map(|s| u32::from_str_radix(s, 10).unwrap())
            .collect();
        let remaps: HashMap<u32, u32> = glob(
            Path::new(&self.folder)
                .join("**/MainField/**/?-?_*.smubin")
                .to_str()
                .context("Bad glob")?,
        )?
        .filter_map(|f| f.ok())
        .flat_map(process_map_file)
        .collect();
        self.objects = remaps
            .iter()
            .map(|(o, r)| format!("0x{:02x} => 0x{:02x}", o, r))
            .collect();
        Ok(())
    }
}

fn process_map_file(file: PathBuf) -> Result<impl Iterator<Item = (u32, u32)>> {
    let mut data =
        Byml::from_binary(&decompress(std::fs::read(&file).unwrap()).unwrap()).unwrap();
    let unit_remaps: HashMap<u32, u32> = data["Objs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|obj| {
            let actor = obj.as_hash().unwrap();
            if let Ok(id) = actor["HashId"].as_uint() {
                if !hashes.contains(&id) {
                    println!("{}", obj.to_text());
                    let new_id = crc::crc32::checksum_ieee(obj.to_text().as_bytes());
                    Some((id, new_id))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();
    data["Objs"]
        .as_mut_array()
        .unwrap()
        .iter_mut()
        .filter(|obj| obj.as_hash().unwrap().contains_key("LinksToObj"))
        .for_each(|obj| {
            for link in obj
                .as_mut_hash()
                .unwrap()
                .get_mut("LinksToObj")
                .unwrap()
                .as_mut_array()
                .unwrap()
            {
                if let Some(replace) = unit_remaps
                    .get(&link.as_hash().unwrap()["DestUnitHashId"].as_uint().unwrap())
                {
                    link.as_mut_hash()
                        .unwrap()
                        .insert("DestUnitHashId".to_owned(), Byml::UInt(*replace));
                }
            }
        });
    Ok(unit_remaps.into_iter())
}
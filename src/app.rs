use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use eframe::{
    egui,
    egui::{Id, Vec2},
    epi,
};
use glob::glob;
use roead::{
    byml::Byml,
    yaz0::{compress, decompress},
};

static HASHES: &str = include_str!("../data/hashes.txt");

pub struct App {
    error: Option<String>,
    show_error: bool,
    folder: String,
    objects: Vec<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            error: None,
            show_error: false,
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
        let Self {
            folder,
            error,
            show_error,
            objects,
        } = self;

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
                ui.label("HashIDs replaced:");
                ui.add(egui::Label::new(objects.join("\n")).code());
            }
            if ui.button("Process").clicked() {
                match process_maps(&*folder) {
                    Ok(objs) => *objects = objs,
                    Err(e) => {
                        *show_error = true;
                        *error = Some(e.to_string());
                    }
                }
            }
        });
        let mut show = *show_error;
        if *show_error {
            egui::Window::new("Error").open(show_error).show(ctx, |ui| {
                ui.label(error.as_ref().unwrap());
                if ui.button("OK").clicked() {
                    show = false;
                }
            });
            if !show {
                *show_error = false;
                *error = None;
            }
        }
    }
}

fn process_maps(folder: &String) -> Result<Vec<String>> {
    let hashes: HashSet<u32> = HASHES
        .split(',')
        .map(|s| u32::from_str_radix(s, 10).unwrap())
        .collect();
    let files: Vec<PathBuf> = glob(
        Path::new(folder)
            .join("**/MainField/**/?-?_*.smubin")
            .to_str()
            .context("Bad glob")?,
    )?
    .filter_map(|f| f.ok())
    .collect();
    let remaps: HashMap<u32, u32> = files
        .iter()
        .map(|file| {
            let bytes = decompress(std::fs::read(&file)?)?;
            let mut data = Byml::from_binary(&bytes)?;
            let endian = if &bytes[0..2] == b"BY" {
                roead::Endian::Big
            } else {
                roead::Endian::Little
            };
            drop(bytes);
            std::fs::rename(&file, file.with_extension("bak"))?;
            let unit_remaps: HashMap<u32, u32> = data["Objs"]
                .as_array()?
                .iter()
                .filter_map(|obj| {
                    let actor = obj.as_hash().unwrap();
                    if let Ok(id) = actor["HashId"].as_uint() {
                        if !hashes.contains(&id) {
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
                .try_for_each(|obj| -> Result<()> {
                    for link in obj
                        .as_mut_hash()?
                        .get_mut("LinksToObj")
                        .unwrap()
                        .as_mut_array()?
                    {
                        if let Some(replace) =
                            unit_remaps.get(&link.as_hash()?["DestUnitHashId"].as_uint()?)
                        {
                            link.as_mut_hash()
                                .unwrap()
                                .insert("DestUnitHashId".to_owned(), Byml::UInt(*replace));
                        }
                    }
                    Ok(())
                })?;
            std::fs::write(&file, compress(data.to_binary(endian)))?;
            Ok(unit_remaps)
        })
        .collect::<Result<Vec<HashMap<u32, u32>>>>()?
        .into_iter()
        .flatten()
        .collect();
    Ok(remaps
        .iter()
        .map(|(o, r)| format!("0x{:02x} => 0x{:02x}", o, r))
        .collect())
}

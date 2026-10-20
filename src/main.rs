use eframe::egui::{self, Color32, ColorImage, RichText, TextureHandle};
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader};
use std::{path::{Path, PathBuf}, process::Command, sync::mpsc::{self, Receiver}, thread};

const BLUE: Color32 = Color32::from_rgb(48, 133, 255);
const BG: Color32 = Color32::from_rgb(17, 21, 28);

#[derive(Clone, Copy, PartialEq)]
enum ExportFormat { Jpeg, Png, Tiff }
impl ExportFormat {
    fn label(self) -> &'static str { match self { Self::Jpeg => "JPEG", Self::Png => "PNG", Self::Tiff => "TIFF" } }
    fn extension(self) -> &'static str { match self { Self::Jpeg => "jpg", Self::Png => "png", Self::Tiff => "tiff" } }
    fn image_format(self) -> ImageFormat { match self { Self::Jpeg => ImageFormat::Jpeg, Self::Png => ImageFormat::Png, Self::Tiff => ImageFormat::Tiff } }
}

struct Photo { path: PathBuf, selected: bool, thumbnail: Option<TextureHandle>, thumbnail_attempted: bool }
struct Preview { path: PathBuf, factor: f32, original: TextureHandle, transformed: TextureHandle, width: u32, height: u32 }
struct App {
    view: View, compare: Compare, logo: Option<TextureHandle>,
    photos: Vec<Photo>, active: usize, factor: f32, custom_factor: f32,
    export_format: ExportFormat, destination: Option<PathBuf>, preview: Option<Preview>,
    busy: bool, status: String, rx: Option<Receiver<String>>,
}
impl Default for App {
    fn default() -> Self { Self { view: View::Process, compare: Compare::Split, logo: None, photos: vec![], active: 0, factor: 1.5, custom_factor: 1.5, export_format: ExportFormat::Tiff, destination: None, preview: None, busy: false, status: "Importa fotografias para começar.".into(), rx: None } }
}

fn is_accepted(p: &Path) -> bool {
    p.extension().and_then(|s| s.to_str()).map(|s| matches!(s.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png" | "tiff" | "tif" | "bmp" | "webp" | "heic" | "heif" | "arw" | "dng" | "cr2" | "cr3" | "nef" | "raf" | "orf" | "rw2")).unwrap_or(false)
}
fn read_photo(path: &Path) -> Result<DynamicImage, String> {
    let direct = ImageReader::open(path).map_err(|e| e.to_string()).and_then(|r| r.with_guessed_format().map_err(|e| e.to_string())).and_then(|r| r.decode().map_err(|e| e.to_string()));
    if let Ok(img) = direct { return Ok(img); }
    #[cfg(target_os = "macos")]
    {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let dest = temp.path().join("preview.tiff");
        let output = Command::new("/usr/bin/sips").arg("-s").arg("format").arg("tiff").arg(path).arg("--out").arg(&dest).output().map_err(|e| e.to_string())?;
        if !output.status.success() { return Err(format!("O macOS não conseguiu abrir este RAW/HEIC: {}", String::from_utf8_lossy(&output.stderr))); }
        return ImageReader::open(&dest).map_err(|e| e.to_string())?.decode().map_err(|e| e.to_string());
    }
    #[cfg(not(target_os = "macos"))]
    Err(format!("Formato não suportado: {}", path.display()))
}
fn squeezed(img: &DynamicImage, factor: f32) -> DynamicImage {
    let (w, h) = img.dimensions();
    let new_w = ((w as f64 * factor as f64).round() as u64).clamp(1, 100_000) as u32;
    img.resize_exact(new_w, h, image::imageops::FilterType::Lanczos3)
}
fn save_export(dir: &Path, source: &Path, format: ExportFormat, img: &DynamicImage) -> Result<PathBuf, String> {
    use std::{fs::OpenOptions, io::{BufWriter, ErrorKind}};
    let name = source.file_stem().and_then(|s| s.to_str()).unwrap_or("imagem");
    for i in 0..100_000 {
        let suffix = if i == 0 { String::new() } else { format!("_{i}") };
        let output = dir.join(format!("{name}_desqueezed{suffix}.{}", format.extension()));
        let file = match OpenOptions::new().write(true).create_new(true).open(&output) {
            Ok(file) => file,
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        };
        let mut writer = BufWriter::new(file);
        // JPEG accepts RGB8, including when the source is RGBA or 16-bit TIFF.
        let result = if format == ExportFormat::Jpeg {
            DynamicImage::ImageRgb8(img.to_rgb8()).write_to(&mut writer, format.image_format())
        } else {
            img.write_to(&mut writer, format.image_format())
        };
        use std::io::Write;
        let result = result.map_err(|e| e.to_string()).and_then(|_| writer.flush().map_err(|e| e.to_string()));
        drop(writer);
        if let Err(e) = result {
            let _ = std::fs::remove_file(&output);
            return Err(e);
        }
        return Ok(output);
    }
    Err("Não foi possível reservar um nome de exportação livre.".into())
}
impl App {
    fn add_paths(&mut self, paths: impl IntoIterator<Item=PathBuf>) {
        let mut count = 0;
        for p in paths { if p.is_file() && is_accepted(&p) && !self.photos.iter().any(|x| x.path == p) { self.photos.push(Photo { path: p, selected: true, thumbnail: None, thumbnail_attempted: false }); count += 1; } }
        if count > 0 { self.preview = None; self.status = format!("{count} fotografia(s) adicionada(s)."); }
    }
    fn load_preview(&mut self, ctx: &egui::Context) {
        let Some(photo) = self.photos.get(self.active) else { self.preview = None; return; };
        if let Some(p) = &self.preview { if p.path == photo.path && (p.factor-self.factor).abs() < 0.00001 { return; } }
        let path = photo.path.clone();
        match read_photo(&path) {
            Ok(img) => {
                let (w,h) = img.dimensions();
                // Preview is limited to 1800 px; full export uses original source resolution.
                let thumbnail = img.thumbnail(1800, 1200);
                let transformed = squeezed(&thumbnail, self.factor);
                let original_rgba = thumbnail.to_rgba8();
                let (ow, oh) = thumbnail.dimensions();
                let original = ctx.load_texture("original-preview", ColorImage::from_rgba_unmultiplied([ow as usize, oh as usize], original_rgba.as_raw()), egui::TextureOptions::LINEAR);
                let rgba = transformed.to_rgba8();
                let (tw,th) = transformed.dimensions();
                let texture = ctx.load_texture("preview", ColorImage::from_rgba_unmultiplied([tw as usize, th as usize], rgba.as_raw()), egui::TextureOptions::LINEAR);
                self.preview = Some(Preview { path, factor: self.factor, original, transformed: texture, width: w, height: h });
            }
            Err(e) => { self.preview = None; self.status = format!("Erro a abrir imagem: {e}"); }
        }
    }
    fn export(&mut self) {
        if self.busy { return; }
        let paths: Vec<_> = self.photos.iter().filter(|p| p.selected).map(|p| p.path.clone()).collect();
        if paths.is_empty() { self.status = "Seleciona pelo menos uma fotografia.".into(); return; }
        let Some(folder) = self.destination.clone() else { self.status = "Escolhe uma pasta de exportação.".into(); return; };
        let factor = self.factor; let format = self.export_format;
        let (tx, rx) = mpsc::channel(); self.rx = Some(rx); self.busy = true;
        self.status = format!("A exportar {} fotografia(s)...", paths.len());
        thread::spawn(move || {
            let mut ok = 0; let mut errors = Vec::new();
            for path in &paths {
                let result = (|| -> Result<(), String> {
                    let img = read_photo(path)?;
                    let transformed = squeezed(&img, factor);
                    save_export(&folder, path, format, &transformed)?;
                    Ok(())
                })();
                match result { Ok(()) => ok += 1, Err(e) => errors.push(format!("{}: {}", path.display(), e)) }
            }
            let msg = if errors.is_empty() { format!("Concluído: {ok} imagem(ns) exportada(s). Originais intactos.") } else { format!("{ok} exportada(s); {} erro(s): {}", errors.len(), errors.join(" | ")) };
            let _ = tx.send(msg);
        });
    }
}
#[derive(Clone, Copy, PartialEq)]
enum View { Process, Batch }
#[derive(Clone, Copy, PartialEq)]
enum Compare { Original, Desqueezed, Split }
const PANEL: Color32 = Color32::from_rgb(19, 27, 38);
const CARD: Color32 = Color32::from_rgb(29, 39, 53);
const MUTED: Color32 = Color32::from_rgb(153, 169, 190);
const BORDER: Color32 = Color32::from_rgb(48, 63, 80);

fn card() -> egui::Frame { egui::Frame::new().fill(CARD).corner_radius(10).inner_margin(12) }
fn panel() -> egui::Frame { egui::Frame::new().fill(PANEL).inner_margin(16) }
fn main_button(label: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(RichText::new(label.into()).color(Color32::WHITE).strong()).fill(BLUE).min_size(egui::vec2(0.0, 34.0))
}
fn show_image(ui: &mut egui::Ui, texture: &TextureHandle, max: egui::Vec2) {
    let native = texture.size_vec2();
    let scale = (max.x / native.x).min(max.y / native.y).min(1.0).max(0.001);
    ui.image((texture.id(), native * scale));
}
impl App {
    fn load_next_thumbnail(&mut self, ctx: &egui::Context) {
        let Some((index, photo)) = self.photos.iter_mut().enumerate().find(|(_, p)| !p.thumbnail_attempted) else { return; };
        photo.thumbnail_attempted = true;
        if let Ok(img) = read_photo(&photo.path) {
            let thumb = img.thumbnail(156, 96).to_rgba8();
            photo.thumbnail = Some(ctx.load_texture(
                format!("batch-thumb-{index}"),
                ColorImage::from_rgba_unmultiplied([thumb.width() as usize, thumb.height() as usize], thumb.as_raw()),
                egui::TextureOptions::LINEAR,
            ));
        }
        ctx.request_repaint();
    }
    fn import_files(&mut self) {
        if let Some(paths) = rfd::FileDialog::new().pick_files() { self.add_paths(paths); }
    }
    fn import_folder(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            if let Ok(read) = std::fs::read_dir(dir) { self.add_paths(read.flatten().map(|x| x.path())); }
        }
    }
    fn selected_count(&self) -> usize { self.photos.iter().filter(|p| p.selected).count() }
    fn logo(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(logo) = &self.logo { ui.image((logo.id(), egui::vec2(38.0, 38.0))); }
            ui.vertical(|ui| {
                ui.label(RichText::new("AnaDesqueeze").size(17.0).strong().color(Color32::WHITE));
                ui.label(RichText::new("Descomprime o teu olhar.").small().color(MUTED));
            });
        });
    }
    fn factor_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("FACTOR ANAMÓRFICO").small().color(MUTED));
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            for factor in [1.25_f32, 1.33, 1.5, 1.6, 1.8, 2.0] {
                let selected = (self.factor - factor).abs() < 0.005;
                let button = egui::Button::new(format!("{factor}×")).fill(if selected { BLUE } else { CARD });
                if ui.add(button).clicked() { self.factor = factor; self.custom_factor = factor; }
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Personalizado").color(MUTED));
            if ui.add(egui::DragValue::new(&mut self.custom_factor).speed(0.01).range(0.5..=4.0)).changed() {
                self.factor = self.custom_factor;
            }
        });
    }
    fn export_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("FORMATO DE EXPORTAÇÃO").small().color(MUTED));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for format in [ExportFormat::Tiff, ExportFormat::Png, ExportFormat::Jpeg] {
                let button = egui::Button::new(format.label()).fill(if self.export_format == format { BLUE } else { CARD });
                if ui.add(button).clicked() { self.export_format = format; }
            }
        });
        ui.label(RichText::new("DNG RAW ainda não disponível.").small().color(MUTED));
    }
    fn destination_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("DESTINO").small().color(MUTED));
        if ui.button("Escolher pasta…").clicked() { self.destination = rfd::FileDialog::new().pick_folder(); }
        if let Some(dir) = &self.destination { ui.label(RichText::new(dir.to_string_lossy()).small().color(MUTED)); }
    }
    fn batch_grid(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width();
        let columns = ((width / 175.0).floor() as usize).max(1);
        egui::ScrollArea::vertical().show(ui, |ui| {
            for row in self.photos.chunks_mut(columns).enumerate() {
                let (row_index, photos) = row;
                ui.horizontal(|ui| {
                    for (col, photo) in photos.iter_mut().enumerate() {
                        let index = row_index * columns + col;
                        card().show(ui, |ui| {
                            ui.set_width((width / columns as f32 - 22.0).max(135.0));
                            let preview = if let Some(thumb) = &photo.thumbnail {
                                ui.add(egui::Image::new((thumb.id(), egui::vec2(150.0, 96.0))).sense(egui::Sense::click()))
                            } else {
                                ui.add_sized([150.0, 96.0], egui::Button::new("Pré-visualização pendente"))
                            };
                            if preview.clicked() { self.active = index; self.preview = None; }
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut photo.selected, "");
                                let name = photo.path.file_name().unwrap_or_default().to_string_lossy();
                                if ui.selectable_label(self.active == index, name).clicked() {
                                    self.active = index; self.preview = None;
                                }
                            });
                        });
                    }
                });
                ui.add_space(6.0);
            }
        });
    }
    fn preview_content(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.load_preview(ctx);
        if let Some(preview) = &self.preview {
            let max = egui::vec2(ui.available_width() - 16.0, (ui.available_height() - 40.0).max(100.0));
            ui.vertical_centered(|ui| {
                match self.compare {
                    Compare::Original => show_image(ui, &preview.original, max),
                    Compare::Desqueezed => show_image(ui, &preview.transformed, max),
                    Compare::Split => {
                        ui.horizontal(|ui| {
                            let each = egui::vec2((max.x - 12.0) / 2.0, max.y);
                            ui.vertical(|ui| {
                                ui.label(RichText::new("ORIGINAL").small().color(MUTED));
                                show_image(ui, &preview.original, each);
                            });
                            ui.vertical(|ui| {
                                ui.label(RichText::new("DESCOMPRIMIDA").small().color(MUTED));
                                show_image(ui, &preview.transformed, each);
                            });
                        });
                    }
                }
            });
            ui.add_space(8.0);
            ui.label(RichText::new(format!("{} × {}  →  {} × {}  ·  {:.2}×", preview.width, preview.height, (preview.width as f32 * self.factor).round() as u32, preview.height, self.factor)).small().color(MUTED));
        } else {
            ui.add_space(100.0);
            ui.vertical_centered(|ui| {
                ui.heading("Arrasta fotografias para começar");
                ui.label(RichText::new("RAW · HEIC · JPEG · PNG · TIFF").color(MUTED));
            });
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The OS theme can be applied after creation; enforce the chosen dark palette each frame.
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = PANEL;
        visuals.window_fill = BG;
        visuals.selection.bg_fill = BLUE;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        ctx.set_visuals(visuals);
        if let Some(rx) = &self.rx { if let Ok(message) = rx.try_recv() { self.status = message; self.busy = false; self.rx = None; } }
        let dropped: Vec<_> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() { self.add_paths(dropped); }
        self.load_next_thumbnail(ctx);
        egui::SidePanel::left("sidebar").exact_width(205.0).frame(panel()).show(ctx, |ui| {
            self.logo(ui);
            ui.add_space(36.0);
            if ui.add_sized([170.0, 36.0], egui::Button::new("◉  Processar").fill(if self.view == View::Process { BLUE } else { CARD })).clicked() { self.view = View::Process; }
            if ui.add_sized([170.0, 36.0], egui::Button::new(format!("▦  Lote  ({})", self.photos.len())).fill(if self.view == View::Batch { BLUE } else { CARD })).clicked() { self.view = View::Batch; }
            ui.add_space(28.0);
            ui.label(RichText::new("IMPORTAR").small().color(MUTED));
            if ui.button("＋  Fotografias").clicked() { self.import_files(); }
            if ui.button("▣  Pasta").clicked() { self.import_folder(); }
            ui.add_space(22.0);
            ui.label(RichText::new(format!("{} selecionadas de {}", self.selected_count(), self.photos.len())).color(MUTED));
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.label(RichText::new("João Costa ©").small().color(MUTED));
            });
        });
        egui::SidePanel::right("settings").exact_width(290.0).frame(panel()).show(ctx, |ui| {
            ui.heading(RichText::new("Ajustes").color(Color32::WHITE));
            ui.label(RichText::new(if self.view == View::Batch { "Aplicar às selecionadas" } else { "Fotografia ativa" }).color(MUTED));
            ui.add_space(22.0);
            card().show(ui, |ui| self.factor_controls(ui));
            ui.add_space(12.0);
            card().show(ui, |ui| self.export_controls(ui));
            ui.add_space(12.0);
            card().show(ui, |ui| self.destination_controls(ui));
            ui.add_space(14.0);
            ui.label(RichText::new("●  Originais preservados").color(Color32::from_rgb(126, 218, 173)));
            ui.label(RichText::new("Nomes existentes não são substituídos.").small().color(MUTED));
            ui.add_space(22.0);
            let label = if self.view == View::Batch { format!("Exportar {} fotografias  →", self.selected_count()) } else { "Exportar selecionadas  →".to_string() };
            if ui.add_enabled(!self.busy && self.selected_count() > 0, main_button(label).min_size(egui::vec2(250.0, 42.0))).clicked() { self.export(); }
            ui.add_space(12.0);
            ui.label(RichText::new(&self.status).small().color(MUTED));
        });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(18)).show(ctx, |ui| {
            if self.view == View::Batch {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(RichText::new("Processamento em lote").size(24.0).color(Color32::WHITE));
                        ui.label(RichText::new("Escolhe as fotografias para exportar.").color(MUTED));
                    });
                    ui.add_space(20.0);
                    if ui.button("Selecionar todas").clicked() { for p in &mut self.photos { p.selected = true; } }
                    if ui.button("Limpar seleção").clicked() { for p in &mut self.photos { p.selected = false; } }
                });
                ui.add_space(18.0);
                ui.separator();
                ui.add_space(10.0);
                if self.photos.is_empty() {
                    ui.add_space(120.0);
                    ui.vertical_centered(|ui| {
                        ui.heading("O teu lote começa aqui");
                        if ui.add(main_button("＋  Importar fotografias")).clicked() { self.import_files(); }
                    });
                } else { self.batch_grid(ui); }
            } else {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(RichText::new("Fotografia anamórfica").size(24.0).color(Color32::WHITE));
                        ui.label(RichText::new("Pré-visualiza a proporção antes de exportar.").color(MUTED));
                    });
                    ui.add_space(18.0);
                    for (mode, title) in [(Compare::Original, "Original"), (Compare::Desqueezed, "Descomprimida"), (Compare::Split, "Comparar")] {
                        if ui.add(egui::Button::new(title).fill(if self.compare == mode { BLUE } else { CARD })).clicked() { self.compare = mode; }
                    }
                });
                ui.add_space(12.0);
                egui::TopBottomPanel::bottom("filmstrip").exact_height(145.0).frame(card()).show_inside(ui, |ui| {
                    ui.label(RichText::new("BIBLIOTECA  ·  clica para abrir").small().color(MUTED));
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (index, photo) in self.photos.iter().enumerate() {
                                ui.vertical(|ui| {
                                    if let Some(thumb) = &photo.thumbnail {
                                        if ui.add(egui::Image::new((thumb.id(), egui::vec2(110.0, 72.0))).sense(egui::Sense::click())).clicked() { self.active = index; self.preview = None; }
                                    } else if ui.add_sized([110.0, 72.0], egui::Button::new("Imagem")).clicked() { self.active = index; self.preview = None; }
                                    let name = photo.path.file_name().unwrap_or_default().to_string_lossy();
                                    if ui.selectable_label(self.active == index, name).clicked() { self.active = index; self.preview = None; }
                                });
                                ui.add_space(4.0);
                            }
                        });
                    });
                });
                card().show(ui, |ui| self.preview_content(ui, ctx));
            }
        });
        if self.busy { ctx.request_repaint_after(std::time::Duration::from_millis(100)); }
    }
}
fn main() -> eframe::Result {
    let mut options = eframe::NativeOptions::default();
    options.viewport = egui::ViewportBuilder::default().with_title("AnaDesqueeze").with_inner_size([1380.0, 850.0]).with_min_inner_size([1050.0, 650.0]);
    eframe::run_native("AnaDesqueeze", options, Box::new(|cc| {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut app = App::default();
        let logo = image::load_from_memory(include_bytes!("../assets/logo.png")).expect("logo.png").to_rgba8();
        app.logo = Some(cc.egui_ctx.load_texture("brand-logo", ColorImage::from_rgba_unmultiplied([logo.width() as usize, logo.height() as usize], logo.as_raw()), egui::TextureOptions::LINEAR));
        Ok(Box::new(app))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_preserves_source_and_existing_exports() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("photo.png");
        let img = DynamicImage::new_rgba8(8, 4);
        img.save(&source).unwrap();
        let original = std::fs::read(&source).unwrap();
        let existing = dir.path().join("photo_desqueezed.png");
        std::fs::write(&existing, b"keep me").unwrap();
        let output = save_export(dir.path(), &source, ExportFormat::Png, &squeezed(&img, 2.0)).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert_eq!(std::fs::read(&existing).unwrap(), b"keep me");
        assert_eq!(image::open(output).unwrap().dimensions(), (16, 4));
    }
    #[test]
    fn rgba_can_export_to_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let output = save_export(dir.path(), Path::new("rgba.png"), ExportFormat::Jpeg, &DynamicImage::new_rgba8(8, 4)).unwrap();
        assert_eq!(image::open(output).unwrap().dimensions(), (8, 4));
    }
}

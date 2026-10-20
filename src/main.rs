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

struct Photo { path: PathBuf, selected: bool }
struct Preview { path: PathBuf, factor: f32, texture: TextureHandle, width: u32, height: u32 }
struct App {
    photos: Vec<Photo>, active: usize, factor: f32, custom_factor: f32,
    export_format: ExportFormat, destination: Option<PathBuf>, preview: Option<Preview>,
    busy: bool, status: String, rx: Option<Receiver<String>>,
}
impl Default for App {
    fn default() -> Self { Self { photos: vec![], active: 0, factor: 1.5, custom_factor: 1.5, export_format: ExportFormat::Tiff, destination: None, preview: None, busy: false, status: "Importa fotografias para começar.".into(), rx: None } }
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
        for p in paths { if p.is_file() && is_accepted(&p) && !self.photos.iter().any(|x| x.path == p) { self.photos.push(Photo { path: p, selected: true }); count += 1; } }
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
                let rgba = transformed.to_rgba8();
                let (tw,th) = transformed.dimensions();
                let texture = ctx.load_texture("preview", ColorImage::from_rgba_unmultiplied([tw as usize, th as usize], rgba.as_raw()), egui::TextureOptions::LINEAR);
                self.preview = Some(Preview { path, factor: self.factor, texture, width: w, height: h });
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
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.rx { if let Ok(message) = rx.try_recv() { self.status = message; self.busy = false; self.rx = None; } }
        let dropped: Vec<_> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() { self.add_paths(dropped); }
        egui::SidePanel::left("left").resizable(false).default_width(210.0).show(ctx, |ui| {
            ui.add_space(16.0); ui.heading(RichText::new("◉ AnaDesqueeze").size(20.0).color(Color32::WHITE));
            ui.label(RichText::new("Fotografia anamórfica").color(Color32::GRAY)); ui.add_space(24.0);
            if ui.add(egui::Button::new("＋  Importar fotografias").fill(BLUE)).clicked() {
                if let Some(paths) = rfd::FileDialog::new().pick_files() { self.add_paths(paths); }
            }
            ui.add_space(12.0);
            if ui.button("＋  Importar pasta").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    if let Ok(read) = std::fs::read_dir(dir) { self.add_paths(read.flatten().map(|x| x.path())); }
                }
            }
            ui.add_space(15.0); ui.separator(); ui.add_space(12.0);
            ui.label(RichText::new(format!("Biblioteca · {}", self.photos.len())).strong());
            egui::ScrollArea::vertical().max_height(370.0).show(ui, |ui| {
                let mut next_active = None;
                for (i,p) in self.photos.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut p.selected, "");
                        let name = p.path.file_name().unwrap_or_default().to_string_lossy();
                        if ui.selectable_label(self.active == i, name).clicked() { next_active = Some(i); }
                    });
                }
                if let Some(i) = next_active { self.active = i; self.preview = None; }
            });
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.label(RichText::new("João Costa ©").size(10.0).color(Color32::GRAY));
            });
        });
        egui::SidePanel::right("right").resizable(false).default_width(295.0).show(ctx, |ui| {
            ui.add_space(16.0); ui.heading("Desqueeze & Exportação"); ui.add_space(14.0);
            ui.label("Fator anamórfico");
            ui.horizontal_wrapped(|ui| {
                for factor in [1.25_f32, 1.33, 1.5, 1.6, 1.8, 2.0] {
                    if ui.selectable_label((self.factor-factor).abs() < 0.005, format!("{factor}×")).clicked() { self.factor = factor; self.custom_factor = factor; }
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Personalizado:");
                if ui.add(egui::DragValue::new(&mut self.custom_factor).speed(0.01).range(0.5..=4.0)).changed() { self.factor = self.custom_factor; }
            });
            ui.add_space(16.0); ui.separator(); ui.add_space(12.0);
            ui.label("Formato de exportação");
            ui.horizontal(|ui| { for fmt in [ExportFormat::Jpeg, ExportFormat::Png, ExportFormat::Tiff] { ui.selectable_value(&mut self.export_format, fmt, fmt.label()); } });
            ui.add_space(9.0);
            ui.label(RichText::new("DNG RAW: em desenvolvimento; não é exportado nesta versão.").small().color(Color32::LIGHT_GRAY));
            ui.add_space(16.0); ui.separator(); ui.add_space(12.0);
            ui.label("Pasta de destino");
            if ui.button("Escolher pasta…").clicked() { self.destination = rfd::FileDialog::new().pick_folder(); }
            if let Some(dir) = &self.destination { ui.label(RichText::new(dir.to_string_lossy()).small().color(Color32::LIGHT_GRAY)); }
            ui.add_space(10.0);
            ui.label(RichText::new("✓ Originais sempre preservados").color(Color32::from_rgb(112, 218, 171)));
            ui.label(RichText::new("✓ Sem sobrescrever exportações").color(Color32::from_rgb(112, 218, 171)));
            ui.add_space(22.0);
            if ui.add_enabled(!self.busy, egui::Button::new("Exportar fotografias  →").fill(BLUE).min_size(egui::vec2(260.0, 42.0))).clicked() { self.export(); }
        });
        egui::CentralPanel::default().frame(egui::Frame::default().fill(BG).inner_margin(18.0)).show(ctx, |ui| {
            ui.heading(RichText::new("A verdadeira proporção das tuas imagens.").size(24.0));
            ui.label(RichText::new("Pré-visualização anamórfica · Os ficheiros originais nunca são alterados.").color(Color32::GRAY));
            ui.add_space(20.0);
            self.load_preview(ctx);
            if let Some(preview) = &self.preview {
                let available = ui.available_size();
                let ratio = preview.texture.size_vec2().x / preview.texture.size_vec2().y;
                let w = available.x.max(1.0).min((available.y - 100.0).max(150.0) * ratio);
                let size = egui::vec2(w, w / ratio);
                ui.vertical_centered(|ui| { ui.image((preview.texture.id(), size)); });
                ui.add_space(12.0);
                ui.label(format!("Original: {} × {}    |    Desqueeze {:.2}×: {} × {}", preview.width, preview.height, self.factor, (preview.width as f32 * self.factor).round() as u32, preview.height));
            } else {
                ui.add_space(140.0);
                ui.vertical_centered(|ui| { ui.heading("Arrasta aqui as tuas fotografias"); ui.label("RAW · DNG · HEIC · JPEG · PNG · TIFF"); });
            }
            ui.add_space(16.0);
            ui.label(RichText::new(&self.status).color(Color32::from_rgb(174, 192, 216)));
        });
        if self.busy { ctx.request_repaint_after(std::time::Duration::from_millis(100)); }
    }
}
fn main() -> eframe::Result {
    let mut options = eframe::NativeOptions::default();
    options.viewport = egui::ViewportBuilder::default().with_title("AnaDesqueeze").with_inner_size([1250.0, 760.0]).with_min_inner_size([920.0, 560.0]);
    eframe::run_native("AnaDesqueeze", options, Box::new(|cc| {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(25, 30, 38);
        visuals.window_fill = BG;
        visuals.selection.bg_fill = BLUE;
        cc.egui_ctx.set_visuals(visuals);
        Ok(Box::new(App::default()))
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

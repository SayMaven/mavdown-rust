use eframe::egui;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[derive(Serialize, Deserialize, Clone)]
struct AppConfig {
    output_path: String,
}

struct MavenApp {
    url: String,
    output_path: String,
    log_text: Arc<Mutex<String>>,
    progress: Arc<Mutex<f32>>,
    is_busy: Arc<Mutex<bool>>,
    title_info: Arc<Mutex<String>>,
    thumbnail: Arc<Mutex<Option<egui::TextureHandle>>>,
    
    // Opsi Download
    mode_video: bool,
    video_codec: String,
    audio_codec: String,
    audio_only_format: String,
    container: String,
    resolution: String,
    use_aria2: bool,
    embed_thumb: bool,
    download_subs: bool,
    embed_subs: bool,
    subs_lang: String,
}

impl MavenApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        let font_data = include_bytes!("../cjk_font.ttf");
        fonts.font_data.insert("my_cjk_font".to_owned(), egui::FontData::from_static(font_data));
        fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "my_cjk_font".to_owned());
        fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().push("my_cjk_font".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        let default_dir = "./downloads".to_string();
        let config_path = "./config.json";
        let output_path = if Path::new(config_path).exists() {
            let data = fs::read_to_string(config_path).unwrap_or_default();
            let config: AppConfig = serde_json::from_str(&data).unwrap_or(AppConfig { output_path: default_dir.clone() });
            config.output_path
        } else {
            default_dir
        };

        fs::create_dir_all(&output_path).ok();

        Self {
            url: String::new(),
            output_path,
            log_text: Arc::new(Mutex::new(String::from("=== Maven Downloader Siap ===\n"))),
            progress: Arc::new(Mutex::new(0.0)),
            is_busy: Arc::new(Mutex::new(false)),
            title_info: Arc::new(Mutex::new(String::from("(Tekan Get Info)"))),
            thumbnail: Arc::new(Mutex::new(None)),
            
            mode_video: true,
            video_codec: "h264".to_string(),
            audio_codec: "m4a".to_string(),
            audio_only_format: "mp3".to_string(),
            container: "mp4".to_string(),
            resolution: "1080".to_string(),
            use_aria2: true,
            embed_thumb: true,
            download_subs: false,
            embed_subs: false,
            subs_lang: "id,en".to_string(),
        }
    }

    fn save_config(&self) {
        let config = AppConfig { output_path: self.output_path.clone() };
        if let Ok(json) = serde_json::to_string(&config) {
            fs::write("./config.json", json).ok();
        }
    }
}

impl eframe::App for MavenApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        // --- RAHASIA UI SCALING (ZOOM IN / ZOOM OUT) ---
        // 1. Ambil ukuran jendela fisik dari Windows
        let (physical_width, physical_height) = ctx.input(|i| {
            let rect = i.screen_rect;
            let ppp = i.pixels_per_point;
            (rect.width() * ppp, rect.height() * ppp)
        });

        // 2. Ini resolusi dasar aplikasi lu (ukuran normal)
        let base_width = 1150.0;
        let base_height = 700.0;

        // 3. Hitung rasio tarikan jendela
        let scale_w = physical_width / base_width;
        let scale_h = physical_height / base_height;

        // 4. Pakai rasio terkecil biar gak ada yang kepotong, batasi zoom dari 40% sampai 300%
        let target_scale = scale_w.min(scale_h).clamp(0.4, 3.0);
        
        // 5. Terapkan skala ke seluruh UI!
        ctx.set_pixels_per_point(target_scale);
        // -----------------------------------------------

        // HANYA ADA SATU PANEL: CENTRAL PANEL
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(10.0))
            .show(ctx, |ui| {
                
                // Bungkus semuanya dalam ScrollArea agar aman kalau resolusi monitor user terlalu kecil
                egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                    
                    // --- 1. KOTAK ATAS (FIXED LEBAR: 1110px) ---
                    ui.vertical(|ui| {
                        let total_width = 1110.0;
                        ui.set_min_width(total_width);
                        ui.set_max_width(total_width);

                        egui::Frame::group(ui.style()).inner_margin(10.0).show(ui, |ui| {
                            ui.label(egui::RichText::new("1. Masukkan URL Video:").strong());
                            ui.text_edit_singleline(&mut self.url);
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                let is_busy = *self.is_busy.lock().unwrap();
                                if ui.add_sized([180.0, 35.0], egui::Button::new("🔍 Get Info (Judul & Preview)")).clicked() && !is_busy {
                                    self.get_info(ctx.clone());
                                }
                                if ui.add_sized([180.0, 35.0], egui::Button::new("⬇ START DOWNLOAD")).clicked() && !is_busy {
                                    self.start_download(ctx.clone());
                                }
                            });

                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label("Folder Output:");
                                ui.label(egui::RichText::new(&self.output_path).color(egui::Color32::LIGHT_BLUE));
                                if ui.button("📂 Pilih Folder").clicked() {
                                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                        self.output_path = folder.display().to_string();
                                        self.save_config();
                                    }
                                }
                                if ui.button("📁 Buka Folder").clicked() {
                                    open::that(&self.output_path).ok();
                                }
                            });

                            ui.add_space(8.0);
                            let current_progress = *self.progress.lock().unwrap();
                            ui.add(egui::ProgressBar::new(current_progress / 100.0)
                                .text(format!("Progress: {:.1}%", current_progress))
                                .animate(*self.is_busy.lock().unwrap()));
                        });

                        ui.add_space(10.0);

                        // --- 2. KOTAK BAWAH (3 KOLOM KAKU) ---
                        ui.horizontal(|ui| {
                            let fixed_height = 450.0; // Tinggi paksa agar sejajar rapi

                            // KOLOM 1: INFO VIDEO (FIXED LEBAR: 320px)
                            ui.vertical(|ui| {
                                ui.set_min_width(320.0);
                                ui.set_max_width(320.0);
                                ui.set_min_height(fixed_height);
                                
                                egui::Frame::group(ui.style()).inner_margin(10.0).show(ui, |ui| {
                                    ui.set_min_height(fixed_height);
                                    ui.label(egui::RichText::new("📺 1. Info Video").strong().size(14.0));
                                    ui.separator();
                                    let title = self.title_info.lock().unwrap().clone();
                                    ui.label(egui::RichText::new(title).size(15.0).strong().color(egui::Color32::WHITE));
                                    ui.add_space(10.0);
                                    
                                    if let Some(texture) = self.thumbnail.lock().unwrap().as_ref() {
                                        let available_width = ui.available_width();
                                        let aspect_ratio = texture.size()[0] as f32 / texture.size()[1] as f32;
                                        let display_height = available_width / aspect_ratio;
                                        ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(available_width, display_height)));
                                    } else {
                                        ui.centered_and_justified(|ui| { ui.label("Preview Thumbnail\nAkan Muncul Disini"); });
                                    }
                                });
                            });

                            // KOLOM 2: SETTINGS (FIXED LEBAR: 410px)
                            ui.vertical(|ui| {
                                ui.set_min_width(410.0);
                                ui.set_max_width(410.0);
                                ui.set_min_height(fixed_height);

                                egui::Frame::group(ui.style()).inner_margin(10.0).show(ui, |ui| {
                                    ui.set_min_height(fixed_height);
                                    ui.label(egui::RichText::new("⚙ 2. Pilihan Download & Kualitas").strong().size(14.0));
                                    ui.separator();
                                    
                                    ui.label(egui::RichText::new("Mode Download:").color(egui::Color32::GRAY));
                                    ui.horizontal(|ui| {
                                        ui.radio_value(&mut self.mode_video, true, "Video + Audio");
                                        ui.radio_value(&mut self.mode_video, false, "Audio Only");
                                    });
                                    ui.add_space(8.0);

                                    if self.mode_video {
                                        ui.label(egui::RichText::new("Video Codec:").color(egui::Color32::GRAY));
                                        ui.horizontal(|ui| {
                                            ui.radio_value(&mut self.video_codec, "h264".to_string(), "H.264");
                                            ui.radio_value(&mut self.video_codec, "h265".to_string(), "H.265");
                                            ui.radio_value(&mut self.video_codec, "vp9".to_string(), "VP9");
                                            ui.radio_value(&mut self.video_codec, "av1".to_string(), "AV1");
                                            ui.radio_value(&mut self.video_codec, "best".to_string(), "Best");
                                        });
                                        ui.add_space(8.0);

                                        ui.label(egui::RichText::new("Audio Codec (Bawaan Video):").color(egui::Color32::GRAY));
                                        ui.horizontal(|ui| {
                                            ui.radio_value(&mut self.audio_codec, "m4a".to_string(), "M4A");
                                            ui.radio_value(&mut self.audio_codec, "opus".to_string(), "Opus");
                                            ui.radio_value(&mut self.audio_codec, "mp3".to_string(), "MP3"); 
                                            ui.radio_value(&mut self.audio_codec, "best".to_string(), "Best");
                                        });
                                        ui.add_space(8.0);

                                        ui.label(egui::RichText::new("Container:").color(egui::Color32::GRAY));
                                        ui.horizontal(|ui| {
                                            ui.radio_value(&mut self.container, "mp4".to_string(), "MP4");
                                            ui.radio_value(&mut self.container, "mkv".to_string(), "MKV");
                                        });
                                        ui.add_space(8.0);

                                        ui.label(egui::RichText::new("Resolusi:").color(egui::Color32::GRAY));
                                        ui.horizontal(|ui| {
                                            for res in ["360", "480", "720", "1080", "1440", "2160", "best"] {
                                                ui.radio_value(&mut self.resolution, res.to_string(), if res == "best" { "Best" } else { res });
                                            }
                                        });
                                    } else {
                                        ui.label(egui::RichText::new("Format Audio:").color(egui::Color32::GRAY));
                                        ui.horizontal(|ui| {
                                            ui.radio_value(&mut self.audio_only_format, "mp3".to_string(), "MP3");
                                            ui.radio_value(&mut self.audio_only_format, "m4a".to_string(), "M4A");
                                            ui.radio_value(&mut self.audio_only_format, "flac".to_string(), "FLAC");
                                            ui.radio_value(&mut self.audio_only_format, "wav".to_string(), "WAV");
                                        });
                                    }

                                    ui.add_space(15.0);
                                    ui.label(egui::RichText::new("🛠 3. Opsi Tambahan").strong().size(14.0));
                                    ui.separator();
                                    ui.checkbox(&mut self.use_aria2, "Gunakan Aria2c (Multi-thread)");
                                    ui.checkbox(&mut self.embed_thumb, "Gabungkan Thumbnail ke File");
                                    ui.checkbox(&mut self.download_subs, "Download Subtitle");
                                    ui.checkbox(&mut self.embed_subs, "Gabungkan Subtitle ke File");
                                    
                                    ui.add_space(5.0);
                                    ui.horizontal(|ui| {
                                        ui.label("Bahasa Subs:");
                                        ui.text_edit_singleline(&mut self.subs_lang);
                                    });
                                });
                            });

                            // KOLOM 3: LOGS (FIXED LEBAR: 360px)
                            ui.vertical(|ui| {
                                ui.set_min_width(360.0);
                                ui.set_max_width(360.0);
                                ui.set_min_height(fixed_height);

                                egui::Frame::group(ui.style()).inner_margin(10.0).show(ui, |ui| {
                                    ui.set_min_height(fixed_height);
                                    ui.label(egui::RichText::new("📝 5. Status/Log Unduhan").strong().size(14.0));
                                    ui.separator();
                                    
                                    let log_content = self.log_text.lock().unwrap().clone();
                                    // ScrollArea vertikal khusus untuk teks log
                                    egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                                        ui.add(egui::TextEdit::multiline(&mut log_content.as_str())
                                            .desired_width(f32::INFINITY)
                                            // Memaksa textbox log mengambil seluruh sisa tinggi frame
                                            .desired_rows(20)
                                            .font(egui::TextStyle::Monospace));
                                    });
                                });
                            });

                        }); // Akhir Horizontal Bawah
                    }); // Akhir Vertical Wrapper
                }); // Akhir ScrollArea Utama
            });

        if *self.is_busy.lock().unwrap() {
            ctx.request_repaint();
        }
    }
}

impl MavenApp {
    fn get_info(&mut self, ctx: egui::Context) {
        if self.url.trim().is_empty() { return; }
        
        *self.is_busy.lock().unwrap() = true;
        *self.title_info.lock().unwrap() = "Mengambil Info...".to_string();
        *self.thumbnail.lock().unwrap() = None;

        let url = self.url.clone();
        let title_clone = self.title_info.clone();
        let thumb_clone = self.thumbnail.clone();
        let busy_clone = self.is_busy.clone();

        tokio::spawn(async move {
            let output = Command::new("yt-dlp")
                .args(["--dump-json", "--skip-download", "--no-warnings", &url])
                .output()
                .await;

            if let Ok(out) = output {
                if let Ok(json) = serde_json::from_slice::<Value>(&out.stdout) {
                    if let Some(title) = json["title"].as_str() {
                        *title_clone.lock().unwrap() = title.to_string();
                    }
                    if let Some(thumb_url) = json["thumbnail"].as_str() {
                        if let Ok(resp) = reqwest::get(thumb_url).await {
                            if let Ok(bytes) = resp.bytes().await {
                                if let Ok(image) = image::load_from_memory(&bytes) {
                                    let image = image.resize_exact(300, 180, image::imageops::FilterType::Lanczos3);
                                    let size = [image.width() as _, image.height() as _];
                                    let image_buffer = image.to_rgba8();
                                    let pixels = image_buffer.into_flat_samples();
                                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                                    let texture = ctx.load_texture("thumbnail", color_image, Default::default());
                                    *thumb_clone.lock().unwrap() = Some(texture);
                                }
                            }
                        }
                    }
                }
            } else {
                *title_clone.lock().unwrap() = "Gagal mengambil info! (Cek URL/Koneksi)".to_string();
            }
            *busy_clone.lock().unwrap() = false;
            ctx.request_repaint();
        });
    }

    fn start_download(&mut self, ctx: egui::Context) {
        if self.url.trim().is_empty() { return; }

        *self.is_busy.lock().unwrap() = true;
        *self.progress.lock().unwrap() = 0.0;
        
        {
            let mut log_w = self.log_text.lock().unwrap();
            log_w.clear();
            log_w.push_str(&format!("Mempersiapkan unduhan untuk: {}\n", self.url));
        }

        let url = self.url.clone();
        let output_dir = self.output_path.clone();
        let use_aria2 = self.use_aria2;
        let mode_video = self.mode_video;
        let v_codec = self.video_codec.clone();
        let a_codec = self.audio_codec.clone();
        let container = self.container.clone();
        let res = self.resolution.clone();
        let a_only_format = self.audio_only_format.clone();
        let mut final_embed_thumb = self.embed_thumb;
        let dl_subs = self.download_subs;
        let embed_subs = self.embed_subs;
        let subs_lang = self.subs_lang.clone();

        let log_clone = self.log_text.clone();
        let prog_clone = self.progress.clone();
        let is_busy_clone = self.is_busy.clone();

        tokio::spawn(async move {
            let output_format = format!("{}/%(title)s.%(ext)s", output_dir);
            let mut args = vec![
                "--no-colors".to_string(),
                "--retries".to_string(), "infinite".to_string(),
                "-o".to_string(), output_format,
                "--ffmpeg-location".to_string(), ".".to_string(),
            ];

            if use_aria2 {
                {
                    let mut lw = log_clone.lock().unwrap();
                    lw.push_str("🚀 [Sistem] Mode Aria2c (Multi-thread 16x) Aktif!\n");
                }
                args.push("--external-downloader".to_string());
                #[cfg(target_os = "windows")]
                args.push("./aria2c.exe".to_string());
                #[cfg(not(target_os = "windows"))]
                args.push("./aria2c".to_string());
                args.push("--external-downloader-args".to_string());
                args.push("-x 16 -k 1M".to_string()); 
            }

            if mode_video {
                let mut f_v = String::from("bestvideo");
                if res != "best" {
                    f_v.push_str(&format!("[height<={}]", res));
                }

                let mut f_v_codec = f_v.clone();
                if v_codec != "best" {
                    let vc_map = match v_codec.as_str() {
                        "h264" => "avc", "h265" => "hevc", "vp9" => "vp09", "av1" => "av01", _ => ""
                    };
                    if !vc_map.is_empty() {
                        f_v_codec.push_str(&format!("[vcodec~={}]", vc_map));
                    }
                }

                let mut f_a = String::from("bestaudio");
                if a_codec != "best" {
                    let ac_map = match a_codec.as_str() {
                        "m4a" => "mp4a", 
                        "opus" => "opus",
                        "mp3" => "mp3", 
                        _ => ""
                    };
                    if !ac_map.is_empty() {
                        f_a.push_str(&format!("[acodec~={}]", ac_map));
                    }
                }

                let format_string = format!(
                    "{vc}+{a} / {vc}+bestaudio / {v}+{a} / {v}+bestaudio / best",
                    vc = f_v_codec, v = f_v, a = f_a
                );
                
                args.push("-f".to_string());
                args.push(format_string);
                args.push("--merge-output-format".to_string());
                args.push(container);
            } else {
                args.push("-f".to_string());
                args.push("bestaudio/best".to_string());
                args.push("--extract-audio".to_string());
                args.push("--audio-format".to_string());
                args.push(a_only_format.clone());
            }

            if !mode_video && a_only_format == "wav" && final_embed_thumb {
                final_embed_thumb = false;
                let mut lw = log_clone.lock().unwrap();
                lw.push_str("⚠️ [Sistem] Format WAV tidak mendukung Embed Thumbnail. Fitur otomatis dimatikan.\n");
            }

            if final_embed_thumb { args.push("--embed-thumbnail".to_string()); }
            
            if dl_subs || embed_subs {
                let lang = if subs_lang.trim().is_empty() { "all".to_string() } else { subs_lang };
                args.push("--write-subs".to_string());
                args.push("--sub-langs".to_string());
                args.push(lang);
                if embed_subs { args.push("--embed-subs".to_string()); }
                if dl_subs {
                    args.push("--sub-format".to_string());
                    args.push(if mode_video { "srt".to_string() } else { "lrc".to_string() });
                }
            }

            args.push(url);

            #[cfg(target_os = "windows")]
            let ytdlp_cmd = "./yt-dlp.exe";
            #[cfg(not(target_os = "windows"))]
            let ytdlp_cmd = "./yt-dlp";

            let mut child = Command::new(ytdlp_cmd)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Gagal memanggil yt-dlp. Pastikan yt-dlp.exe ada di folder yang sama!");

            let mut stdout = child.stdout.take().unwrap();
            let mut buf = [0u8; 512];
            let mut line_buffer = String::new();
            
            let re = Regex::new(r"(\d+(?:\.\d+)?)%").unwrap();

            loop {
                match stdout.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let chunk = String::from_utf8_lossy(&buf[..n]);
                        for c in chunk.chars() {
                            if c == '\r' || c == '\n' {
                                if !line_buffer.trim().is_empty() {
                                    if let Some(caps) = re.captures(&line_buffer) {
                                        if let Ok(pct) = caps[1].parse::<f32>() {
                                            *prog_clone.lock().unwrap() = pct;
                                        }
                                    }
                                    if c == '\n' {
                                        let mut lw = log_clone.lock().unwrap();
                                        lw.push_str(&line_buffer);
                                        lw.push('\n');
                                    }
                                }
                                line_buffer.clear();
                            } else {
                                line_buffer.push(c);
                            }
                        }
                        ctx.request_repaint();
                    }
                    Err(_) => break,
                }
            }

            let _ = child.wait().await;
            
            {
                let mut lw = log_clone.lock().unwrap();
                lw.push_str("\n--- UNDUHAN SELESAI ---\n");
            }
            *prog_clone.lock().unwrap() = 100.0;
            *is_busy_clone.lock().unwrap() = false;
            ctx.request_repaint();
        });
    }
}

#[tokio::main]
async fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 700.0]) 
            .with_title("Maven Downloader (By SayMaven) V1.4 - Rust Edition"),
        ..Default::default()
    };

    eframe::run_native(
        "Maven Downloader",
        options,
        Box::new(|cc| Box::new(MavenApp::new(cc))),
    )
}
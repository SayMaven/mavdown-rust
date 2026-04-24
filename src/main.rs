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
    custom_command: String,
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
            custom_command: String::new(),
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

        // Modifikasi styling untuk tampilan lebih modern
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.visuals.window_rounding = egui::Rounding::same(8.0);
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(6.0);
        ctx.set_style(style);

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
                
                // --- 1. KOTAK ATAS ---
                ui.vertical(|ui| {
                    ui.set_min_width(ui.available_width());

                    egui::Frame::group(ui.style()).inner_margin(12.0).show(ui, |ui| {
                        ui.label(egui::RichText::new("1. Masukkan URL Video:").strong());
                        
                        let url_edit = egui::TextEdit::singleline(&mut self.url).desired_width(f32::INFINITY);
                        ui.add(url_edit);
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            let is_busy = *self.is_busy.lock().unwrap();
                            let available_width = ui.available_width();
                            let btn_width = (available_width - 8.0) / 2.0;
                            
                            if ui.add_sized([btn_width, 35.0], egui::Button::new("Get Info (Judul & Preview)")).clicked() && !is_busy {
                                self.get_info(ctx.clone());
                            }
                            if ui.add_sized([btn_width, 35.0], egui::Button::new("START DOWNLOAD")).clicked() && !is_busy {
                                self.start_download(ctx.clone());
                            }
                        });

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label("Folder Output:");
                            ui.label(egui::RichText::new(&self.output_path).color(egui::Color32::from_rgb(100, 150, 255)));
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let is_busy = *self.is_busy.lock().unwrap();
                                if ui.button("Update yt-dlp").clicked() && !is_busy {
                                    self.update_ytdlp(ctx.clone());
                                }
                                if ui.button("Buka Folder Hasil").clicked() {
                                    open::that(&self.output_path).ok();
                                }
                                if ui.button("Pilih Folder Output").clicked() {
                                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                        self.output_path = folder.display().to_string();
                                        self.save_config();
                                    }
                                }
                            });
                        });

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let current_progress = *self.progress.lock().unwrap();
                            let status_text = if *self.is_busy.lock().unwrap() {
                                if current_progress == 0.0 { "Memproses..." } else { "Mengunduh..." }
                            } else {
                                "Siap"
                            };
                            ui.label(format!("Progress: {}", status_text));
                            ui.add(egui::ProgressBar::new(current_progress / 100.0)
                                .animate(*self.is_busy.lock().unwrap()));
                        });
                    });

                    ui.add_space(10.0);

                    // --- 2. KOTAK BAWAH (3 KOLOM) ---
                    ui.horizontal(|ui| {
                        let dynamic_height = ui.available_height(); // Gunakan semua sisa tinggi
                        let total_avail_w = ui.available_width();
                        let spacing = ui.style().spacing.item_spacing.x * 2.0; 
                        let col1_w = (total_avail_w - spacing) * 0.28;
                        let col2_w = (total_avail_w - spacing) * 0.40;
                        let col3_w = (total_avail_w - spacing) * 0.32;

                        // KOLOM 1: INFO VIDEO
                        ui.vertical(|ui| {
                            ui.set_min_width(col1_w);
                            ui.set_max_width(col1_w);
                            
                            egui::Frame::group(ui.style()).inner_margin(12.0).show(ui, |ui| {
                                ui.set_min_height(dynamic_height);
                                ui.label(egui::RichText::new("1. Info Video").strong().size(14.0));
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
                                    ui.centered_and_justified(|ui| { 
                                        ui.label(egui::RichText::new("Preview Thumbnail").color(egui::Color32::GRAY)); 
                                    });
                                }
                            });
                        });

                        // KOLOM 2: SETTINGS (2., 3., 4.)
                        ui.vertical(|ui| {
                            ui.set_min_width(col2_w);
                            ui.set_max_width(col2_w);
                            ui.set_min_height(dynamic_height);

                            egui::Frame::group(ui.style()).inner_margin(12.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("2. Pilihan Download & Kualitas").strong().size(14.0));
                                ui.separator();
                                
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Mode Download:").strong());
                                    ui.radio_value(&mut self.mode_video, true, "Video + Audio");
                                    ui.radio_value(&mut self.mode_video, false, "Audio Only");
                                });
                                ui.add_space(8.0);

                                if self.mode_video {
                                    ui.horizontal(|ui| {
                                        ui.label("Video Codec:");
                                        ui.radio_value(&mut self.video_codec, "h264".to_string(), "H.264");
                                        ui.radio_value(&mut self.video_codec, "vp9".to_string(), "VP9");
                                        ui.radio_value(&mut self.video_codec, "av1".to_string(), "AV1");
                                        ui.radio_value(&mut self.video_codec, "h265".to_string(), "H.265");
                                        ui.radio_value(&mut self.video_codec, "best".to_string(), "Best");
                                    });
                                    ui.add_space(8.0);

                                    ui.horizontal(|ui| {
                                        ui.label("Audio Codec:");
                                        ui.radio_value(&mut self.audio_codec, "m4a".to_string(), "M4A");
                                        ui.radio_value(&mut self.audio_codec, "opus".to_string(), "Opus");
                                        ui.radio_value(&mut self.audio_codec, "best".to_string(), "Best");
                                    });
                                    ui.add_space(8.0);

                                    ui.horizontal(|ui| {
                                        ui.label("Container:");
                                        ui.radio_value(&mut self.container, "mp4".to_string(), "MP4");
                                        ui.radio_value(&mut self.container, "mkv".to_string(), "MKV");
                                    });
                                    ui.add_space(8.0);

                                    ui.horizontal_wrapped(|ui| {
                                        ui.label("Max. Resolusi:");
                                        for res in ["360p", "480p", "720p", "1080p", "1440p", "2160p", "Best"] {
                                            let val = if res == "Best" { "best".to_string() } else { res.replace("p", "") };
                                            ui.radio_value(&mut self.resolution, val, res);
                                        }
                                    });
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label("Format Audio:");
                                        ui.radio_value(&mut self.audio_only_format, "mp3".to_string(), "MP3 (Kompatibilitas)");
                                        ui.radio_value(&mut self.audio_only_format, "m4a".to_string(), "M4A (Kualitas Asli)");
                                    });
                                }
                            });

                            ui.add_space(8.0);

                            egui::Frame::group(ui.style()).inner_margin(12.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("3. Opsi Downloader, Metadata & Subtitle").strong().size(14.0));
                                ui.separator();
                                
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut self.use_aria2, "Aria2c (Multi-thread)");
                                    ui.checkbox(&mut self.embed_thumb, "Gabung Thumbnail");
                                });
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut self.download_subs, "Download Subtitle");
                                    ui.checkbox(&mut self.embed_subs, "Gabung Subtitle");
                                });
                                
                                ui.add_space(5.0);
                                ui.horizontal(|ui| {
                                    ui.label("Bahasa (misal: id,en,ja,all):");
                                    ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut self.subs_lang));
                                });
                            });

                            ui.add_space(8.0);

                            egui::Frame::group(ui.style()).inner_margin(12.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("4. Custom Command").strong().size(14.0));
                                ui.separator();
                                ui.add(egui::TextEdit::singleline(&mut self.custom_command).desired_width(f32::INFINITY));
                                ui.label(egui::RichText::new("Contoh: --skip-download --write-thumbnail. Untuk download thumbnail saja.")
                                    .color(egui::Color32::from_rgb(100, 150, 255))
                                    .size(12.0));
                            });
                        });

                        // KOLOM 3: LOGS
                        ui.vertical(|ui| {
                            ui.set_min_width(col3_w);
                            ui.set_max_width(col3_w);

                            egui::Frame::group(ui.style()).inner_margin(12.0).show(ui, |ui| {
                                ui.set_min_height(dynamic_height);
                                ui.label(egui::RichText::new("5. Status/Log Unduhan (Real-time)").strong().size(14.0));
                                ui.separator();
                                
                                let log_content = self.log_text.lock().unwrap().clone();
                                egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                                    ui.add(egui::TextEdit::multiline(&mut log_content.as_str())
                                        .desired_width(f32::INFINITY)
                                        .font(egui::TextStyle::Monospace));
                                });
                            });
                        });

                    }); // Akhir Horizontal Bawah
                }); // Akhir Vertical Wrapper
            });

        if *self.is_busy.lock().unwrap() {
            ctx.request_repaint();
        }
    }
}

impl MavenApp {
    fn update_ytdlp(&mut self, ctx: egui::Context) {
        *self.is_busy.lock().unwrap() = true;
        let log_clone = self.log_text.clone();
        let is_busy_clone = self.is_busy.clone();
        
        tokio::spawn(async move {
            {
                let mut lw = log_clone.lock().unwrap();
                lw.push_str("Memulai update yt-dlp...\n");
            }
            
            #[cfg(target_os = "windows")]
            let ytdlp_cmd = "./yt-dlp.exe";
            #[cfg(not(target_os = "windows"))]
            let ytdlp_cmd = "./yt-dlp";

            let output = Command::new(ytdlp_cmd)
                .arg("-U")
                .output()
                .await;
                
            let mut lw = log_clone.lock().unwrap();
            match output {
                Ok(out) => {
                    lw.push_str(&String::from_utf8_lossy(&out.stdout));
                    lw.push_str(&String::from_utf8_lossy(&out.stderr));
                    lw.push_str("\nUpdate selesai.\n");
                }
                Err(e) => {
                    lw.push_str(&format!("Gagal update: {}\n", e));
                }
            }
            *is_busy_clone.lock().unwrap() = false;
            ctx.request_repaint();
        });
    }

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
            #[cfg(target_os = "windows")]
            let ytdlp_cmd = "./yt-dlp.exe";
            #[cfg(not(target_os = "windows"))]
            let ytdlp_cmd = "./yt-dlp";

            let output = Command::new(ytdlp_cmd)
                .args(["--dump-json", "--skip-download", "--no-warnings", &url])
                .output()
                .await;

            if let Ok(out) = output {
                if let Ok(json) = serde_json::from_slice::<Value>(&out.stdout) {
                    if let Some(title) = json["title"].as_str() {
                        *title_clone.lock().unwrap() = title.to_string();
                    } else {
                        *title_clone.lock().unwrap() = "Judul tidak ditemukan!".to_string();
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
                } else {
                    let err_msg = String::from_utf8_lossy(&out.stderr);
                    if !err_msg.trim().is_empty() {
                        let first_line = err_msg.lines().next().unwrap_or("Error tak diketahui");
                        let mut msg = first_line.to_string();
                        if msg.len() > 60 {
                            msg.truncate(57);
                            msg.push_str("...");
                        }
                        *title_clone.lock().unwrap() = format!("Error: {}", msg);
                    } else {
                        *title_clone.lock().unwrap() = "Gagal mem-parsing info yt-dlp!".to_string();
                    }
                }
            } else {
                *title_clone.lock().unwrap() = "Gagal mengambil info! (Cek URL/Koneksi/yt-dlp)".to_string();
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
        let custom_cmd = self.custom_command.clone();

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

            if !custom_cmd.trim().is_empty() {
                // Gunakan parser sederhana (split by whitespace)
                for arg in custom_cmd.split_whitespace() {
                    args.push(arg.to_string());
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
            .with_inner_size([1150.0, 760.0]) 
            .with_title("Maven Downloader (By SayMaven) V1.4 - Rust Edition"),
        ..Default::default()
    };

    eframe::run_native(
        "Maven Downloader",
        options,
        Box::new(|cc| Box::new(MavenApp::new(cc))),
    )
}
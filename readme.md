# 🚀 Maven Downloader V1.4 - Rust Edition

Maven Downloader is a lightning-fast, portable, and feature-rich video/audio downloader built from the ground up using **Rust** and the **Egui** framework. Originally written in Python (Tkinter), this project has been completely rewritten to deliver native performance, a responsive UI, and seamless portability.

Powered by `yt-dlp` and `aria2c` under the hood, it offers granular control over your downloads without sacrificing user experience.

![Maven Downloader Screenshot](https://res.cloudinary.com/ds4a54vuy/image/upload/v1775159887/Screenshot_mavdown_rust.png) 

## ✨ Features

- 🦀 **Blazing Fast & Lightweight:** Built with Rust, ensuring minimal memory footprint and instant startup times.
- 🎨 **Scalable Dark Mode UI:** A clean, modern interface powered by Egui. It features a fixed, robust layout with automatic UI scaling (Zoom in/out) that adapts perfectly to any screen resolution without breaking the layout.
- ⚡ **Multi-Threaded Downloading:** Integrates **Aria2c** to accelerate download speeds up to 16x by utilizing multiple concurrent connections.
- 🎬 **Granular Quality Control:**
  - Choose between **Video + Audio** or **Audio Only**.
  - Select preferred Video Codecs (`H.264`, `H.265`, `VP9`, `AV1`) and Audio Codecs (`M4A`, `Opus`, `MP3`).
  - Max Resolution capping (up to 4K/Best).
- 🏷️ **Metadata & Subtitles:** Automatically embed thumbnails (ID3 tags) and download/embed subtitles (SRT/LRC) in multiple languages.
- 📦 **100% Portable:** No installation required. Just keep the executable in the same folder as its dependencies (`yt-dlp`, `ffmpeg`, `aria2c`) and run it anywhere!
- 🇯🇵 **CJK Font Support:** Fully supports Japanese, Korean, and Chinese characters natively in the UI.

## 🛠️ Requirements & Installation

Maven Downloader is designed to be portable. To run the application, you need the compiled `.exe` and its three core backend tools in the **same directory**.

1. **Download the Dependencies:**
   - [yt-dlp](https://github.com/yt-dlp/yt-dlp/releases) (For extracting and downloading)
   - [Aria2c](https://github.com/aria2/aria2/releases) (For multi-threaded downloading)
   - [FFmpeg](https://ffmpeg.org/download.html) (For merging video and audio streams)

2. **Folder Structure:**
   Ensure your folder looks exactly like this before running the app:
   ```text
   Maven_Downloader_Folder/
   ├── maven_downloader.exe   <-- The Rust App
   ├── yt-dlp.exe             
   ├── aria2c.exe
   └── ffmpeg.exe
   ```

3. **Run `maven_downloader.exe` and enjoy!**

## 💻 Building from Source

If you want to compile the Rust code yourself, ensure you have [Rust & Cargo](https://www.rust-lang.org/tools/install) installed.

1. **Clone the repository:**
   ```bash
   git clone https://github.com/YourUsername/maven_downloader.git
   cd maven_downloader
   ```

2. **Add the CJK Font:**
   - Download a CJK font (e.g., *Noto Sans JP*).
   - Rename it to `cjk_font.ttf` and place it in the root directory (right next to `Cargo.toml`). The compiler will embed this font directly into the executable.

3. **Build the Release Version:**
   ```bash
   cargo build --release
   ```
   *The compiled executable will be located in `target/release/maven_downloader.exe`.*

## ⚙️ How It Works (The Rust Magic)

- **Egui (Immediate Mode GUI):** The UI is redrawn every frame, providing buttery-smooth animations and eliminating the layout-collapsing issues common in traditional retained-mode GUIs like Tkinter.
- **Tokio (Async Runtime):** Used to spawn background tasks (`yt-dlp` subprocesses) so the UI never freezes during heavy downloads.
- **UI Scaling:** Implements dynamic `pixels_per_point` scaling based on OS window sizing, allowing the application to zoom smoothly without breaking the fixed layout constraints.
- **Byte-Stream Parsing:** Instead of waiting for new lines (`\n`), the app reads the `yt-dlp` and `aria2c` stdout byte-by-byte. This ensures the progress bar is truly real-time and buttery smooth.

## 🤝 Contributing
Pull requests are welcome! If you find any bugs or have feature requests, feel free to open an issue.

## 📜 License
[MIT License](LICENSE)

---
*Created with ❤️ by SayMaven.*
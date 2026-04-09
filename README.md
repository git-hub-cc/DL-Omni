# 🚀 DL-Omni

**DL-Omni** 是一款基于 **Tauri 2** 和 **Svelte 5 (Runes)** 开发的全能流媒体下载工具。它秉持极简美学设计理念，通过 Rust 原生性能与 `yt-dlp` 强大的解析能力，为用户提供极致的音视频下载体验。

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Tauri](https://img.shields.io/badge/Tauri-2.0-orange.svg)
![Svelte](https://img.shields.io/badge/Svelte-5.0-ff3e00.svg)
![Rust](https://img.shields.io/badge/Rust-2021-000000.svg)

---

## ✨ 核心特性

-   **🎯 精准链接下载**：支持 Bilibili、YouTube、TikTok 等全球数百个主流流媒体平台的链接解析与下载。
-   **🔍 猫抓级高级嗅探**：提供独立的 Webview 容器，基于多路底层拦截自动提取 `.m3u8`、`.mp4` 等媒体流地址；支持自定义命名模板与正则黑名单过滤。
-   **⚡ 高性能下载引擎**：
    -   **双轨制调度**：智能切换 `yt-dlp` 托管下载与 Rust 原生多线程直链下载。
    -   **并发控制**：支持自定义最大同时下载任务数及单任务分片线程数。
-   **🛡️ 鲁棒性设计**：
    -   **断点续传**：基于 SQLite 数据库的任务持久化，支持应用崩溃后的进度恢复。
    -   **磁盘预警**：在下载前自动检查目标磁盘空间，防止写入失败。
-   **🔄 引擎自动更新**：无需更新整个应用，即可通过 GitHub API 自动检测并下载最新的 `yt-dlp` 解析核心与 `FFmpeg` 合并引擎。
-   **🎨 极简美学 UI**：采用 Svelte 5 Runes 实现细粒度响应式更新，配合 Tailwind CSS v4 构建的高级感深色模式界面。

---

## 🛠️ 技术栈

-   **前端 (Frontend)**: [Svelte 5](https://svelte.dev/) (Runes), TypeScript, [Tailwind CSS v4](https://tailwindcss.com/)
-   **后端 (Backend)**: [Tauri 2](https://v2.tauri.app/), [Rust](https://www.rust-lang.org/)
-   **解析引擎**: [yt-dlp](https://github.com/yt-dlp/yt-dlp), [FFmpeg](https://ffmpeg.org/)
-   **数据库**: [SQLite](https://www.sqlite.org/) (通过 `rusqlite` 实现任务持久化)
-   **进程通信**: Tauri IPC (Commands & Events)

---

## 📦 环境准备

在开始之前，请确保您的开发环境已安装以下工具：

1.  **Node.js**: 18.x 或更高版本。
2.  **Rust**: 最新的稳定版 (通过 [rustup](https://rustup.rs/) 安装)。
3.  **系统依赖**:
    -   **Windows**: 安装 [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)。
    -   **macOS/Linux**: 参考 [Tauri 官方安装指南](https://v2.tauri.app/start/prerequisites/) 安装必要的系统库。

---

## 🚀 快速开始

### 1. 克隆项目
```bash
git clone https://github.com/your-username/dl-omni.git
cd dl-omni
```

### 2. 安装前端依赖
```bash
npm install
```

### 3. 启动开发模式
开发模式下，Tauri 会自动启动 Rust 后端服务并打开应用窗口：
```bash
npm run tauri dev
```

---

## 🏗️ 生产打包

根据您的操作系统，执行以下命令进行应用打包。生成的安装包将位于 `src-tauri/target/release/bundle/` 目录下。

```bash
# 通用打包命令
npm run build
npm run tauri build
```

> **注意**：首次打包或运行预览版时，Rust 后端会自动从 GitHub 下载最新的 `yt-dlp` 及 `FFmpeg` 二进制文件并存放于应用的 `AppData/bin` 目录。

---

## 📂 项目结构

```text
├── src/                  # 前端源码 (SvelteKit)
│   ├── lib/
│   │   ├── api/          # IPC 通信接口
│   │   ├── components/   # UI 组件 (Sidebar, TitleBar, ProgressBar等)
│   │   ├── stores/       # Svelte 5 状态树 (TaskStore, ConfigStore)
│   │   └── types/        # TypeScript 类型定义
│   └── routes/           # 页面路由 (任务、嗅探、设置、历史)
├── src-tauri/            # 后端源码 (Rust)
│   ├── src/
│   │   ├── engine/       # 下载逻辑 (yt-dlp 封装, FFmpeg管理, 嗅探脚本注入)
│   │   ├── commands.rs   # 前端调用接口映射
│   │   ├── database.rs   # SQLite 任务管理
│   │   ├── config.rs     # 配置文件持久化
│   │   └── state.rs      # 全局状态与事件聚合器
│   └── tauri.conf.json   # Tauri 配置文件
└── static/               # 静态资源
```

---

## 🛠️ 常见问题

-   **无法解析 1080P/4K 视频 / 需要会员权限？**
    请在“全局设置”中开启“使用内置浏览器 Cookie”，并先前往左侧“嗅探”页面访问目标网站完成登录（内置引擎会自动同步您的登录状态）。
-   **嗅探器无法捕获链接？**
    请确保在弹出的嗅探窗口中实际点击了播放按钮，部分网站需要开始缓冲后才会触发媒体流请求。
-   **更新核心失败？**
    请检查网络是否可以访问 GitHub API，或在设置中配置代理地址（支持 HTTP/SOCKS5）。

---

## 📄 开源协议

本项目基于 [MIT License](LICENSE) 协议开源。
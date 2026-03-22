# CyberSnatcher — Phase 1

A cyberpunk-themed desktop video downloader built with **Tauri 2.x + React 19 + Rust**.

## Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/) (stable)
- Tauri 2 system dependencies:
  - **Windows**: WebView2 (usually pre-installed on Win10/11), Visual Studio C++ Build Tools
  - **macOS**: Xcode Command Line Tools
  - **Linux**: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`

## Setup

```bash
cd cybersnatcher

# Install frontend deps
npm install

# Run in dev mode (starts Vite + compiles Rust + opens window)
npm run tauri dev
```

First launch will take a while as Rust compiles all dependencies. Subsequent runs are fast.

## Build for Production

```bash
npm run tauri build
```

The binary will be in `src-tauri/target/release/`.

## Project Structure

```
cybersnatcher/
├── src/                          # React frontend
│   ├── App.tsx                   # Main layout
│   ├── main.tsx                  # Entry point
│   ├── index.css                 # Global styles + theme
│   ├── components/
│   │   ├── TitleBar.tsx          # Custom drag-region title bar
│   │   ├── StatusBar.tsx         # Bottom status bar
│   │   ├── Sidebar/
│   │   │   ├── Sidebar.tsx       # Sidebar container (collapsible)
│   │   │   ├── UrlInput.tsx      # URL paste + detect badge + Snatch button
│   │   │   ├── QueueList.tsx     # Download queue list
│   │   │   └── QueueItem.tsx     # Individual queue item
│   │   ├── MainView/
│   │   │   ├── EmptyState.tsx    # "Paste a URL" splash
│   │   │   ├── DownloadView.tsx  # Active download detail view
│   │   │   ├── ProgressBar.tsx   # Animated purple progress bar
│   │   │   └── StatsGrid.tsx     # Speed, ETA, size stats
│   │   └── Settings/
│   │       └── SettingsModal.tsx  # Settings overlay
│   ├── hooks/
│   │   ├── useDownloads.ts       # Download state + helpers
│   │   └── useSettings.ts        # Settings state
│   ├── lib/
│   │   ├── tauri.ts              # Tauri invoke wrappers
│   │   └── types.ts              # TS types matching Rust types
│   └── stores/
│       └── downloadStore.ts      # Zustand store + mock data
│
├── src-tauri/                    # Rust backend
│   ├── tauri.conf.json           # Tauri 2.x config (no decorations, 1100x700)
│   ├── Cargo.toml
│   ├── capabilities/
│   │   └── default.json          # Window permissions
│   └── src/
│       ├── main.rs               # Binary entry
│       ├── lib.rs                # Tauri builder + command registration
│       ├── types.rs              # Shared types + URL detection
│       └── commands/
│           ├── mod.rs
│           ├── analyze.rs        # analyze_url placeholder
│           ├── download.rs       # start/pause/cancel/resume placeholders
│           └── settings.rs       # get_settings / set_download_folder
│
├── tailwind.config.js            # Cyberpunk color palette
├── postcss.config.js
├── vite.config.ts
├── tsconfig.json
└── package.json
```

## What's Included (Phase 1)

- Full cyberpunk black/purple UI theme
- Custom title bar with drag region + window controls (min/max/close)
- Collapsible sidebar with URL input, auto-detect badges, download queue
- Main area with empty state splash + detailed download view
- Animated progress bars with purple gradient + shimmer
- Stats grid (speed, ETA, file size, format)
- Collapsible terminal-style output log
- Settings modal with all options
- Bottom status bar with live counts
- 4 mock downloads in different states (Complete, Downloading, Queued, Failed)
- Custom scrollbar, scanline overlay, selection colors
- All Tauri IPC commands scaffolded (placeholder logic)
- Zustand store for state management

## What's NOT Built Yet

- yt-dlp integration (Phase 2)
- ffmpeg integration (Phase 3)
- HLS/DASH downloading (Phase 4)
- Settings persistence to disk
- Auto-update system
- System tray

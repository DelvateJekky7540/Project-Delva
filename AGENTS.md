# Project Delva - Agent Instructions

## 🎯 Project Overview
Project Delva is a Windows desktop widget inspired by Apple's "Dynamic Island", built using Tauri v2 and Vanilla TypeScript (Vite). It features a floating, transparent pill-shaped window, a custom mascot (Delva), and an expanded "Command Center" UI.

## 🛠️ Tech Stack
- **Framework:** Tauri v2 (Rust backend)
- **Frontend:** Vanilla HTML, CSS, TypeScript (Vite)
- **Styling:** Pure CSS (no Tailwind/frameworks). Uses CSS variables for theming (glassmorphism effect).
- **Icons:** Inline SVGs (Lucide-style).

## 📂 Project Structure
- `src/` : Frontend code (HTML, CSS, TS).
- `src-tauri/` : Rust backend code.
- `index.html` : Main UI file (contains both Compact and Expanded views).

## ⚠️ CRITICAL RULES (Tauri v2 Specifics)
1. **Tauri v2 Permissions:** This project uses Tauri v2. NEVER use Tauri v1 allowlists. Whenever you add a new Tauri API/Plugin (e.g., `getCurrentWindow`, `system-info`), you MUST add the corresponding permission to `src-tauri/capabilities/default.json`.
2. **Window Management:** The main window is already set to transparent, undecorated, and always-on-top in `src-tauri/tauri.conf.json`. Do not remove these settings.
3. **Compile Times:** Rust compilation takes time. Do not assume the app is broken if the terminal hangs for 1-2 minutes during `npm run tauri dev`.
4. **Resizing Window:** The window should resize dynamically when the Dynamic Island expands or collapses. Use `getCurrentWindow().setSize()` from `@tauri-apps/api/window`.

## 🎨 Frontend Rules
- Keep the UI extremely lightweight (this is a desktop widget, avoid heavy JS libraries).
- Use standard DOM manipulation (`document.getElementById`) instead of React/Vue states.
- Preserve the CSS variables (`--bg-color`, `--accent`, etc.) used for the glass effect.
- Animations should use CSS transitions or keyframes. 

## 🦀 Backend (Rust) Rules
- Use `tauri::command` for functions exposed to the frontend.
- Keep Rust code modular. Place plugins and commands logically inside `src-tauri/src/lib.rs`.

## 🤖 Current Status
- UI (Compact & Expanded views) is fully built.
- Data in Command Center (CPU, RAM, Battery) is currently a mockup/static.
- Next tasks: Integrate `tauri-plugin-system-info`, dynamic window resizing, and System Tray icon.
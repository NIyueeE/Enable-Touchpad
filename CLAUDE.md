# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Tauri desktop application built with:
- **Frontend**: Preact + TypeScript + Vite
- **Backend**: Rust with Tauri 2.0
- **Purpose**: Touchpad control application with system tray functionality

## Architecture

The application follows a multi-platform touchpad control architecture:

### Core Components:
- `src-tauri/src/core/` - Platform-agnostic touchpad control logic
  - `state.rs` - Application state management
  - `hotkey_manager.rs` - Global hotkey handling
  - `input_controller.rs` - Platform-specific touchpad control
  - `mouse_emulator.rs` - Mouse emulation functionality
- `src-tauri/src/tray.rs` - System tray implementation
- `src-tauri/src/osd.rs` - On-screen display notifications
- `src-tauri/src/commands.rs` - Tauri commands for frontend communication

### Frontend Structure:
- `src/App.tsx` - Main Preact component (currently basic template)
- Uses Tauri's invoke API to call Rust commands

## Development Commands

### Frontend (JavaScript/TypeScript):
```bash
npm run dev      # Start Vite dev server
npm run build    # Build frontend (tsc + vite build)
npm run preview  # Preview built frontend
npm run tauri    # Run Tauri CLI commands
```

### Backend (Rust):
```bash
cd src-tauri
cargo build      # Build Rust code
cargo run        # Run application (with frontend)
cargo check      # Type check Rust code
```

### Full Application:
```bash
npm run tauri dev    # Develop with hot reload
npm run tauri build  # Build production application
```

## Platform Support

The application supports multiple platforms with platform-specific dependencies:
- **Windows**: Windows API for input control
- **macOS**: Objective-C bindings
- **Linux**: X11 library for input control

## Key Features
- System tray-only application (main window hidden)
- Global hotkey support for touchpad toggle
- On-screen display notifications
- Cross-platform touchpad control
- Permission handling for accessibility features

## Development Notes
- The application is designed to run primarily from system tray
- Frontend is currently a basic template - main functionality is in Rust backend
- Uses Tauri's security model with capability-based permissions
- Logging is implemented with `simple_logger` for debugging
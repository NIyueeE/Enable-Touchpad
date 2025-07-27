# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Tauri application with Preact frontend and TypeScript, built with Vite.

## Key Commands

- `npm run dev`: Start development server
- `npm run build`: Build project (TS compile + Vite build)
- `npm run preview`: Preview production build
- `npm run tauri`: Run Tauri commands

## Architecture

- Frontend: Preact components in `src/`
- Backend: Tauri (Rust) in `src-tauri/`
- Build: Vite + TypeScript
- IPC: `@tauri-apps/api` for frontend-backend communication
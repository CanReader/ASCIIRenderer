```
 █████╗ ███████╗ ██████╗██╗██╗██████╗
██╔══██╗██╔════╝██╔════╝██║██║╚════██╗
███████║███████╗██║     ██║██║ █████╔╝
██╔══██║╚════██║██║     ██║██║ ╚═══██╗
██║  ██║███████║╚██████╗██║██║██████╔╝
╚═╝  ╚═╝╚══════╝ ╚═════╝╚═╝╚═╝╚═════╝
                        3D  RENDERER
```

**Real-time 3D software rasterizer that streams rendered frames as ASCII art to your browser over WebSocket.**

![Rust](https://img.shields.io/badge/Rust-2021-orange?style=flat-square&logo=rust)
![React](https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=black)
![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?style=flat-square&logo=typescript&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-4caf50?style=flat-square)
![WebSocket](https://img.shields.io/badge/WebSocket-30_FPS-8A2BE2?style=flat-square)

---

## What you see when you run it

A full-screen terminal-style interface opens in your browser: a phosphor-green ASCII mesh of the Utah Teapot (or whichever model you select) slowly auto-rotating against a near-black background, rendered through a CRT scanline filter with a subtle vignette. Characters drawn from a dense 70-glyph ramp give the illusion of continuous shading — bright `@` and `#` on lit surfaces fading to `.` and `'` at the terminator, then empty space beyond. Switch to `texture` mode and a Duck or Damaged Helmet renders in full per-character RGB color directly on an HTML5 Canvas. A slim left panel lets you swap shading modes, charsets, color profiles, and lighting in real time; every change is forwarded over the open WebSocket and the next frame already reflects it.

---

## Features

### Rendering
▸ Software rasterizer written from scratch in Rust — perspective projection, z-buffering, barycentric interpolation, Blinn-Phong specular  
▸ 30 FPS frame loop driven by `tokio::time::interval`; rasterization offloaded to a blocking thread pool via `spawn_blocking`  
▸ Up to **2400 × 1200** character resolution, configurable per session  
▸ **7 shading modes:** phong (default) · flat · depth · normal map · wireframe · texture · texture+lit  
▸ **6 charsets:** standard (70-glyph dense ramp) · simple · blocks · dots · binary · matrix  
▸ **6 color modes:** phosphor green · amber · white · cyan · red · purple — rendered as CRT glow in CSS  
▸ Texture mode bypasses the monochrome pipeline and paints per-character RGB via HTML5 Canvas  
▸ Invert toggle, adjustable ambient, and free light-direction vector  
▸ Auto-rotate with per-axis speed control; drag-to-rotate disables auto-rotate automatically

### Formats
▸ **OBJ** — full triangulation via `tobj`, smooth normals computed when mesh omits them  
▸ **glTF / GLB** — triangles, triangle strips, triangle fans; PBR base-color textures extracted and sampled at UV coordinates  
▸ **FBX** — binary FBX 7.4+ via `fbxcel-dom`; fan-triangulation of arbitrary polygons  
▸ Upload limit: 200 MB; accepted extensions: `.obj` `.gltf` `.glb` `.fbx`  
▸ All uploaded models are held in a lock-free `DashMap`; built-in examples are immutable and survive session restarts

### Built-in examples (no upload needed)
▸ **Procedural:** Cube · Sphere (24×16 UV) · Torus (32×24 segments)  
▸ **Classic OBJ:** Teapot · Suzanne · Spot · Bunny · Armadillo  
▸ **Textured GLB:** Duck · Damaged Helmet · Avocado

### Architecture
▸ Single production binary — Rust backend serves the Vite-built frontend as static files via `tower-http`  
▸ TypeScript types auto-generated from Rust structs (`RenderParams`) via `ts-rs`; no hand-maintained duplicates  
▸ Zero dependencies on a GPU, WebGL, or a graphics driver

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  Browser                                                        │
│                                                                 │
│  ┌──────────────┐   WebSocket /ws/render   ┌─────────────────┐ │
│  │ useRenderer  │ ──── init + params ────▶ │   ws_handler    │ │
│  │  (React hook)│ ◀─── frame + colors ──── │   (Axum)        │ │
│  └──────┬───────┘        30 FPS            └────────┬────────┘ │
│         │                                           │           │
│  ┌──────▼───────┐                        ┌─────────▼────────┐  │
│  │ AsciiDisplay │                        │   render loop    │  │
│  │              │                        │  tokio::interval │  │
│  │  <pre> mode  │                        │  (33 ms tick)    │  │
│  │  monochrome  │                        └─────────┬────────┘  │
│  │              │                                  │           │
│  │  <canvas>    │                        ┌─────────▼────────┐  │
│  │  RGB texture │                        │ spawn_blocking   │  │
│  └──────────────┘                        │  render_frame()  │  │
│                                          │  · MVP transform │  │
│  ┌──────────────┐                        │  · rasterize     │  │
│  │  Controls    │  HTTP POST /api/upload │  · shade         │  │
│  │  (sidebar)   │ ─────────────────────▶ │  · UV sample     │  │
│  └──────────────┘                        └─────────┬────────┘  │
│                                                     │           │
└─────────────────────────────────────────────────────────────────┘
                                                      │
                              ┌───────────────────────┤
                              │   Axum AppState       │
                              │                       │
                              │  ┌─────────────────┐  │
                              │  │  DashMap         │  │
                              │  │  model store     │  │
                              │  │  (Arc<Model>)    │  │
                              │  └─────────────────┘  │
                              │                       │
                              │  ┌─────────────────┐  │
                              │  │  ServeDir        │  │
                              │  │  frontend/dist   │  │
                              │  │  (fallback SPA)  │  │
                              │  └─────────────────┘  │
                              └───────────────────────┘
```

**Data flow per frame:**

```
Client params update
  → JSON over WebSocket → drained each tick
  → RenderParams struct updated in render loop
  → spawn_blocking(render_frame)
      → MVP matrix build (Rust Mat4)
      → per-triangle: clip → NDC → screen-space
      → rasterize_triangle (barycentric, z-test)
      → intensity_to_char (charset ramp)
      → UV sample → RGB (texture mode only)
  → Framebuffer::to_string() + colors_hex()
  → JSON Frame message → WebSocket → browser
  → AsciiDisplay: <pre> or <canvas> render
```

---

## Getting started

### Prerequisites

- Rust stable toolchain (`rustup install stable`)
- Node.js 18+ and npm

### Development (two terminals)

```bash
# Terminal 1 — backend (auto-reloads on cargo run)
make dev-backend
# Listening on http://0.0.0.0:3000

# Terminal 2 — frontend (Vite hot reload)
make dev-frontend
# open http://localhost:5173
```

The frontend dev server proxies `/api` and `/ws` to the backend on port 3000.

### Production (single binary)

```bash
make build
# → generates TypeScript bindings
# → npm run build  (outputs to frontend/dist/)
# → cargo build --release

./backend/target/release/ascii-renderer
# open http://localhost:3000
```

The binary locates `frontend/dist` at `./frontend/dist` by default. Override with:

```bash
STATIC_DIR=/path/to/dist ./backend/target/release/ascii-renderer
```

---

## Usage guide

### Loading a model

Open the **models** tab in the left panel. Built-in examples load instantly — click any entry to start streaming. To load your own file, drag it onto the drop zone or click to browse. The file is uploaded to `POST /api/upload`, parsed on the server, normalized to fit a unit sphere, and immediately available for rendering.

### Controls

| Input | Action |
|-------|--------|
| Drag (left button) | Rotate model · disables auto-rotate |
| Scroll | Zoom in / out |
| `Tab` | Show / hide panel |
| `Space` | Toggle auto-rotate |
| `R` | Reset rotation and zoom |
| `W` | Toggle wireframe mode |
| `+` / `-` | Zoom in / out |

### Shading modes

| Mode | Description |
|------|-------------|
| `phong` | Blinn-Phong with per-vertex normals and specular highlight (default) |
| `flat` | Face normals — sharp-faceted look, one shade per triangle |
| `depth` | Proximity to camera mapped to brightness |
| `normal map` | View-space normal visualized as intensity |
| `wireframe` | Edge lines only, no fill |
| `texture` | Per-character RGB sampled from the model's base-color texture |
| `texture+lit` | Texture multiplied by Phong diffuse — shaded and colored |

**Texture note:** texture modes are only effective on models that carry UV coordinates and embedded textures (the GLB examples: Duck, Damaged Helmet, Avocado). On OBJ or FBX models without textures the mode falls back to black. The browser switches from `<pre>` to `<canvas>` rendering automatically when color data is present.

---

## Generating TypeScript types

`RenderParams` is defined once in Rust and exported to TypeScript via [`ts-rs`](https://github.com/Aleph-Alpha/ts-rs):

```bash
make generate-types
# equivalent to:
cd backend && cargo test --test generate_bindings
```

This writes `frontend/src/bindings/RenderParams.ts`. The file is committed to the repository so the frontend always compiles without requiring a prior Rust build. Re-run this command whenever the `RenderParams` struct in `backend/src/renderer.rs` changes — the TypeScript file should be updated in the same PR.

---

## Supported formats

| Format | Extension | Notes |
|--------|-----------|-------|
| Wavefront OBJ | `.obj` | Triangulated via `tobj`; smooth normals computed when absent; UV coordinates preserved |
| glTF / GLB | `.gltf` · `.glb` | Binary or JSON; PBR base-color textures extracted; triangle strip and fan primitives supported |
| FBX | `.fbx` | Binary FBX 7.4+ via `fbxcel-dom`; arbitrary polygons fan-triangulated; UVs not yet extracted |

---

## REST API

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/models` | List all loaded models (examples + uploads) |
| `POST` | `/api/upload` | Upload a model file (multipart/form-data, max 200 MB) |
| `DELETE` | `/api/models/:id` | Remove an uploaded model (built-in examples return 403) |
| `GET` | `/ws/render` | Upgrade to WebSocket render stream |

### WebSocket protocol

**Client → Server**

```jsonc
// Start a render session
{ "type": "init", "model_id": "<uuid>", "width": 160, "height": 60 }

// Update render parameters (sent on every UI change)
{ "type": "update_params", "params": { /* RenderParams */ } }

// Keep-alive
{ "type": "ping" }
```

**Server → Client**

```jsonc
// Sent once after init
{ "type": "model_info", "name": "Teapot", "vertex_count": 18960, "face_count": 6320 }

// Sent once when the render loop is ready
{ "type": "ready" }

// Sent at ~30 FPS
{
  "type": "frame",
  "content": "<ascii string, newline-delimited rows>",
  "elapsed_ms": 12,
  "colors": "<hex string, 6 chars per cell, null when no texture>"
}

{ "type": "pong" }
{ "type": "error", "message": "..." }
```

---

## Project layout

```
ASCIIRenderer/
├── Makefile                    · build / dev / generate-types targets
├── backend/
│   ├── Cargo.toml
│   ├── assets/                 · embedded model files (OBJ + GLB)
│   └── src/
│       ├── main.rs             · Axum router, startup, static file serving
│       ├── api.rs              · HTTP handlers, WebSocket render loop, AppState
│       ├── renderer.rs         · RenderParams, Framebuffer, rasterizer, render_frame
│       ├── model.rs            · Mesh/Model/Texture types, OBJ/glTF/FBX loaders
│       ├── examples.rs         · procedural geometry (cube, sphere, torus)
│       ├── math.rs             · Vec2, Vec3, Vec4, Mat4 (no external math crate)
│       └── lib.rs              · crate root (for integration tests)
└── frontend/
    ├── package.json
    ├── vite.config.ts
    └── src/
        ├── App.tsx             · layout, keyboard shortcuts, tab routing
        ├── bindings/
        │   └── RenderParams.ts · auto-generated by ts-rs — do not edit
        ├── components/
        │   ├── AsciiDisplay.tsx · <pre>/<canvas> renderer, CRT CSS classes
        │   ├── Controls.tsx     · shading / charset / lighting panel
        │   └── FileUpload.tsx   · drag-drop upload, example model list
        └── hooks/
            └── useRenderer.ts  · WebSocket state machine, drag/scroll handling
```

---

## License

MIT — see [LICENSE](LICENSE).

---

> Built by [@CanReader](https://github.com/CanReader) · [github.com/CanReader/ASCIIRenderer](https://github.com/CanReader/ASCIIRenderer)

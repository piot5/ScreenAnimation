# ScreenAnimation

GPU-beschleunigte Bildschirmanimationen und Wallpaper-Engine für Windows. Nutzt WGPU für hardwarebeschleunigtes Rendering mit direkter Windows-Integration.

## Features

- WGPU-basiertes Rendering mit WGSL-Shadern
- Multi-Monitor-Unterstützung mit individuellen Fenstern
- Zwei Betriebsmodi:
  - **V1 (Einfach)**: Kontinuierliche Animation mit Mausinteraktion
  - **V2 (Sequenz)**: Zeitbasierte Sequenzen mit mehreren Shader- und Sound-Stufen
- Audio-Wiedergabe via Rodio (WAV-Format)
- Wallpaper-Modus (Einbettung hinter Desktop-Icons)
- Transparente Overlay-Fenster für Animationen
- Konfigurierbare Parameter und Logik-Variablen

## Voraussetzungen

- Windows 10/11
- Rust Toolchain (rustup)
- Visual Studio Build Tools 2019+ (für Windows-Kompatibilität)
- GPU mit WGPU-Unterstützung (Vulkan, DX12, Metal)

## Installation

```bash
# Repository klonen
git clone <repository-url>
cd Build_ScreenAnimation

# Release-Build
cargo build --release

# Binary befindet sich in:
# target\release\animationengine.exe
```

## Verwendung

### Allgemeine Syntax

```bash
animationengine.exe <MODUS> <PFAD>
```

### Modi

#### Animation-Modus (V1)
Öffnet ein transparentes Overlay-Fenster über dem gesamten Bildschirm:

```bash
animationengine.exe Animation assets\animation1\animation1.flow
```

**Merkmale:**
- Transparentes, anklickbares Fenster
- Mausposition wird an Shader übergeben
- 60 FPS Rendering-Loop
- Überlagert alle anderen Fenster

#### Wallpaper-Modus (V1/V2)
Einbettung als Desktop-Hintergrund hinter Icons:

```bash
animationengine.exe Wallpaper assets\wallpaper1\wallpaper1.flow
```

**Merkmale:**
- Nutzt WorkerW-Fenster-Hierarchie von Windows
- Keine Interaktion mit Maus
- Rendert hinter Desktop-Icons

### .flow-Paket-Format

Ein .flow-Paket ist eine ZIP-Archive mit folgender Struktur:

```
animation.flow
├── config.toml       # Konfiguration
├── shader.wgsl       # WGSL-Shader-Code
├── sound1.wav        # Audio-Dateien
├── sound2.wav
├── texture.png       # Optionale Texturen
└── background.png    # Wallpaper-Hintergrund (optional)
```

#### config.toml Struktur

```toml
# V1-Modus: Einzelner Shader
mode = "animation"  # oder "wallpaper"
shader = "fs_default"
direction = "forward"
z_order = "top"

# Logik-Parameter (an Shader übergeben)
[p1] = 1.0
[p2] = 0.0

# Feature-Flags
[f1] = true
[f2] = false

# Audio-Lautstärke
volume = 0.5
```

#### WGSL-Shader-Struktur

**Uniforms (automatisch gebunden):**
```wgsl
struct Uniforms {
    mouse: vec2<f32>,      // Mausposition (0-1)
    offset: vec2<f32>,     // Offset
    scale: f32,            // Skalierung
    time: f32,             // Laufzeit in Sekunden
    logic_params: vec4<f32>, // p1-p4 aus config
    feature_flags: vec4<f32> // f1-f4 aus config
}

@group(0) @binding(0) var tex0: texture_2d<f32>;
@group(0) @binding(1) var samp0: sampler;
@group(0) @binding(2) var tex1: texture_2d<f32>;  // Optional
@group(0) @binding(3) var samp1: sampler;         // Optional
@group(1) @binding(0) var<uniform> u: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    // Standard-Quad: 6 Vertices (2 Dreiecke)
    var pos = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0),
        vec2(-1.0, 1.0), vec2(1.0, -1.0), vec2(1.0, 1.0)
    );
    return vec4(pos[vid], 0.0, 1.0);
}

@fragment
fn fs_default(@builtin(position) coord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = coord.xy / vec2<f32>(1920.0, 1080.0);
    let color = textureSample(tex0, samp0, uv);
    return color;
}
```

#### V2-Sequenz-Modus

Definiert eine zeitbasierte Abfolge von Shader/Sound-Kombinationen:

```toml
mode = "sequence"

# Einzelne Sequenz-Schritte
[[sequence]]
name = "intro"
duration_ms = 3000
shader_entry = "fs_intro"
sound = "intro.wav"
easing = "easeInOut"

[[sequence]]
name = "main"
duration_ms = 5000
shader_entry = "fs_main"
sound = "main.wav"
texture = "texture.png"

# Shader-Entry-Points müssen in shader.wgsl definiert sein
```

**Sequenz-Logik:**
- Jeder Schritt läuft für `duration_ms` Millisekunden
- `duration_ms = 0` bedeutet Endlos-Schleife
- Shader-Eintritte müssen in `shader.wgsl` als `@fragment`-Funktionen definiert sein
- Sounds werden synchron zum Schrittstart abgespielt

## Architektur

### Projektstruktur

```
src/
├── lib.rs                    # Crate-Root, öffentliche API
├── engine.rs                 # WGPU-Core: Gerät, Queue, Pipelines
├── gpu_core.rs              # GPU-Abstraktion (Deprecated)
├── loader.rs                # .flow-Paket-Lader
├── logic.rs                 # Logik-Engine (Uniform-Berechnung)
├── windows.rs               # Windows-API: Fenster, Desktop-Integration
├── system_integration.rs    # Alternative Fenster-Implementierung
└── bin/
    └── animationengine.rs   # Hauptprogramm (CLI)
```

### Module

#### engine
Kern-Abstraktionen für WGPU:
- `GpuCore`: Gerät, Queue, Bind-Group-Layouts, Sampler, Render-Pipelines
- `Uniforms`: Vereinheitlichtes Uniform-Buffer-Layout (208 Bytes)
- `WindowWrapper`: Raw-Window-Handle-Implementierung für WGPU-Surfaces

#### loader
Liest .flow-Pakete (ZIP-Archive):
- `FlowPackage`: Container für Config, Shader, Sounds, Textures
-parst `config.toml` und `shader.wgsl`
- Extrahiert WAV-Sounds und Bilder

#### logic
Berechtet Frame-Update-Parameter:
- `LogicEngine`: `start_time`-basiertes Zeitmodell
- `update()`: Erzeugt `Uniforms` aus Flow-Konfiguration und Mausposition

#### windows
Windows-API-Integration:
- `MonitorWindow`: Repräsentiert ein Fenster pro Monitor
- `init_windows()`: Erstellt Fenster auf allen Bildschirmen
- `GpuCore::fetch_worker_w()`: Findet WorkerW-Fenster für Wallpaper-Modus

### Datenfluss

1. **Initialisierung**:
   ```
   FlowPackage::load() → Config + Shader + Assets
   ↓
   GpuCore::new() → Gerät, Pipelines, Bind-Groups
   ↓
   init_windows() → WindowWrapper + Surface + Texture pro Monitor
   ```

2. **Render-Loop (V1)**:
   ```
   GetCursorPos() → Mauskoordinaten
   ↓
   LogicEngine::update() → Uniforms (Maus, Offset, Skalierung, Zeit)
   ↓
   write_buffer() → GPU-Buffer aktualisieren
   ↓
   get_current_texture() → Swapchain-Frame
   ↓
   Render-Pass → Set Pipeline, Bind Groups, Draw(6)
   ↓
   queue.submit() → Frame an GPU übergeben
   ```

3. **Sequenz-Modus (V2)**:
   ```
   Für jeden Step in sequence:
   - Start-Zeit merken
   - Sound abspielen
   - Render-Loop bis duration_ms abgelaufen
   ↓
   Nächster Step
   ```

## Building

### Debug-Build
```bash
cargo build
```

### Release-Build (optimiert)
```bash
cargo build --release
```

**Release-Optimierungen:**
- `opt-level = "z"` (Größenoptimierung)
- `lto = true` (Link-Time-Optimization)
- `codegen-units = 1` (Maximale Optimierung)
- `panic = "abort"` (Keine Unwind-Tabellen)
- `strip = true` (Symboltabelle entfernen)

### Windows-Crate Features

Die `windows`-Crate erfordert explizite Feature-Flags:

```toml
windows = {
    version = "0.54",
    features = [
        "Win32_Graphics_Gdi",
        "Win32_UI_WindowsAndMessaging",
        "Win32_System_LibraryLoader",
        "Win32_UI_HiDpi"
    ]
}
```

## Troubleshooting

### Build-Fehler: `could not find Win32`
→ Windows-Crate Features in `Cargo.toml` fehlen

### Fehler: `Arc<Vec<u8>>: AsRef<[u8]>`
→ Sound-Daten werden als `Arc<Vec<u8>>` gespeichert, für Decoder dereferenzieren

### Fenster erscheint nicht (Wallpaper-Modus)
→ WorkerW-Fenster-Suche fehlgeschlagen. Prüfen ob Desktop-Icons sichtbar sind.

### Schwarzer Bildschirm
→ Shader-Kompilierung fehlgeschlagen. WGSL-Syntax prüfen.

## Performance

- **Rendering**: 60 FPS (V1) bzw. Sequenz-gesteuert (V2)
- **Gpu-Buffer-Update**: Pro Frame ~208 Bytes pro Monitor
- **Audio**: Asynchron via native OS-Audio-API
- **Memory**: Shared Arc<Vec<u8>> für Sound-Daten

## Lizenz

Proprietär. Alle Rechte vorbehalten.

## Autor

Entwickelt für Screen-Animation-Projekte mit WGPU und Windows-API.
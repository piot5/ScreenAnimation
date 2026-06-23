[Showcase] ScreenAnimation – GPU-beschleunigte Animationen & Wallpaper Engine für Windows

Hallo zusammen,

ich möchte mein neues Projekt vorstellen: **ScreenAnimation** – eine Open-Source Animation Engine für Windows, die WGPU und eigene WGSL-Shader nutzt.

## Was kann es?

1. **Overlay-Modus**: Transparente Animationen über dem Desktop (wie bei Rainmeter oder Wallpaper Engine)
2. **Wallpaper-Modus**: Animationen hinter den Desktop-Icons
3. **Multi-Monitor**: Läuft auf allen angeschlossenen Displays
4. **Audio-Support**: Eingebaute WAV-Wiedergabe synchron zu Animationen

## Tech Stack

- **Rust** + **WGPU 0.19** für hardwarebeschleunigtes Rendering
- **WGSL-Shader** für visuelle Effekte
- **windows-rs** für Win32-Integration
- **Rodio** für Audio

## Performance

- 60 FPS (vsync-locked)
- ~50 MB RAM + ~10 MB pro Monitor
- Nur 208 Bytes Uniform-Updates pro Frame
- Lock-Free Maus-Tracking

## Beispiel-Shader

```wgsl
@fragment
fn fs_default(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    let uv = coord.xy / vec2f(textureDimensions(tex0));
    let bg = textureSample(tex0, samp0, uv);
    
    var color = bg.rgb;
    color.r += sin(u.time * u.logic_params[0] + u.mouse.x * 3.14) * 0.1;
    
    return vec4f(color, bg.a);
}
```

## Nutzung

```bash
# Build
cargo build --release

# Animation als Overlay
animationengine.exe Animation assets/animation1.flow

# Als Wallpaper
animationengine.exe Wallpaper assets/wallpaper1.flow
```

## .flow-Pakete

einfache ZIP-Archive mit:
- `config.toml` – Parameter & Sequenzen
- `shader.wgsl` – WGSL-Code
- `*.wav` – Sounds
- `background.png` – Hintergrund (optional)

## Warum Rust + WGPU?

Ich wollte eine moderne Alternative zu Wallpaper Engine (C++, DX11) bauen. Rust gibt mir Safety + Performance, WGPU funktioniert auf Vulkan/DX12/Metal ohne Plattformcode.

Besonders stolz bin ich auf:
- Modulare Architektur (loader/engine/logic/windows klar getrennt)
- Zero-Copy Audio-Sharing via Arc<Vec<u8>>
- Einfaches Shader-Hotloading (Shader sind nur Strings)

## Herausforderungen

- **Windows-API**: WorkerW-Trick für Wallpaper-Modus war tricky
- **WGPU-Learning-Curve**: Aber die Dokumentation ist jetzt deutlich besser
- **Multi-Monitor**: Jeder Monitor kriegt sein eigenes Window + Swapchain

## Fragen an die Community

1. Hat jemand Erfahrung mit **Shader-Debugging** unter Windows?
2. **Sicherheit**: Sollte man externe .flow-Pakete signieren?
3. **Cross-Platform**: Linux-Port mit Wayland + wgpu machbar?
4. **Feature-Wunsch**: Video-Texturen (MP4) – nutzt jemand `mpv` oder `ffmpeg` als Library?

## Links

- **Source**: GitHub (im Profil verlinkt)
- **Doku**: README.md + docs/architecture.md
- **Beispiele**: assets/animation1.flow, assets/wallpaper1.flow

## Was kommt als Nächstes?

- [ ] Hot-Reload für Shader während Laufzeit
- [ ] Video-Textur-Support
- [ ] GUI-Editor für .flow-Pakete (Tauri?)
- [ ] Plugin-System für Effekte
- [ ] Linux/MacOS Port

Fragen, Feedback, Ideen sehr willkommen! 🦀

Besonders interessiert mich:
- Wie handhabt ihr Multi-Monitor-Setups bei solchen Engines?
- Gibt es Best Practices für Shader-Compilation-Fehler-Ratge?

Danke für's Lesen!
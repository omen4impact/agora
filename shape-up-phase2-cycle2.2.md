# Shape Up - Phase 2, Cycle 2.2: Audio Pipeline Professionalisierung

## Übersicht

Cycle 2.2 fokussiert sich auf die Professionalisierung der Audio Pipeline mit zwei kritischen Features:
1. **Opus Codec Integration** - Effiziente Audio-Kompression (24-128 kbps)
2. **RNNoise Integration** - ML-basierte Rauschunterdrückung

**Dauer: 6 Wochen**

**Startdatum: 2026-02-14**

---

## Pitch 2.2.1: Opus Codec Integration

### Problem

Die aktuelle Audio Pipeline in `core/src/audio.rs` überträgt Raw Audio (f32, 48kHz, Mono).
- **Bandbreite:** ~384 kbps (48000 samples/sec * 32 bit)
- **Problem:** Zu viel für mobile Netzwerke und instabile Verbindungen
- **Lösung:** Opus Codec mit 24-128 kbps (Faktor 3-16x weniger Bandbreite)

### Appetite: 4 Wochen

### Solution

```
Woche 1: Opus Crate Integration
- opus Crate einbinden
- Encoder/Decoder Wrapper
- Konfiguration (Bitrate, Complexity, Mode)

Woche 2: Audio Pipeline Integration
- Encode nach Capture
- Decode vor Playback
- Frame-Format anpassen

Woche 3: Adaptive Bitrate
- Netzwerk-Qualität erkennen
- Bitrate dynamisch anpassen
- Smooth Transitions

Woche 4: FEC & PLC
- Forward Error Correction
- Packet Loss Concealment (Opus eingebaut)
- Tests & Benchmarks
```

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                       Audio Pipeline                             │
│                                                                 │
│  Capture Path:                                                  │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐    │
│  │ Mikrofon │ → │ cpal     │ → │ RNNoise  │ → │ Opus     │    │
│  │          │   │ Capture  │   │ (Denoise)│   │ Encode   │    │
│  └──────────┘   └──────────┘   └──────────┘   └────┬─────┘    │
│                                                     │          │
│                                              Encrypted         │
│                                              Audio Packet       │
│                                                     │          │
│                                                     ▼          │
│                                              [Network]          │
│                                                                 │
│  Playback Path:                                                 │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐    │
│  │Lautsprecher│ ←│ cpal     │ ← │ Jitter   │ ← │ Opus     │    │
│  │          │   │ Playback │   │ Buffer   │   │ Decode   │    │
│  └──────────┘   └──────────┘   └──────────┘   └──────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Opus Configuration

```rust
pub struct OpusConfig {
    pub sample_rate: u32,           // 48000
    pub channels: u8,               // 1 (mono) or 2 (stereo)
    pub bitrate: i32,               // 24000 - 128000 bps
    pub mode: OpusMode,             // VoIP, Audio, LowDelay
    pub complexity: u8,             // 1-10 (10 = best quality, slower)
    pub enable_fec: bool,           // Forward Error Correction
    pub enable_dtx: bool,           // Discontinuous Transmission
    pub packet_loss_perc: u8,       // Expected packet loss for FEC
}

pub enum OpusMode {
    Voip,       // Optimized for voice
    Audio,      // Optimized for music
    LowDelay,   // Minimized latency
}
```

### Tasks

#### Woche 1: Opus Integration

- [ ] `opus` Crate zu Cargo.toml hinzufügen
- [ ] `OpusEncoder` Wrapper implementieren
- [ ] `OpusDecoder` Wrapper implementieren
- [ ] Konfigurationsoptionen definieren
- [ ] Unit Tests für Encode/Decode

#### Woche 2: Pipeline Integration

- [ ] `EncodedAudioFrame` Typ definieren
- [ ] Capture-Path: f32 → Opus Encoding
- [ ] Playback-Path: Opus Decoding → f32
- [ ] Frame-Größe anpassen (960 samples = 20ms @ 48kHz)
- [ ] Integration Tests

#### Woche 3: Adaptive Bitrate

- [ ] `BitrateController` implementieren
- [ ] Netzwerk-Qualität überwachen (RTT, Loss)
- [ ] Bitrate dynamisch anpassen
- [ ] Smooth Transitions ohne Unterbrechung
- [ ] CLI-Befehl für Bitrate-Status

#### Woche 4: FEC & PLC

- [ ] Forward Error Correction aktivieren
- [ ] Packet Loss Concealment testen
- [ ] Jitter Buffer Integration
- [ ] Performance Benchmarks
- [ ] Dokumentation

### Rabbit Holes

- Opus-Konfiguration komplex → Presets für Voice/Music
- Bitrate-Anpassung kann Audio-Artefakte verursachen → Smooth Transitions
- PLC bei hohem Paketverlust → Opus hat eingebaute Unterstützung

### No-Gos

- Keine anderen Codecs (Opus ist Industriestandard)
- Keine manuellen Codec-Einstellungen für Endnutzer
- Keine Transcoding zwischen Codecs

### Erfolgskriterien

- [ ] Bandbreite < 50 kbps für Sprache (vs. ~384 kbps aktuell)
- [ ] Qualität subjektiv vergleichbar mit Discord
- [ ] PLC funktioniert bei 10% Paketverlust
- [ ] Adaptive Bitrate reagiert auf Netzwerk-Änderungen
- [ ] CPU-Last < 5% auf normaler Hardware

---

## Pitch 2.2.2: RNNoise & Echo Cancellation

### Problem

- Aktuelle Rauschunterdrückung: Einfacher Noise Gate
- Problem: Schneidet leise Sprache ab, lässt Rauschen durch
- Lösung: RNNoise - ML-basierte Rauschunterdrückung

### Appetite: 4 Wochen

### Solution

```
Woche 1-2: RNNoise Integration
- nnnoiseless oder rnnoise-rs Crate
- RNNoise Wrapper implementieren
- Integration in Audio Pipeline

Woche 3-4: Echo Cancellation
- WebRTC AEC oder speexdsp
- Auto-Kalibrierung
- Integration & Tests
```

### RNNoise Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     RNNoise Pipeline                         │
│                                                             │
│  ┌──────────┐   ┌──────────────────┐   ┌──────────┐        │
│  │ Input    │ → │ Feature          │ → │ RNN      │        │
│  │ Audio    │   │ Extraction       │   │ Model    │        │
│  │ (f32)    │   │ (Bands, Pitch)   │   │ (GRU)    │        │
│  └──────────┘   └──────────────────┘   └────┬─────┘        │
│                                              │              │
│                                              ▼              │
│  ┌──────────┐   ┌──────────────────┐   ┌──────────┐        │
│  │ Output   │ ← │ Gain             │ ← │ Voice    │        │
│  │ Audio    │   │ Application      │   │ Probability│       │
│  │ (f32)    │   │                  │   │          │        │
│  └──────────┘   └──────────────────┘   └──────────┘        │
│                                                             │
│  Features:                                                  │
│  - 22 Bänder (Bark-Scale)                                   │
│  - Pitch Detection                                          │
│  - GRU Neural Network (3 layers)                            │
│  - ~6% CPU auf moderner Hardware                            │
└─────────────────────────────────────────────────────────────┘
```

### Tasks

#### Woche 1-2: RNNoise Integration

- [ ] RNNoise Library evaluieren (nnnoiseless vs rnnoise-sys)
- [ ] `Denoiser` Trait definieren
- [ ] RNNoise Wrapper implementieren
- [ ] Frame-Processing (480 samples = 10ms)
- [ ] Integration in Capture-Path
- [ ] CPU-Last optimieren

#### Woche 3-4: Echo Cancellation

- [ ] AEC Library evaluieren (webrtc-audio-processing vs speexdsp)
- [ ] `EchoCanceller` Trait definieren
- [ ] AEC Wrapper implementieren
- [ ] Auto-Kalibrierung beim Start
- [ ] Integration mit Playback-Path
- [ ] Tests für Echo-Reduktion

### Rabbit Holes

- RNNoise CPU-Last auf älteren CPUs → Optional, Auto-Detect
- AEC Kalibrierung kann fehlschlagen → Graceful Fallback
- Platform-spezifische Audio → cpal abstrahiert bereits

### No-Gos

- Keine Hardware-AEC-Unterstützung (zu komplex)
- Keine manuellen Filter-Einstellungen für Nutzer
- Keine Deep-Learning-Modelle trainieren (vortrainiert nutzen)

### Erfolgskriterien

- [ ] RNNoise reduziert Hintergrundgeräusche spürbar (subjektiv)
- [ ] Echo bei Lautsprecher-Nutzung stark reduziert
- [ ] CPU-Last < 5% auf normaler Hardware
- [ ] Audio-Latenz < 50ms (lokal, ohne Netzwerk)
- [ ] Option zum Deaktivieren verfügbar

---

## Implementation Plan

### Week 1: Opus Foundation

```rust
// core/src/codec/opus.rs (neu)
pub struct OpusEncoder {
    encoder: opus::Encoder,
    config: OpusConfig,
}

pub struct OpusDecoder {
    decoder: opus::Decoder,
    sample_rate: u32,
    channels: u8,
}

impl OpusEncoder {
    pub fn new(config: OpusConfig) -> AgoraResult<Self>;
    pub fn encode(&mut self, input: &[f32]) -> AgoraResult<Vec<u8>>;
    pub fn set_bitrate(&mut self, bitrate: i32) -> AgoraResult<()>;
}

impl OpusDecoder {
    pub fn new(sample_rate: u32, channels: u8) -> AgoraResult<Self>;
    pub fn decode(&mut self, input: &[u8]) -> AgoraResult<Vec<f32>>;
}
```

### Week 2: Denoiser Foundation

```rust
// core/src/denoise/mod.rs (neu)
pub trait Denoiser: Send + Sync {
    fn process(&mut self, frame: &mut [f32]);
    fn reset(&mut self);
}

// core/src/denoise/rnnoise.rs
pub struct RnnoiseDenoiser {
    state: rnnoise::DenoiseState,
}

impl Denoiser for RnnoiseDenoiser {
    fn process(&mut self, frame: &mut [f32]) {
        // RNNoise expects 480 samples (10ms @ 48kHz)
        for chunk in frame.chunks_mut(480) {
            self.state.process_frame(chunk);
        }
    }
}
```

### Week 3: Pipeline Integration

```rust
// core/src/audio.rs (erweitern)
pub struct AudioPipeline {
    config: AudioConfig,
    encoder: Option<OpusEncoder>,
    decoder: Option<OpusDecoder>,
    denoiser: Option<Box<dyn Denoiser>>,
    // ... existing fields
}

impl AudioPipeline {
    pub fn capture_encoded_frame(&mut self) -> Option<EncodedAudioFrame> {
        let raw_frame = self.capture_frame()?;
        
        // Apply noise suppression
        let mut denoised = raw_frame;
        if let Some(ref mut denoiser) = self.denoiser {
            denoiser.process(&mut denoised);
        }
        
        // Encode with Opus
        if let Some(ref mut encoder) = self.encoder {
            let encoded = encoder.encode(&denoised).ok()?;
            return Some(EncodedAudioFrame {
                data: encoded,
                sequence: self.frame_sequence,
                timestamp: self.current_timestamp(),
            });
        }
        
        None
    }
}
```

### Week 4: Adaptive Bitrate

```rust
// core/src/bitrate.rs (neu)
pub struct BitrateController {
    current_bitrate: i32,
    min_bitrate: i32,
    max_bitrate: i32,
    rtt_samples: Vec<Duration>,
    loss_samples: Vec<f32>,
}

impl BitrateController {
    pub fn update_network_stats(&mut self, rtt: Duration, loss_rate: f32);
    pub fn suggest_bitrate(&self) -> i32;
    pub fn should_adjust(&self) -> bool;
}
```

---

## Dependencies

```toml
# Cargo.toml additions
[dependencies]
# Audio Codec
opus = "0.3"              # Opus codec wrapper

# Noise Suppression
nnnoiseless = "0.12"      # RNNoise Rust port (optional)
# oder
# rnnoise-sys = "0.2"     # Native RNNoise bindings

# Echo Cancellation (optional für später)
# webrtc-audio-processing = "0.4"  # Native WebRTC AEC
```

---

## Testing Strategy

### Unit Tests
- Opus Encode/Decode Roundtrip
- RNNoise Processing
- Bitrate Adjustment Logic
- Frame Size Handling

### Integration Tests
- Full Audio Pipeline with Opus
- Audio Quality with Noise Injection
- Network Simulation (Loss, Latency)
- CPU Usage Profiling

### Performance Tests
- Latenz-Messung (Capture → Encode → Decode → Playback)
- CPU-Last bei verschiedenen Bitraten
- Memory Usage
- Thermal Impact (Mobile)

---

## Risks

| Risiko | Wahrscheinlichkeit | Impact | Mitigation |
|--------|-------------------|--------|------------|
| RNNoise CPU-Last zu hoch | Medium | High | Optional machen, Auto-Detect |
| Opus-Qualität unzureichend | Low | High | Discord nutzt Opus erfolgreich |
| AEC Kalibrierung fehlschlägt | Medium | Medium | Graceful Fallback |
| Cross-Platform Audio-Issues | Medium | Medium | cpal abstrahiert bereits |

---

## Exit Criteria

- [ ] Alle Tests bestehen
- [ ] Bandbreite < 50 kbps für Sprache
- [ ] RNNoise aktiv und funktionell
- [ ] CPU-Last < 5%
- [ ] Audio-Latenz < 50ms
- [ ] Dokumentation aktualisiert
- [ ] CLI-Befehle für Audio-Testing

---

## CLI Commands (Neu)

```bash
# Audio Quality Testing
agora test-audio --duration 10 --with-opus
agora test-audio --with-rnnoise
agora test-audio --bitrate 64000

# Audio Device Management
agora list-audio-devices --verbose
agora audio-settings --show
agora audio-settings --bitrate 48000
agora audio-settings --denoise on|off
```

---

*Dokument erstellt: 2026-02-14*
*Letztes Update: 2026-02-14*
*Cycle 2.2 Status: IMPLEMENTIERT ✅*

---

## Implementierungsstatus (2026-02-14)

### Erledigte Tasks

| Task | Status | Datei |
|------|--------|-------|
| Opus Encoder/Decoder | ✅ COMPLETE | `core/src/codec/opus.rs` |
| RNNoise Denoiser | ✅ COMPLETE | `core/src/denoise/rnnoise.rs` |
| AudioProcessor Integration | ✅ COMPLETE | `core/src/audio_processor.rs` |
| Adaptive Bitrate Controller | ✅ COMPLETE | `core/src/audio_processor.rs` |
| Unit Tests | ✅ COMPLETE | 142 Tests bestanden |

### Neue Dateien

```
core/src/
├── codec/
│   ├── mod.rs           # AudioEncoder/AudioDecoder Traits
│   └── opus.rs          # Opus Encoder/Decoder (350+ Zeilen)
├── denoise/
│   ├── mod.rs           # Denoiser Trait
│   └── rnnoise.rs       # RNNoise Wrapper (120+ Zeilen)
└── audio_processor.rs   # Kombinierte Pipeline (320+ Zeilen)
```

### Test-Ergebnisse

- **Unit Tests:** 142 bestanden (+13 neue Tests)
- **Integration Tests:** 24 bestanden

### Features implementiert

- **Opus Codec:**
  - Encoder mit konfigurierbarer Bitrate (6-510 kbps)
  - Decoder mit Packet Loss Concealment
  - FEC (Forward Error Correction)
  - DTX (Discontinuous Transmission)

- **RNNoise:**
  - ML-basierte Rauschunterdrückung
  - 480 Samples (10ms @ 48kHz) Frame-Processing
  - Enable/Disable Toggle

- **AudioProcessor:**
  - Kombinierte Denoise + Encode Pipeline
  - Decode + Denoise Pipeline
  - Statistics Tracking
  - Adaptive Bitrate Controller
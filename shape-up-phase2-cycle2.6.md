# Shape Up - Phase 2, Cycle 2.6: Mobile Foundation

## Übersicht

Cycle 2.6 fokussiert sich auf die Mobile-App Foundation mit Flutter. Mobile ist der primäre Nutzungsort für Voice-Chat.

**Dauer: 6 Wochen**

**Startdatum: 2026-02-14**

---

## Pitch 2.6.1: Flutter Project Setup

### Problem

Mobile ist der primäre Nutzungsort für Voice-Chat. Desktop-only limitiert die Zielgruppe massiv.

### Appetite: 2 Wochen

### Solution

```
Woche 1: Flutter Projekt Setup (bereits vorhanden)
Woche 2: FFI Bindings vervollständigen
```

### Architecture

```
┌─────────────────────────────────────────────────┐
│                 Flutter UI                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │  Screens │ │ Widgets  │ │  State   │       │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘       │
│       └────────────┼────────────┘              │
│                    │                           │
│              ┌─────┴─────┐                     │
│              │  FFI      │                     │
│              │  Bridge   │                     │
│              └─────┬─────┘                     │
│                    │                           │
│  ┌─────────────────┼─────────────────┐        │
│  │            Core Rust               │        │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ │        │
│  │  │Network │ │ Audio  │ │Crypto  │ │        │
│  │  └────────┘ └────────┘ └────────┘ │        │
│  └────────────────────────────────────┘        │
└─────────────────────────────────────────────────┘
```

### Project Structure

```
mobile/
├── lib/
│   ├── main.dart
│   ├── screens/
│   │   ├── home_screen.dart
│   │   ├── create_room_screen.dart
│   │   ├── join_room_screen.dart
│   │   └── session_screen.dart
│   ├── services/
│   │   ├── agora_ffi.dart
│   │   ├── identity_service.dart
│   │   └── audio_service.dart
│   └── widgets/
│       └── status_indicator.dart
├── android/
├── ios/
├── macos/
├── linux/
├── web/
└── pubspec.yaml
```

### Tasks

- [x] Flutter Projekt erstellt
- [x] FFI Bindings für Core Functions
- [x] Home Screen UI
- [x] Identity Service
- [x] FFI Struct Handling (NATInfo, MixerInfo)
- [x] Session Screen vervollständigen
- [x] iOS Background Audio
- [x] Android Foreground Service

### Rabbit Holes

- FFI auf iOS erfordert spezielle Build-Konfiguration
- Background Audio auf iOS ist komplex
- Permission Handling auf Android

### No-Gos

- Keine Web-Version (separater Cycle)
- Keine Auto-Updates
- Kein Push Notifications

### Erfolgskriterien

- [x] App startet auf iOS und Android
- [x] FFI Bindings funktionieren
- [x] Basic UI funktioniert

---

## Pitch 2.6.2: Audio Integration

### Problem

Audio ist der Kern der App. Mobile Audio ist komplexer als Desktop.

### Appetite: 2 Wochen

### Solution

```
Woche 1: Audio Recording/Playback
Woche 2: Background Audio, Permissions
```

### Audio Architecture

```
┌─────────────────────────────────────────────┐
│           Flutter Audio Layer               │
├─────────────────────────────────────────────┤
│                                             │
│  ┌───────────────┐    ┌───────────────┐    │
│  │  record       │    │  audioplayers  │    │
│  │  (Recording)  │    │  (Playback)   │    │
│  └───────┬───────┘    └───────┬───────┘    │
│          │                     │            │
│          └──────────┬──────────┘            │
│                     │                       │
│              ┌──────┴──────┐               │
│              │ AudioService│               │
│              └──────┬──────┘               │
│                     │                       │
│  ┌──────────────────┼──────────────────┐  │
│  │         Rust Core FFI               │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐  │  │
│  │  │ Opus   │ │RNNoise │ │Mixer   │  │  │
│  │  └────────┘ └────────┘ └────────┘  │  │
│  └─────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

### Permissions

**Android (AndroidManifest.xml):**
```xml
<uses-permission android:name="android.permission.RECORD_AUDIO" />
<uses-permission android:name="android.permission.INTERNET" />
<uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.WAKE_LOCK" />
```

**iOS (Info.plist):**
```xml
<key>NSMicrophoneUsageDescription</key>
<string>Agora needs microphone access for voice chat</string>
<key>UIBackgroundModes</key>
<array>
    <string>audio</string>
</array>
```

### Tasks

- [x] Audio Permissions
- [x] Audio Recording Service
- [x] Audio Playback Service
- [x] iOS Background Audio
- [x] Android Foreground Service

### Erfolgskriterien

- [x] Audio Recording funktioniert
- [x] Audio Playback funktioniert
- [x] Hintergrund-Audio auf iOS

---

## Pitch 2.6.3: Platform Integration

### Problem

Jede Platform hat spezifische Anforderungen für Background-Audio und Permissions.

### Appetite: 2 Wochen

### Solution

```
Woche 1: iOS Background Mode
Woche 2: Android Foreground Service
```

### iOS Background Audio

```swift
// AppDelegate.swift
import Flutter
import AVFoundation

@UIApplicationMain
@objc class AppDelegate: FlutterAppDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    
    // Configure audio session for background
    do {
      try AVAudioSession.sharedInstance().setCategory(
        .playAndRecord,
        mode: .voiceChat,
        options: [.duckOthers, .allowBluetooth]
      )
      try AVAudioSession.sharedInstance().setActive(true)
    } catch {
      print("Audio session error: \(error)")
    }
    
    GeneratedPluginRegistrant.register(with: self)
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }
}
```

### Android Foreground Service

```kotlin
// MainActivity.kt
class MainActivity : FlutterActivity() {
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        
        // Request permissions
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            requestPermissions(
                arrayOf(
                    Manifest.permission.RECORD_AUDIO,
                    Manifest.permission.INTERNET
                ),
                0
            )
        }
    }
}
```

### Tasks

- [x] iOS Audio Session Configuration
- [x] iOS Background Mode
- [x] Android Foreground Service
- [x] Android Wake Lock
- [x] Permission Handler Integration

### Erfolgskriterien

- [x] App läuft im Hintergrund weiter
- [x] Audio funktioniert im Hintergrund
- [x] Permissions werden korrekt angefragt

---

## Dependencies

```yaml
# pubspec.yaml (bereits vorhanden)
dependencies:
  flutter:
    sdk: flutter
  ffi: ^2.1.0
  provider: ^6.1.1
  shared_preferences: ^2.2.2
  url_launcher: ^6.2.2
  permission_handler: ^11.1.0
  flutter_foreground_task: ^6.1.0
  record: ^5.1.0
  audioplayers: ^6.0.0
```

---

## Testing Strategy

### Manual Testing
- [x] App auf iOS Simulator starten
- [x] App auf Android Emulator starten
- [x] Audio Recording testen
- [x] Background Audio testen

### Build Testing
- [x] iOS Build (Debug/Release)
- [x] Android Build (Debug/Release)
- [x] macOS Build

---

## Exit Criteria

- [x] App startet auf iOS
- [x] App startet auf Android
- [x] FFI Bindings funktionieren
- [x] Audio funktioniert
- [x] Background Audio funktioniert (iOS)
- [x] Alle Tests bestehen

---

*Dokument erstellt: 2026-02-14*
*Letztes Update: 2026-02-14*
*Cycle 2.6 Status: COMPLETE*
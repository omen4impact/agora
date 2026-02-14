# Shape Up - Cycle 2.7: Web-Version (WebRTC)

## Übersicht

**Problem:** Nutzer ohne Installation möchten an Voice-Chats teilnehmen können. Eine Web-Version senkt die Einstiegshürde massiv und ermöglicht maximale Zugänglichkeit.

**Appetite:** 6 Wochen

---

## Pitch 2.7.1: Flutter Web Build

### Problem
Flutter bereits vorhanden, aber Web-Target noch nicht konfiguriert. Web-Version muss dieselben Features bieten wie Mobile/Desktop.

### Solution
```
Woche 1-2: Flutter Web Setup, CORS, Asset-Loading
Woche 3-4: Web-optimierte UI (Responsive, Touch/Mouse)
Woche 5-6: Progressive Web App (PWA), Offline-Support
```

### Architecture
```
┌─────────────────────────────────────────────────────┐
│                   Flutter Web App                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │  Screens │ │ Widgets  │ │  State   │            │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘            │
│       └────────────┼────────────┘                   │
│                    │                                │
│              ┌─────┴─────┐                          │
│              │  WebRTC   │                          │
│              │  Audio    │                          │
│              └─────┬─────┘                          │
│                    │                                │
│  ┌─────────────────┼─────────────────┐             │
│  │            WebSocket              │             │
│  │         (Signaling)               │             │
│  └─────────────────┬─────────────────┘             │
│                    │                                │
│              ┌─────┴─────┐                          │
│              │   Agora   │                          │
│              │   Node    │                          │
│              └───────────┘                          │
└─────────────────────────────────────────────────────┘
```

### Erfolgskriterien
- [ ] Flutter Web Build funktioniert
- [ ] Responsives UI für Desktop/Mobile Browser
- [ ] PWA installierbar
- [ ] Basis-Offline-Support

---

## Pitch 2.7.2: WebRTC Audio Integration

### Problem
Browser nutzen WebRTC für Audio, nicht native Audio APIs. AudioPipeline muss an WebRTC angepasst werden.

### Solution
```
Woche 1-2: WebRTC Audio API Integration
Woche 3-4: MediaStream Management, Echo Cancellation
Woche 5-6: Browser-Compatibility (Chrome, Firefox, Safari)
```

### WebRTC Audio Pipeline
```
┌─────────────────────────────────────────────────────┐
│                 Browser WebRTC                       │
│                                                      │
│  ┌─────────────┐    ┌─────────────┐                │
│  │ getUserMedia│ -> │ MediaStream │                │
│  │  (Audio)    │    │   Track     │                │
│  └─────────────┘    └──────┬──────┘                │
│                            │                        │
│                     ┌──────┴──────┐                 │
│                     │ AudioContext │                │
│                     │ (Processing) │                │
│                     └──────┬──────┘                 │
│                            │                        │
│              ┌─────────────┼─────────────┐          │
│              │             │             │          │
│        [Echo Cancel] [Noise Suppression] [AGC]     │
│              │             │             │          │
│              └─────────────┼─────────────┘          │
│                            │                        │
│                     ┌──────┴──────┐                 │
│                     │ RTCPeerConn │                 │
│                     │   (P2P)     │                 │
│                     └─────────────┘                 │
└─────────────────────────────────────────────────────┘
```

### Browser Audio Constraints
```dart
final constraints = {
  'audio': {
    'echoCancellation': true,
    'noiseSuppression': true,
    'autoGainControl': true,
    'sampleRate': 48000,
    'channelCount': 1,
  },
  'video': false,
};
```

### Erfolgskriterien
- [x] getUserMedia funktioniert in Chrome/Firefox/Safari
- [x] Audio-Aufnahme und Wiedergabe (`mobile/lib/services/webrtc_service.dart`)
- [x] Echo Cancellation aktiv
- [x] Noise Suppression funktioniert

---

## Pitch 2.7.3: WebSocket Signaling

### Problem
WebRTC benötigt einen Signaling-Kanal für Verbindungsaufbau. P2P-Nodes müssen über WebSocket erreichbar sein.

### Solution
```
Woche 1-2: WebSocket Server im Node
Woche 3-4: Signaling Protocol (SDP/ICE Exchange)
Woche 5-6: Fallback für eingeschränkte Netzwerke
```

### Signaling Flow
```
┌─────────┐          ┌─────────┐          ┌─────────┐
│  Web    │          │  Node   │          │  Web    │
│  Client │          │(Signaling)│        │  Client │
└────┬────┘          └────┬────┘          └────┬────┘
     │                    │                    │
     │  1. Connect WS     │                    │
     │ ──────────────────>│                    │
     │                    │                    │
     │  2. Join Room      │                    │
     │ ──────────────────>│  3. Notify Peer    │
     │                    │ ──────────────────>│
     │                    │                    │
     │  4. SDP Offer      │                    │
     │<───────────────────│<───────────────────│
     │                    │                    │
     │  5. SDP Answer     │                    │
     │ ──────────────────>│ ──────────────────>│
     │                    │                    │
     │  6. ICE Candidates │                    │
     │<───────────────────│<───────────────────│
     │                    │                    │
     │  7. Direct P2P     │                    │
     │<───────────────────────────────────────>│
     │                    │                    │
```

### Signaling Message Types
```rust
enum SignalingMessage {
    Join { room_id: String, peer_id: String },
    Leave { room_id: String, peer_id: String },
    SdpOffer { from: String, sdp: String },
    SdpAnswer { from: String, sdp: String },
    IceCandidate { from: String, candidate: String },
    PeerList { peers: Vec<PeerInfo> },
}
```

### Erfolgskriterien
- [x] WebSocket Server läuft auf Node (`node/src/signaling.rs`)
- [x] SDP/ICE Exchange funktioniert
- [x] P2P-Verbindung zwischen Web-Clients (via WebRTC)
- [x] Fallback für eingeschränkte Netzwerke (TURN support)

---

## Pitch 2.7.4: Browser-Compatibility

### Problem
Unterschiedliche Browser haben unterschiedliche WebRTC-Implementierungen. Safari ist besonders restriktiv.

### Solution
```
Woche 1-2: Chrome/Chromium Support (80% Market)
Woche 3-4: Firefox Support
Woche 5-6: Safari Workarounds, Mobile Browser
```

### Browser-Specific Issues

| Browser | Issue | Solution |
|---------|-------|----------|
| Chrome | - | Standard WebRTC |
| Firefox | Different codec priorities | Opus preference |
| Safari | No VP8, strict autoplay | H.264 fallback, user gesture |
| Mobile iOS | Background audio | PWA + Audio Session |

### Feature Detection
```dart
bool isWebRTCSupported() {
  return js.context['RTCPeerConnection'] != null &&
         js.context['navigator']['mediaDevices'] != null &&
         js.context['navigator']['mediaDevices']['getUserMedia'] != null;
}
```

### Erfolgskriterien
- [x] Chrome/Chromium: 100% Features
- [x] Firefox: 100% Features
- [x] Safari: Core Features (Audio works)
- [x] Mobile Browser: Audio works

---

## Rabbit Holes

- WebRTC Codec-Negotiation zu komplex → Opus-only forciert
- Safari Auto-Policy → User Gesture requirement beachten
- iOS Background Audio → PWA limitations dokumentieren

## No-Gos

- Keine Browser-Extensions
- Keine App-Store-Abhängigkeit
- Keine Server-seitige Audio-Verarbeitung

---

## Testing Strategy

### Unit Tests
- Signaling Message Serialization
- SDP Parsing
- ICE Candidate Handling

### Integration Tests
- WebSocket Connection Flow
- WebRTC Connection Establishment
- Audio Stream End-to-End

### Browser Tests
- Chrome (Desktop + Android)
- Firefox (Desktop + Android)
- Safari (Desktop + iOS)

---

## Exit Criteria

- [x] Flutter Web Build läuft
- [x] Audio funktioniert in Chrome/Firefox
- [x] P2P-Verbindung zwischen Web-Clients
- [x] PWA installierbar
- [x] Dokumentation für Browser-Support

---

## Zeitplan

```
Woche 1-2: Flutter Web Setup + UI
Woche 3-4: WebRTC Audio + WebSocket Signaling
Woche 5-6: Browser-Compatibility + PWA
```

---

*Dokument erstellt: 2026-02-14*
*Cycle 2.7 Status: 100% COMPLETE*

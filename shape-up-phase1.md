# Shape Up Setup - Phase 1: Dezentrales Fundament

## Übersicht: Shape Up für Meshvoice

Shape Up (von Basecamp) eignet sich ideal für Phase 1, weil:
- **6-Wochen-Cycles** → passen zu den beta-Releases alle 6 Wochen
- **Appetite-Driven** → "Wie viel Zeit wollen wir investieren?" statt ungenauer Schätzungen
- **Fixed Time, Variable Scope** → perfekt für experimentelle NAT-Traversal-Entwicklung
- **Pitches vor Coding** → zwingt zu klaren Problembeschreibungen

### Zeitrahmen Phase 1
**Gesamtdauer: 9-12 Monate = 6-7 Cycles à 6 Wochen + Cool-down**

```
Cycle 1-2:   Core Infrastructure (Networking + Identity)
Cycle 3-4:   Audio Pipeline + Mixer Logic
Cycle 5-6:   UI/UX Desktop + Mobile Foundation
Cycle 7:     Polish, Testing, Beta Release Preparation
```

---

## Shaping-Prozess

### Wer shapet?
**Shaper-Team (2-3 Personen):**
- 1 Technical Lead (Architektur-Verständnis)
- 1 Product Lead (Nutzer-Perspektive)
- Optional: 1 External Advisor (libp2p/P2P-Erfahrung)

### Wann findet Shaping statt?
- **Kontinuierlich**: Shaper arbeiten parallel zum laufenden Cycle am nächsten
- **Cool-down-Woche**: Finales Pitching für nächsten Cycle
- **Betting Table**: Ende jedes Cool-downs

### Shaping-Kriterien
Jeder Pitch muss beantworten:
1. **Problem**: Was genau ist das Problem? (Nicht die Lösung!)
2. **Appetite**: Wie viel Zeit sind wir bereit zu investieren?
3. **Solution**: Fat-Marker-Skizze der Lösung (nicht zu detailliert)
4. **Rabbit Holes**: Wo können wir uns verrennen?
5. **No-Gos**: Was bauen wir bewusst NICHT?

---

## Cycle 1: Netzwerk-Fundament & Identität

### Pitch 1.1: libp2p Core Integration

**Problem:**
Ohne funktionierende P2P-Verbindungen existiert keine Anwendung. Nutzer müssen sich gegenseitig finden und identifizieren können, ohne zentrale Server.

**Appetite: 6 Wochen** (1 ganzer Cycle)

**Solution:**
```
Woche 1-2: libp2p in Rust einbinden, Schlüsselgenerierung
Woche 3-4: Kademlia DHT für Peer Discovery
Woche 5-6: Basis-Room-System (erstellen/beitreten via Hash)
```

**Breadboarding:**
```
[Nutzer A] --erstellt Raum--> [DHT: room_hash → peer_ids]
                                    |
[Nutzer B] --sucht room_hash--> [DHT lookup]
                                    |
                              [Peer A kontaktieren]
                                    |
                              [Direkte Verbindung]
```

**Rabbit Holes:**
- libp2p-Doku ist teilweise unvollständig → early prototyping
- Kademlia-Performance bei vielen Peers → für Phase 1 irrelevant
- Verschiedene Transport-Protocols (TCP/QUIC/WebSockets) → erst TCP, später erweitern

**No-Gos:**
- Keine Mobile-Integration in diesem Cycle
- Keine NAT-Traversal-Optimierung (kommt in Cycle 2)
- Keine Verschlüsselung (kommt in Cycle 2)

**Erfolgskriterien:**
- [ ] Zwei Clients finden sich gegenseitig via DHT
- [ ] Raum-Erstellung generiert teilbaren Hash
- [ ] Basic CLI zum Testen vorhanden

---

### Pitch 1.2: Identitäts-System

**Problem:**
Nutzer müssen sich über Sessions hinweg wiedererkennen, ohne Login. Kryptografische Identität muss persistent und nutzerfreundlich sein.

**Appetite: 3 Wochen** (im selben Cycle parallel zu 1.1)

**Solution:**
- Ed25519 Schlüsselpaar beim ersten Start generieren
- Schlüssel sicher im OS-Keychain speichern
- Peer ID als öffentliche Identität
- Optional: Display Name zuordnen

**Rabbit Holes:**
- Keychain-Integration variiert stark zwischen OS → Tauri-Plugin nutzen
- Schlüssel-Migration bei reinstall → dokumentieren, nicht automatisieren

**No-Gos:**
- Keine Identitäts-Verifikation (kommt später)
- Keine Multi-Device-Support (kommt in Phase 2)

**Erfolgskriterien:**
- [ ] Schlüssel persistent nach App-Restart
- [ ] Peer ID wird angezeigt
- [ ] Display Name kann gesetzt werden

---

## Cycle 2: NAT-Traversal & Verschlüsselung

### Pitch 2.1: Hole-Punching-Implementation

**Problem:**
85%+ direkte Verbindungen sind kritisch für dezentrale Architektur. Ohne funktionierendes Hole-Punching sind wir von TURN-Servern abhängig.

**Appetite: 6 Wochen**

**Solution:**
```
Woche 1-2: ICE-Framework implementieren, STUN-Server integrieren
Woche 3-4: TCP + UDP Hole-Punching parallel
Woche 5-6: UPnP/NAT-PMP Auto-Config, IPv6-Support
```

**Fat-Marker Sketch:**
```
┌─────────────┐                    ┌─────────────┐
│   Client A  │                    │   Client B  │
│  (NAT/FW)   │                    │  (NAT/FW)   │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │  1. STUN: "Meine öffentliche IP" │
       │──────────────> STUN <────────────│
       │                                  │
       │  2. Exchange via DHT              │
       │<════════════════════════════════>│
       │                                  │
       │  3. Simultaneous Connect          │
       │════════════════════════════════>│
       │<════════════════════════════════│
       │                                  │
       │  ✅ Direkte Verbindung           │
```

**Rabbit Holes:**
- Symmetric NAT ist oft unüberwindbar → dokumentieren, nicht ewig kämpfen
- Carrier-Grade NAT (CGNAT) → TURN als Fallback akzeptieren
- Hole-Punching-Timing ist kritisch → iterative Tests

**No-Gos:**
- Kein TURN-Server-Betrieb in diesem Cycle
- Keine Tor/I2P-Integration (Phase 2)

**Erfolgskriterien:**
- [ ] Hole-Punching-Erfolgsrate > 80% in Test-Umgebungen
- [ ] Automatischer Fallback zu TURN bei Symmetric NAT
- [ ] Verbindungsaufbau < 5 Sekunden

---

### Pitch 2.2: End-to-End-Verschlüsselung

**Problem:**
Ohne E2E-Verschlüsselung ist P2P-Voice-Chat nicht vertrauenswürdig. Mixer dürfen keinen Zugriff auf Audio-Inhalt haben.

**Appetite: 4 Wochen**

**Solution:**
- Noise Protocol Framework integrieren
- Ephemere Session-Keys pro Raum
- Forward Secrecy implementieren

**Rabbit Holes:**
- Key-Exchange bei vielen Teilnehmern skalieren → X-Kombinationen vermeiden
- Perfect Forward Secrecy aufwendig → erstmal Forward Secrecy

**No-Gos:**
- Keine Post-Quantum-Kryptografie (zu experimentell)
- Keine Fingerprint-Verifikation (Cycle 3)

**Erfolgskriterien:**
- [ ] Alle Audio-Pakete verschlüsselt
- [ ] Mixer können Pakete nicht entschlüsseln
- [ ] Performance-Impact < 5ms Latenz

---

## Cycle 3: Audio-Pipeline

### Pitch 3.1: Audio-Capture und Playback

**Problem:**
Ohne hochwertige Audio-Verarbeitung ist Voice-Chat unbrauchbar. Niedrige Latenz und gute Qualität sind Essentials.

**Appetite: 6 Wochen**

**Solution:**
```
Woche 1-2: cpal (Rust audio lib) integrieren, Device-Enumeration
Woche 3-4: Opus-Codec einbinden, adaptive Bitrate
Woche 5-6: Basis-RNNoise für Noise-Cancellation
```

**Audio-Pipeline Sketch:**
```
[Mikrofon] → [cpal capture] → [RNNoise] → [Opus encode]
                                              │
                                          [Netzwerk]
                                              │
                                          [Opus decode] → [cpal playback] → [Lautsprecher]
```

**Rabbit Holes:**
- Audio-Latenz variiert stark je Hardware → adaptive Buffer
- RNNoise CPU-Last auf älteren Geräten → optional machen
- Platform-spezifische Audio-APIs → cpal abstrahiert größtenteils

**No-Gos:**
- Keine Echo-Cancellation in diesem Cycle (kommt in 3.2)
- Keine erweiterten Audio-Features (Equalizer, etc.)

**Erfolgskriterien:**
- [ ] Audio-Latenz < 50ms (ohne Netzwerk)
- [ ] Opus-Qualität vergleichbar mit Discord
- [ ] RNNoise reduziert Hintergrundgeräusche spürbar

---

### Pitch 3.2: Echo-Cancellation & Aggregierte Pipeline

**Problem:**
Echo ist einer der häufigsten Gründe für schlechte Voice-Chat-Erfahrung. Ohne Echo-Cancellation ist die Anwendung unbenutzbar ohne Headset.

**Appetite: 4 Wochen**

**Solution:**
- WebRTC AEC (Acoustic Echo Cancellation) Algorithmus portieren
- Oder: existierende AEC-Library integrieren
- Integrierte Audio-Pipeline testen

**Rabbit Holes:**
- AEC funktioniert schlecht ohne Kalibrierung → Auto-Kalibrierung implementieren
- Unterschiedliche Audio-Setups (Headset vs. Lautsprecher) → Standard-Konfiguration optimieren

**No-Gos:**
- Keine professionellen Audio-Features
- Keine Hardware-AEC-Unterstützung

**Erfolgskriterien:**
- [ ] Echo auch ohne Headset stark reduziert
- [ ] Vollständige Audio-Pipeline funktioniert
- [ ] CPU-Last < 5% auf normaler Hardware

---

## Cycle 4: Mixer-Logik & Skalierung

### Pitch 4.1: Full-Mesh für kleine Gruppen

**Problem:**
Bei ≤5 Teilnehmern ist Full-Mesh optimal (niedrigste Latenz). Muss automatisch und transparent funktionieren.

**Appetite: 3 Wochen**

**Solution:**
- Jeder Client verbindet zu allen anderen
- Audio von allen empfangen und mischen
- Einfache UI zeigt Verbindungstopologie

**Rabbit Holes:**
- Bandbreiten-Überlastung bei vielen Teilnehmern → klar auf ≤5 limitieren
- Audio-Mixing auf CPU → SIMD-Optimierung falls nötig

**No-Gos:**
- Kein SFU in diesem Pitch (kommt in 4.2)

**Erfolgskriterien:**
- [ ] Full-Mesh funktioniert mit 5 Teilnehmern
- [ ] Latenz < 100ms End-to-End
- [ ] UI zeigt alle Verbindungen

---

### Pitch 4.2: SFU-Modus & Mixer-Algorithmus

**Problem:**
Bei >5 Teilnehmern überfordert Full-Mesh die Upload-Bandbreite. Automatische Umschaltung auf SFU mit intelligenter Mixer-Auswahl.

**Appetite: 5 Wochen**

**Solution:**
```
Woche 1-2: SFU-Logik implementieren (ein Client mixt für alle)
Woche 3-4: Mixer-Selection-Algorithmus (Score-basiert)
Woche 5: Rotation und Fallback bei Mixer-Ausfall
```

**Mixer-Selection Sketch:**
```
Jeder Client berechnet lokal:
┌─────────────────────────────────────────┐
│ Bandwidth Score    (40% weight)         │
│ Stability Score    (25% weight)         │
│ Resource Score     (20% weight)         │
│ Duration Score     (15% weight)         │
└─────────────────────────────────────────┘
              │
              ▼
    [Highest Score = New Mixer]
              │
              ▼
    [Broadcast decision via DHT]
```

**Rabbit Holes:**
- Score-Manipulation durch Clients → nicht in Phase 1 relevant
- Mixer-Wechsel verursacht Audio-Unterbrechung → während Sprechpause wechseln
- Gleichstand bei Scores → deterministischer Hash-Entscheid

**No-Gos:**
- Keine Multi-Mixer für sehr große Gruppen (Phase 2)
- Keine dedizierten Server-Nodes (Phase 2)

**Erfolgskriterien:**
- [ ] Automatische Umschaltung bei >5 Teilnehmern
- [ ] Mixer-Wechsel < 1 Sekunde Unterbrechung
- [ ] Algorithmus funktioniert dezentral ohne Koordination

---

## Cycle 5: Desktop UI

### Pitch 5.1: Core UI - Raum erstellen & Beitreten

**Problem:**
Technisch funktionierende Anwendung braucht nutzerfreundliche Oberfläche. Erste Interaktion: Raum erstellen oder beitreten.

**Appetite: 4 Wochen**

**Solution:**
- Tauri + Svelte (oder React) für UI
- Drei-Optionen-Startbildschirm: Erstellen, Beitreten, Link
- Clipboard-Integration für Raumeinladungen
- Passwort-Schutz optional

**UI Sketch:**
```
┌────────────────────────────────────┐
│         🔊 MESHVOICE               │
│                                    │
│  ┌──────────────────────────────┐  │
│  │      🎙️ Neuen Raum erstellen   │  │
│  └──────────────────────────────┘  │
│                                    │
│  ┌──────────────────────────────┐  │
│  │      🔗 Raum beitreten        │  │
│  │      [____________Hash___]   │  │
│  └──────────────────────────────┘  │
│                                    │
│  ┌──────────────────────────────┐  │
│  │      📋 Link öffnen          │  │
│  └──────────────────────────────┘  │
│                                    │
└────────────────────────────────────┘
```

**Rabbit Holes:**
- UI-Framework-Wahl → Svelte für geringe Bundle-Size
- Design-System → erstmal minimal, später ausbauen

**No-Gos:**
- Keine Einstellungen in diesem Pitch
- Keine Session-UI (kommt in 5.2)

**Erfolgskriterien:**
- [ ] Raum erstellen generiert Hash + Link
- [ ] Link kopiert in Clipboard
- [ ] Raumeintritt via Hash funktioniert

---

### Pitch 5.2: Session UI & Teilnehmer-Übersicht

**Problem:**
Während eines Calls müssen Nutzer sehen, wer spricht, wie die Verbindung ist, und individuelle Einstellungen vornehmen.

**Appetite: 4 Wochen**

**Solution:**
- Teilnehmer-Liste mit Avatar/Name
- Sprech-Indikator (grüner Ring)
- Verbindungsqualität (grün/gelb/rot)
- Lautstärke-Regler pro Teilnehmer
- Push-to-Talk Toggle

**Session UI Sketch:**
```
┌────────────────────────────────────┐
│ Raum: gaming-night-abc123    [🔧] │
├────────────────────────────────────┤
│                                    │
│  🟢 Alice (Mixer)         🔊───╮  │
│      ████████░░░░ (Lat: 45ms)   │  │
│                         [🔊]────╯  │
│                                    │
│  🟡 Bob                   🔊───╮  │
│      ████░░░░░░░░ (Lat: 120ms) │  │
│                         [🔊]────╯  │
│                                    │
│  🟢 Charlie              🔊───╮  │
│      ██████████░░ (Lat: 32ms)  │  │
│                         [🔊]────╯  │
│                                    │
├────────────────────────────────────┤
│  [🎤 Voice] [🔇 Mute] [📞 Leave]  │
└────────────────────────────────────┘
```

**Rabbit Holes:**
- Real-time-Updates → effiziente Event-Struktur
- Viele Teilnehmer → scrollbare Liste

**No-Gos:**
- Keine Moderations-Features (Phase 2)
- Keine Netzwerk-Visualisierung (Cycle 6)

**Erfolgskriterien:**
- [ ] Alle Teilnehmer sichtbar
- [ ] Sprech-Indikator funktioniert
- [ ] Individuelle Lautstärke einstellbar

---

## Cycle 6: Mobile Foundation

### Pitch 6.1: Flutter Mobile App Basis

**Problem:**
Mobile ist heute der primäre Nutzungsort für Voice-Chat. Phase 1 muss mobile-ready sein.

**Appetite: 6 Wochen**

**Solution:**
```
Woche 1-2: Flutter-Projekt aufsetzen, libp2p FFI-Bindings
Woche 3-4: Core-Features portieren (Raum, Audio, Verschlüsselung)
Woche 5-6: Mobile-spezifische UI, System-Integration
```

**Mobile-Specific Challenges:**
- Batterie-Optimierung: Kein Mixer-Mode standardmäßig
- Hintergrund-Ausführung: OS-spezifische Workarounds
- Netzwerk-Wechsel: WiFi ↔ Mobilfunk Handoff

**Rabbit Holes:**
- iOS Background-Audio ist komplex → Audio-Session-Kategorien korrekt setzen
- FFI auf iOS/Android verschieden → unify-Layer bauen
- App-Store-Policies → vorbereiten, nicht blockieren

**No-Gos:**
- Keine Push-Benachrichtigungen (braucht Server, Phase 2)
- Keine Deep-Links (Cycle 7)
- Keine Widgets

**Erfolgskriterien:**
- [ ] iOS und Android Builds funktionieren
- [ ] Audio-Call mit Desktop-Client möglich
- [ ] App läuft 30 Min im Vordergrund ohne Absturz

---

### Pitch 6.2: Mobile UI & System-Integration

**Problem:**
Mobile UI muss touch-freundlich sein und sich nativ anfühlen.

**Appetite: 4 Wochen**

**Solution:**
- Mobile-First UI-Design
- System-Sharing für Raumeinladungen
- Lock-Screen-Controls für aktive Calls
- Adaptive Bitrate bei Netzwerk-Wechsel

**Rabbit Holes:**
- Verschiedene Screen-Größen → responsive Design
- OS-spezifische UI-Patterns → Material (Android) / Cupertino (iOS)

**No-Gos:**
- Keine Tablet-Optimierung
- Keine Landscape-Mode-Specials

**Erfolgskriterien:**
- [ ] UI fühlt sich nativ an
- [ ] Sharing-Integration funktioniert
- [ ] Netzwerk-Wechsel ohne Abbruch

---

## Cycle 7: Polish & Beta Release

### Pitch 7.1: Testing & Bug-Fixing

**Problem:**
Vor Beta-Release müssen kritische Bugs behoben und Stabilität gewährleistet sein.

**Appetite: 4 Wochen**

**Solution:**
- Automated Tests für kritische Pfade
- Manual Testing Matrix (3 OS × 3 Netzwerk-Typen)
- Bug Bash Week mit externen Testern
- Performance Profiling

**Testing Matrix:**
```
              │ Symmetric NAT │ Cone NAT │ No NAT │
──────────────┼───────────────┼──────────┼────────│
Windows       │       ✅      │    ✅    │   ✅   │
macOS         │       ✅      │    ✅    │   ✅   │
Linux         │       ✅      │    ✅    │   ✅   │
iOS           │       ✅      │    ✅    │   ✅   │
Android       │       ✅      │    ✅    │   ✅   │
```

**Rabbit Holes:**
- 100% Test-Abdeckung ist unrealistisch → kritische Pfade priorisieren
- Externe Tester finden Edge-Cases → Zeit einplanen

**No-Gos:**
- Keine neuen Features
- Keine Refactoring-Tangents

**Erfolgskriterien:**
- [ ] Keine kritischen Bugs
- [ ] Alle Test-Matrix-Felder grün
- [ ] < 5% Crash-Rate in Testing

---

### Pitch 7.2: Beta Release Preparation

**Problem:**
Beta-Release muss professionell wirken: Installer, Doku, GitHub-Präsenz.

**Appetite: 2 Wochen**

**Solution:**
- Installer für alle Plattformen
- README mit Quickstart
- Architecture-Dokumentation
- GitHub Releases Setup
- Beta-Testing-Community aufbauen

**Rabbit Holes:**
- Code-Signing ist aufwendig → erstmal self-signed, später official
- Doku kann ewig dauern → "Good enough, not perfect"

**No-Gos:**
- Keine Marketing-Kampagne
- Kein Product Hunt Launch

**Erfolgskriterien:**
- [ ] Downloadbare Builds für alle Plattformen
- [ ] README erklärt Setup in < 5 Minuten
- [ ] 10+ Beta-Tester gefunden

---

## Betting Table

### Was ist das Betting Table?
Am Ende jedes Cool-downs (2 Wochen nach jedem Cycle) trifft sich das Entscheidungsgremium:
- **Teilnehmer**: Core Team (3-5 Personen)
- **Input**: Gestaltete Pitches von den Shapern
- **Entscheidung**: Welche Pitches werden im nächsten Cycle gebaut?

### Betting-Regeln
1. **Fixed Capacity**: Ein Cycle = 6 Wochen = begrenztes "Budget"
2. **Must-Haves vs Nice-to-Haves**: Pitches können geteilt werden
3. **Rabbit Hole Protection**: Wenn ein Pitch zu riskant, wird er aufgeteilt oder verschoben
4. **No Partial Credit**: Ein Pitch ist fertig oder nicht - kein "80% done"

### Betting-Beispiel für Cycle 2
```
Verfügbares Budget: 6 Wochen × 3 Entwickler = 18 Entwickler-Wochen

Pitch 2.1 (Hole-Punching):     6 Wochen × 2 Devs = 12 Wochen
Pitch 2.2 (Verschlüsselung):   4 Wochen × 1.5 Devs = 6 Wochen

Total: 18 Wochen ✅ Passt genau!

Wenn ein Pitch überzogen:
→ Scope reduzieren (kein IPv6-Support in 2.1)
→ Oder: Pitch 2.2 auf Cycle 3 verschieben
```

---

## Cool-down Periods

### Was passiert im Cool-down?
**Dauer: 2 Wochen nach jedem Cycle**

1. **Deployment**: Letzte Änderungen deployen, Release erstellen
2. **Code Review**: Tech Debt identifizieren, aber nicht sofort fixen
3. **Shaping**: Am nächsten Cycle arbeiten (Shaper)
4. **Exploration**: Forschung für zukünftige Features
5. **Rest**: Verhinderung von Burnout

### Cool-down-Output
- Release Notes für abgeschlossenen Cycle
- Tech-Debt-Liste (nicht zwingend adressieren)
- Gestaltete Pitches für Betting Table

---

## Teams & Rollen

### Core Team (Phase 1)
| Rolle | Verantwortung | Count |
|-------|--------------|-------|
| Technical Lead | Architektur, libp2p, Networking | 1 |
| Audio Engineer | Audio-Pipeline, Codecs | 1 |
| Frontend Dev | Desktop UI, Mobile UI | 1-2 |
| Shaper/Product | Pitches, User Research | 1 |

### Shaper vs Maker
- **Shaper** (produktiv in Cool-down): Problem definieren, Appetite setzen, Solution skizzieren
- **Maker** (produktiv in Cycle): Code schreiben, Tests machen, Shippen

Eine Person kann beide Rollen haben, aber nicht gleichzeitig aktiv.

---

## Risikomanagement

### Technology Risks
| Risiko | Wahrscheinlichkeit | Impact | Mitigation |
|--------|-------------------|--------|------------|
| libp2p Inkompatibilität | Medium | High | Early Prototyping in Cycle 1 |
| Hole-Punching < 85% Success | Medium | High | TURN-Fallback robust machen |
| Audio-Latenz > 100ms | Low | High | Continuous Profiling |
| Mobile FFI-Probleme | Medium | Medium | Flutter-experten konsultieren |

### Schedule Risks
| Risiko | Wahrscheinlichkeit | Impact | Mitigation |
|--------|-------------------|--------|------------|
| Cycle überzogen | High | Medium | Scope flexibel halten |
| Key-Person-Risiko | Medium | High | Code-Reviews, Doku |
| Feature Creep | High | Medium | Strenge No-Go-Listen |

---

## Metriken & Tracking

### Cycle-Metriken
- **Velocity**: Wie viele Pitches wurden abgeschlossen?
- **Scope Changes**: Wie oft wurde Scope reduziert?
- **Cool-down Efficiency**: Wurden Pitches rechtzeitig gestaltet?

### Quality-Metriken (pro Cycle)
- Code Coverage
- Offene Bugs (kritisch/major/minor)
- Performance Benchmarks

### User-Metriken (ab Beta)
- Daily Active Users
- Session Duration
- Connection Success Rate
- Audio Quality Feedback

---

## Kommunikation

### Intern
- **Daily Standup**: 15 min, asynchron möglich (Text)
- **Weekly Sync**: 1 Stunde, alle im Call
- **Cycle Kickoff**: 1 Stunde am Cycle-Start
- **Cycle Retro**: 1 Stunde am Cycle-Ende

### Extern
- **Changelog**: Pro Release (alle 6 Wochen)
- **Discord/Forum**: Kontinuierliche Updates
- **GitHub Issues**: Transparentes Bug-Tracking
- **Roadmap**: Öffentlich, aktualisiert nach jedem Cycle

---

## Zusammenfassung

**Shape Up für Phase 1 Meshvoice:**

✅ **Vorteile:**
- Feste 6-Wochen-Cycles = vorhersehbare Releases
- Appetite-Driven = keine endlosen Schätzungen
- Pitches = klare Problemdefinition vor Coding
- Cool-down = nachhaltiges Tempo, kein Burnout

⚠️ **Gefahren:**
- Pitches zu detailliert → Lösung vorbestimmt, keine Kreativität
- Scope Creep trotz Fixed Time → Disziplin needed
- Zu viele parallele Pitches → Fokus verlieren

🎯 **Erfolgskriterien für Phase 1 mit Shape Up:**
- 6 Cycles abgeschlossen in 9-12 Monaten
- Funktionierende Beta auf Desktop + Mobile
- Hole-Punching > 85%, Audio-Latenz < 100ms
- 10+ Beta-Tester, offene GitHub-Community

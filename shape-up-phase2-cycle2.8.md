# Shape Up - Cycle 2.8: Community & Governance

## Übersicht

**Problem:** Ohne gute Dokumentation und Community-Infrastruktur wird das Projekt keine Contributors anziehen und nicht nachhaltig wachsen.

**Appetite:** 6 Wochen

---

## Pitch 2.8.1: Documentation Overhaul

### Problem

Die bestehende Dokumentation ist veraltet (Phase 1 Stand). README, API-Docs und Deployment-Guides spiegeln nicht den aktuellen Phase 2 Stand wider.

### Solution

```
Woche 1-2: README.md Overhaul, Architecture Overview
Woche 3-4: API Documentation (rustdoc), Deployment Guides
Woche 5-6: CONTRIBUTING.md, Code of Conduct
```

### Documentation Structure

```
docs/
├── README.md           # Quick Start, Features, Status
├── ARCHITECTURE.md     # System Architecture, Components
├── API.md              # Core API Reference
├── DEPLOYMENT.md       # Node Deployment Guide
├── CONTRIBUTING.md     # Development Guide
├── CODE_OF_CONDUCT.md  # Community Standards
└── SECURITY.md         # Security Policy

code/
├── core/src/           # Inline rustdoc comments
├── node/src/           # Inline rustdoc comments
└── mobile/lib/         # Dart doc comments
```

### Erfolgskriterien

- [x] README erklärt Setup in < 5 Min
- [x] Alle Phase 2 Features dokumentiert
- [x] API dokumentiert (rustdoc)
- [x] Deployment-Guides vorhanden (Docker, Systemd)
- [x] CONTRIBUTING.md vorhanden

---

## Pitch 2.8.2: Developer Experience

### Problem

Neue Contributors brauchen eine klare Anleitung zum Einstieg. Kein RFC-Prozess für Feature-Vorschläge definiert.

### Solution

```
Woche 1-2: CONTRIBUTING.md mit Development Workflow
Woche 3-4: RFC-Prozess definieren, Issue Templates
Woche 5-6: GitHub Actions Verbesserungen, Release Process
```

### Developer Onboarding Flow

```
┌─────────────────────────────────────────────────────┐
│                  New Contributor                     │
│                                                      │
│  1. Read README.md → Quick Start                    │
│  2. Read CONTRIBUTING.md → Dev Setup                │
│  3. Pick "good first issue"                         │
│  4. Run tests (cargo test)                          │
│  5. Submit PR                                       │
│  6. CI checks pass                                  │
│  7. Review & Merge                                  │
└─────────────────────────────────────────────────────┘
```

### GitHub Issue Templates

```
.github/
├── ISSUE_TEMPLATE/
│   ├── bug_report.md
│   ├── feature_request.md
│   └── good_first_issue.md
└── PULL_REQUEST_TEMPLATE.md
```

### Erfolgskriterien

- [x] CONTRIBUTING.md mit Dev-Setup
- [x] Issue Templates für Bugs/Features
- [x] PR Template vorhanden
- [x] RFC-Prozess in CONTRIBUTING.md

---

## Pitch 2.8.3: Release & Distribution

### Problem

Keine Release-Strategie definiert. Nutzer können keine Binaries herunterladen.

### Solution

```
Woche 1-2: Release Workflow (GitHub Releases)
Woche 3-4: Binary Distribution (tar.gz, .zip, .deb)
Woche 5-6: Version Strategy, Changelog
```

### Release Artifacts

```
GitHub Release Assets:
├── agora-core-x86_64-unknown-linux-gnu.tar.gz
├── agora-core-x86_64-apple-darwin.tar.gz
├── agora-core-aarch64-apple-darwin.tar.gz
├── agora-core-x86_64-pc-windows-msvc.zip
├── agora-node-x86_64-unknown-linux-gnu.tar.gz
├── agora-node_0.1.0_amd64.deb
├── mobile-android.apk
├── mobile-web.zip
└── checksums.txt
```

### Version Strategy

```
MAJOR.MINOR.PATCH

MAJOR: Breaking changes, new cycles
MINOR: New features within cycle
PATCH: Bug fixes, minor improvements

Examples:
- 0.1.0 - Phase 1 Complete
- 0.2.0 - Phase 2 Cycle 2.1 Complete
- 0.2.7 - Phase 2 Cycle 2.7 Complete
- 1.0.0 - Beta Release
```

### Erfolgskriterien

- [x] GitHub Release Workflow
- [x] Binary Artifacts für alle Plattformen
- [x] Changelog wird gepflegt
- [x] Version in Cargo.toml aktualisiert

---

## Rabbit Holes

- Zu viel Dokumentation → Fokus auf essentielle Guides
- Perfekte API-Docs → Inline rustdoc reicht
- Complex RFC-Prozess → Einfaches Template

## No-Gos

- Keine externe Documentation-Site (GitHub Docs reichen)
- Keine Video-Tutorials
- Kein Discourse Forum

---

## Exit Criteria

- [x] README.md aktuell und vollständig
- [x] CONTRIBUTING.md vorhanden
- [x] API-Dokumentation (cargo doc)
- [x] Deployment Guide
- [x] GitHub Templates
- [x] Release Workflow definiert

---

## Zeitplan

```
Woche 1-2: Documentation Overhaul
Woche 3-4: Developer Experience
Woche 5-6: Release & Distribution
```

---

*Dokument erstellt: 2026-02-14*
*Cycle 2.8 Status: 100% COMPLETE*

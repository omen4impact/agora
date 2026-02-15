# Roadmap - Phase 3 & Beyond

**Current Version:** 0.3.0-beta.1
**Last Updated:** 2026-02-15

---

## Between Beta and Phase 3

### Priority 1: Critical (Must Have)

| # | Task | Effort | Impact | Assignee |
|---|------|--------|--------|----------|
| 1 | **Bootstrap Peers** - Set up public bootstrap servers | 2 days | High | - |
| 2 | **Audio Pipeline** - Fix JACK/PulseAudio detection | 1 day | High | - |
| 3 | **Mobile Build** - Fix Android Kotlin/Gradle issues | 2 days | High | - |
| 4 | **Security Audit** - External security review | 1 week | Critical | - |

### Priority 2: High (Should Have)

| # | Task | Effort | Impact | Assignee |
|---|------|--------|--------|----------|
| 5 | **Demo Video** - Screen recording of working app | 1 day | Medium | - |
| 6 | **User Guide** - Step-by-step documentation | 2 days | Medium | - |
| 7 | **Docker Compose** - One-command deployment | 1 day | Medium | - |
| 8 | **Metrics Dashboard** - Grafana integration | 2 days | Medium | - |

### Priority 3: Medium (Nice to Have)

| # | Task | Effort | Impact | Assignee |
|---|------|--------|--------|----------|
| 9 | **WebRTC Signaling** - Complete signaling integration | 3 days | Medium | - |
| 10 | **iOS Build** - Fix iOS background audio | 2 days | Low | - |
| 11 | **Desktop App** - Tauri release builds | 3 days | Medium | - |
| 12 | **Community Setup** - Discord server, GitHub Discussions | 1 day | Low | - |

---

## Phase 3: Production Ready

### Goal
Transform from beta to production-ready with:
- 99.9% uptime bootstrap infrastructure
- Full mobile app releases (iOS + Android)
- Desktop app releases (Windows, macOS, Linux)
- Complete user documentation
- Community infrastructure

### Milestones

#### M3.1: Infrastructure (Week 1-2)
- [ ] Deploy bootstrap peers on multiple regions
- [ ] Set up TURN servers for relay fallback
- [ ] Configure monitoring (Prometheus + Grafana)
- [ ] Implement health checks and alerts

#### M3.2: Mobile Release (Week 3-4)
- [ ] Fix Android build pipeline
- [ ] iOS TestFlight release
- [ ] Android Play Store release
- [ ] Mobile-specific documentation

#### M3.3: Desktop Release (Week 5-6)
- [ ] Tauri production builds
- [ ] Code signing for Windows/macOS
- [ ] Auto-update mechanism
- [ ] Desktop user guide

#### M3.4: Documentation & Community (Week 7-8)
- [ ] Complete user documentation
- [ ] API reference documentation
- [ ] Video tutorials
- [ ] Community Discord/forum setup

---

## Future Considerations (Post Phase 3)

### Features
- End-to-end tests for real audio streaming
- Load testing with 100+ peers
- Bandwidth optimization
- Offline mode support
- Multi-room support

### Infrastructure
- Global bootstrap network (5+ regions)
- Managed TURN service
- CDN for mobile/desktop updates
- Analytics (privacy-preserving)

### Business
- Paid hosting for dedicated nodes
- Enterprise features (SSO, audit logs)
- SDK for third-party integration

---

## Technical Debt

### Code Quality
- [ ] Remove unused code in signaling.rs
- [ ] Improve FFI error messages
- [ ] Add integration tests for WebRTC
- [ ] Document public API (rustdoc)

### Security
- [ ] Upgrade ring dependency (RUSTSEC-2025-0009)
- [ ] Monitor transitive advisories
- [ ] Add security.txt

### Performance
- [ ] Profile audio pipeline latency
- [ ] Optimize DHT operations
- [ ] Memory leak audit

---

## Release Schedule

| Version | Target Date | Focus |
|---------|-------------|-------|
| 0.3.0-beta.2 | 2026-02-22 | Bug fixes, mobile builds |
| 0.3.0-rc.1 | 2026-03-01 | Release candidate |
| 0.3.0 | 2026-03-08 | Production release |
| 0.4.0 | 2026-04-01 | Phase 3 complete |
# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| < 0.2   | :x:                |

## Reporting a Vulnerability

**DO NOT** open a public issue for security vulnerabilities.

Instead, please report security issues privately:

1. Email: security@agora-voice.org
2. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We will respond within 48 hours and provide a timeline for a fix.

## Security Architecture

### Encryption

Agora uses multiple layers of encryption:

```
┌─────────────────────────────────────────────────────────────┐
│                    Encryption Layers                         │
│                                                              │
│  Transport Layer (Noise Protocol)                           │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ • Noise_XX handshake pattern                        │    │
│  │ • Ed25519 identity authentication                   │    │
│  │ • X25519 key exchange                               │    │
│  │ • Forward secrecy                                   │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  Application Layer (ChaCha20-Poly1305)                      │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ • Per-room session keys                             │    │
│  │ • Automatic key rotation (every hour)               │    │
│  │ • Replay attack protection (nonce counter)          │    │
│  │ • AEAD authentication                               │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Key Management

```
┌─────────────────────────────────────────────────────────────┐
│                     Key Hierarchy                            │
│                                                              │
│  Identity Key (Ed25519)                                      │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ • Long-term keypair                                 │    │
│  │ • Stored in ~/.config/agora/identity.bin           │    │
│  │ • Used for: signing, authentication                 │    │
│  └─────────────────────────────────────────────────────┘    │
│                           │                                  │
│                           ▼                                  │
│  Ephemeral Key (X25519)                                      │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ • Generated per handshake                           │    │
│  │ • Used for: Diffie-Hellman key exchange            │    │
│  │ • Discarded after session established              │    │
│  └─────────────────────────────────────────────────────┘    │
│                           │                                  │
│                           ▼                                  │
│  Session Key (ChaCha20)                                      │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ • Derived from DH shared secret                     │    │
│  │ • Rotated every hour                               │    │
│  │ • Used for: audio packet encryption                │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Trust Model

**What is trusted:**
- Your own identity key (stored locally)
- Peers you directly connect to (after Noise_XX handshake)
- DHT for peer discovery (but not for data storage)

**What is NOT trusted:**
- Network intermediaries (ISPs, etc.)
- Mixer nodes (cannot decrypt audio)
- Relay/TURN servers (cannot decrypt audio)
- Other peers' identity claims (must verify fingerprint)

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Eavesdropping | E2E encryption with ChaCha20-Poly1305 |
| Man-in-the-middle | Noise_XX mutual authentication |
| Replay attacks | Nonce counter, timestamp validation |
| Key compromise | Automatic key rotation, forward secrecy |
| Identity theft | Ed25519 signatures, fingerprint verification |
| Sybil attacks | Reputation system, proof-of-bandwidth |
| DDoS | Rate limiting, reputation requirements |

## Best Practices

### For Users

1. **Verify fingerprints** - When connecting to known contacts, verify their fingerprint matches
2. **Secure identity file** - Backup `~/.config/agora/identity.bin` securely
3. **Use strong passwords** - For password-protected rooms
4. **Keep software updated** - Security patches in new releases

### For Node Operators

1. **Keep system updated** - Regular security patches
2. **Use firewall** - Only expose necessary ports
3. **Monitor logs** - Watch for suspicious activity
4. **Limit resources** - Use systemd resource limits
5. **Run as non-root** - Create dedicated `agora` user

```bash
# Firewall example (ufw)
sudo ufw allow 7001/tcp  # P2P
sudo ufw allow 8080/tcp  # Dashboard (if public)
sudo ufw allow 9090/tcp  # Metrics (if public, restrict to monitoring)
```

### For Developers

1. **Never log secrets** - Keys, passwords, decrypted data
2. **Validate all inputs** - Especially from network
3. **Use constant-time comparisons** - For authentication tokens
4. **Follow Rust security guidelines** - `#![forbid(unsafe_code)]` where possible
5. **Audit dependencies** - Run `cargo audit` regularly

```bash
# Check for vulnerable dependencies
cargo install cargo-audit
cargo audit
```

## Known Security Considerations

### Metadata Privacy

While audio content is encrypted, the following metadata is visible to network observers:
- IP addresses of participants
- Room IDs (if DHT is used)
- Connection timing and duration
- Traffic patterns

Future work may include:
- Traffic padding
- Connection obfuscation
- Anonymous routing

### Social Engineering

The system cannot prevent:
- Impersonation if identity keys are stolen
- Social engineering attacks
- Compromised endpoints

### Forward Secrecy

ChaCha20-Poly1305 provides forward secrecy through:
- Hourly session key rotation
- Ephemeral X25519 keys per connection

If long-term identity key is compromised:
- Past sessions remain secure (forward secrecy)
- Future sessions can be impersonated

## Security Audit Status

**Status: Internal review only**

The cryptography implementation has not been externally audited. For production use with sensitive communications:
- Consider the risk/reward tradeoff
- Verify fingerprints out-of-band
- Keep software updated

We welcome security reviews and bug bounty reports.

## Changelog

### Security-relevant changes

| Version | Change |
|---------|--------|
| 0.2.1 | Initial implementation of ChaCha20-Poly1305 encryption |
| 0.2.1 | Noise_XX handshake with Ed25519 authentication |
| 0.2.1 | Session key rotation (1 hour) |
| 0.2.1 | Replay attack protection with nonce counter |

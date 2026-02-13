# Agora Mobile (Flutter)

Flutter mobile client for Agora P2P Voice Chat.

## Status

⚠️ **Not yet initialized** - Run `flutter create .` in this directory after installing Flutter.

## Prerequisites

- Flutter SDK 3.0+
- Android Studio / Xcode (for respective platforms)
- NDK for Rust FFI bindings

## Planned Features

- iOS and Android support
- FFI bindings to agora-core Rust library
- Native UI with Material (Android) and Cupertino (iOS) widgets
- Background audio support
- Push notifications (Phase 2)

## Setup (After Flutter Installation)

```bash
# Initialize Flutter project
flutter create . --org app.agora

# Add dependencies
flutter pub add ffi

# Build for iOS
flutter build ios

# Build for Android
flutter build apk
```

## Rust FFI Integration

The mobile app will use FFI to call into the `agora-core` Rust library:

1. Build Rust library for mobile targets:
   ```bash
   cargo build --target aarch64-linux-android
   cargo build --target aarch64-apple-ios
   ```

2. Generate FFI bindings using `flutter_rust_bridge` or manual bindings

3. Load native library in Dart via `dart:ffi`

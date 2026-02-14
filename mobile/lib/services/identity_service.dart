import 'package:flutter/foundation.dart';
import 'agora_ffi_stub.dart'
    if (dart.library.io) 'agora_ffi.dart'
    if (dart.library.js_interop) 'agora_ffi_web.dart';

class IdentityService extends ChangeNotifier {
  String? _peerId;
  String? _displayName;
  bool _isInitialized = false;
  final AgoraFFI _ffi = AgoraFFI();

  String? get peerId => _peerId;
  String? get displayName => _displayName;
  bool get isInitialized => _isInitialized;

  Future<void> initialize() async {
    if (_isInitialized) return;

    try {
      _peerId = _ffi.init();
      if (_peerId!.startsWith('ERROR:')) {
        _peerId = null;
        debugPrint('FFI init failed: ${_ffi.init()}');
        _peerId = _generateFallbackPeerId();
      }
    } catch (e) {
      debugPrint('FFI not available, using fallback: $e');
      _peerId = _generateFallbackPeerId();
    }

    _isInitialized = true;
    notifyListeners();
  }

  String _generateFallbackPeerId() {
    final timestamp = DateTime.now().millisecondsSinceEpoch;
    final hash = timestamp.toRadixString(16).padLeft(44, '0');
    return '12D3KooW${hash.substring(0, 44)}';
  }

  void setDisplayName(String name) {
    _displayName = name;
    notifyListeners();
  }

  String generateFingerprint() {
    if (_peerId == null) return '';
    return _peerId!.substring(_peerId!.length - 8).toUpperCase();
  }

  String generateRoomId() {
    try {
      return _ffi.generateRoomId();
    } catch (e) {
      final timestamp = DateTime.now().millisecondsSinceEpoch;
      return timestamp.toRadixString(16).padLeft(16, '0').substring(0, 16);
    }
  }

  String createRoomLink(String roomId) {
    try {
      return _ffi.createRoomLink(roomId);
    } catch (e) {
      return 'agora://room/$roomId';
    }
  }

  String? parseRoomLink(String link) {
    try {
      return _ffi.parseRoomLink(link);
    } catch (e) {
      final regex = RegExp(r'agora://room/([a-fA-F0-9]+)');
      final match = regex.firstMatch(link);
      return match?.group(1);
    }
  }

  String getSdkVersion() {
    try {
      return _ffi.version();
    } catch (e) {
      return '0.1.0';
    }
  }
}

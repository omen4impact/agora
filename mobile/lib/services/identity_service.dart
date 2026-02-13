import 'dart:math';
import 'package:flutter/foundation.dart';

class IdentityService extends ChangeNotifier {
  String? _peerId;
  String? _displayName;
  bool _isInitialized = false;

  String? get peerId => _peerId;
  String? get displayName => _displayName;
  bool get isInitialized => _isInitialized;

  Future<void> initialize() async {
    if (_isInitialized) return;
    
    _peerId = _generatePeerId();
    _isInitialized = true;
    notifyListeners();
  }

  String _generatePeerId() {
    final random = Random.secure();
    final bytes = List<int>.generate(32, (_) => random.nextInt(256));
    final hash = bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
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
}

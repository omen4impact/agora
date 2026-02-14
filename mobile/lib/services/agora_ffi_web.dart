import 'package:flutter/foundation.dart';

class AgoraFFI {
  String init() {
    return _generateFallbackPeerId();
  }

  String _generateFallbackPeerId() {
    final timestamp = DateTime.now().millisecondsSinceEpoch;
    final hash = timestamp.toRadixString(16).padLeft(44, '0');
    return '12D3KooW${hash.substring(0, 44)}';
  }

  String version() => '0.1.0-web';

  String generateRoomId() {
    final timestamp = DateTime.now().millisecondsSinceEpoch;
    return timestamp.toRadixString(16).padLeft(16, '0').substring(0, 16);
  }

  String createRoomLink(String roomId) {
    return 'agora://room/$roomId';
  }

  String parseRoomLink(String link) {
    final regex = RegExp(r'agora://room/([a-fA-F0-9]+)');
    final match = regex.firstMatch(link);
    return match?.group(1) ?? '';
  }

  void freeString(int ptr) {}
}

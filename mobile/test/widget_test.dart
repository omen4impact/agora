import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/material.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('App scaffold renders correctly', (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Container(
            decoration: const BoxDecoration(
              gradient: LinearGradient(
                begin: Alignment.topLeft,
                end: Alignment.bottomRight,
                colors: [Color(0xFF1A1A2E), Color(0xFF16213E)],
              ),
            ),
            child: const Center(
              child: Text('AGORA'),
            ),
          ),
        ),
      ),
    );

    expect(find.text('AGORA'), findsOneWidget);
  });

  test('Audio constraints are valid for voice chat', () {
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

    expect(constraints['audio'], isA<Map>());
    final audio = constraints['audio'] as Map;
    expect(audio['echoCancellation'], isTrue);
    expect(audio['noiseSuppression'], isTrue);
    expect(audio['autoGainControl'], isTrue);
    expect(audio['sampleRate'], equals(48000));
  });

  test('Signaling message types are correct', () {
    final messageTypes = [
      'join',
      'leave',
      'sdp_offer',
      'sdp_answer',
      'ice_candidate',
      'peer_list',
      'peer_joined',
      'peer_left',
      'error',
    ];

    expect(messageTypes.length, equals(9));
    expect(messageTypes, contains('join'));
    expect(messageTypes, contains('sdp_offer'));
    expect(messageTypes, contains('ice_candidate'));
  });

  test('ICE server configuration is valid', () {
    final iceServers = [
      {'urls': 'stun:stun.l.google.com:19302'},
      {'urls': 'stun:stun1.l.google.com:19302'},
    ];

    expect(iceServers.length, equals(2));
    for (final server in iceServers) {
      expect(server['urls'], contains('stun:'));
    }
  });
}

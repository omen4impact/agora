import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import '../services/identity_service.dart';
import 'session_screen.dart';

class JoinRoomScreen extends StatefulWidget {
  const JoinRoomScreen({super.key});

  @override
  State<JoinRoomScreen> createState() => _JoinRoomScreenState();
}

class _JoinRoomScreenState extends State<JoinRoomScreen> {
  final _linkController = TextEditingController();
  bool _isJoining = false;

  void _joinRoom() {
    final link = _linkController.text.trim();
    if (link.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Please enter a room link or ID'),
          backgroundColor: Colors.red,
        ),
      );
      return;
    }

    String roomId = link;
    if (link.startsWith('agora://room/')) {
      roomId = link.substring('agora://room/'.length);
      if (roomId.contains('?')) {
        roomId = roomId.substring(0, roomId.indexOf('?'));
      }
    }

    setState(() => _isJoining = true);

    final identity = context.read<IdentityService>();
    final parsed = identity.parseRoomLink(link);

    final resolvedRoomId = parsed ?? roomId;

    Navigator.pushReplacement(
      context,
      MaterialPageRoute(
        builder: (_) => SessionScreen(
          roomId: resolvedRoomId,
          roomLink: link.startsWith('agora://')
              ? link
              : 'agora://room/$resolvedRoomId',
        ),
      ),
    );
  }

  Future<void> _pasteFromClipboard() async {
    final data = await Clipboard.getData(Clipboard.kTextPlain);
    if (data?.text != null) {
      _linkController.text = data!.text!;
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Container(
        decoration: const BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: [Color(0xFF1A1A2E), Color(0xFF16213E)],
          ),
        ),
        child: SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(24.0),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    IconButton(
                      onPressed: () => Navigator.pop(context),
                      icon: const Icon(Icons.arrow_back, color: Colors.white),
                    ),
                    const Text(
                      'Join Room',
                      style: TextStyle(
                        fontSize: 24,
                        fontWeight: FontWeight.bold,
                        color: Colors.white,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 32),
                TextField(
                  controller: _linkController,
                  decoration: InputDecoration(
                    labelText: 'Room Link or ID',
                    labelStyle: TextStyle(color: Colors.grey[400]),
                    filled: true,
                    fillColor: Colors.white.withOpacity(0.05),
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(12),
                    ),
                    suffixIcon: IconButton(
                      onPressed: _pasteFromClipboard,
                      icon: const Icon(Icons.paste, color: Colors.grey),
                    ),
                  ),
                  style: const TextStyle(color: Colors.white),
                  onSubmitted: (_) => _joinRoom(),
                ),
                const SizedBox(height: 24),
                const Text(
                  'Example formats:',
                  style: TextStyle(
                    color: Colors.grey,
                    fontSize: 12,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  'agora://room/a1b2c3d4e5f6g7h8\na1b2c3d4e5f6g7h8',
                  style: TextStyle(
                    color: Colors.grey[600],
                    fontSize: 11,
                    fontFamily: 'monospace',
                  ),
                ),
                const Spacer(),
                SizedBox(
                  width: double.infinity,
                  height: 56,
                  child: ElevatedButton(
                    onPressed: _isJoining ? null : _joinRoom,
                    style: ElevatedButton.styleFrom(
                      backgroundColor: const Color(0xFF00D9FF),
                      foregroundColor: const Color(0xFF1A1A2E),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(16),
                      ),
                    ),
                    child: _isJoining
                        ? const CircularProgressIndicator()
                        : const Text(
                            'Join Room',
                            style: TextStyle(
                              fontSize: 16,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  @override
  void dispose() {
    _linkController.dispose();
    super.dispose();
  }
}

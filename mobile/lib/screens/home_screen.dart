import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/identity_service.dart';
import '../widgets/status_indicator.dart';
import 'create_room_screen.dart';
import 'join_room_screen.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  final _displayNameController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _initializeIdentity();
  }

  Future<void> _initializeIdentity() async {
    final identityService = context.read<IdentityService>();
    await identityService.initialize();
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
          child: Consumer<IdentityService>(
            builder: (context, identity, child) {
              return Padding(
                padding: const EdgeInsets.all(24.0),
                child: Column(
                  children: [
                    const Spacer(flex: 2),
                    _buildLogo(),
                    const SizedBox(height: 48),
                    _buildStatusCard(identity),
                    const SizedBox(height: 24),
                    _buildDisplayNameField(identity),
                    const Spacer(flex: 1),
                    _buildActionButtons(context),
                    const Spacer(flex: 2),
                  ],
                ),
              );
            },
          ),
        ),
      ),
    );
  }

  Widget _buildLogo() {
    return Column(
      children: [
        Container(
          width: 80,
          height: 80,
          decoration: BoxDecoration(
            gradient: const LinearGradient(
              colors: [Color(0xFF00D9FF), Color(0xFF00FF88)],
            ),
            borderRadius: BorderRadius.circular(24),
          ),
          child: const Icon(
            Icons.graphic_eq,
            size: 48,
            color: Color(0xFF1A1A2E),
          ),
        ),
        const SizedBox(height: 16),
        Text(
          'AGORA',
          style: TextStyle(
            fontSize: 36,
            fontWeight: FontWeight.bold,
            letterSpacing: 4,
            foreground: Paint()
              ..shader = const LinearGradient(
                colors: [Color(0xFF00D9FF), Color(0xFF00FF88)],
              ).createShader(const Rect.fromLTWH(0, 0, 200, 70)),
          ),
        ),
        const SizedBox(height: 8),
        Text(
          'Decentralized P2P Voice Chat',
          style: TextStyle(
            fontSize: 14,
            color: Colors.grey[400],
          ),
        ),
      ],
    );
  }

  Widget _buildStatusCard(IdentityService identity) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: Colors.white.withOpacity(0.05),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: Colors.white.withOpacity(0.1),
        ),
      ),
      child: Column(
        children: [
          Row(
            children: [
              StatusIndicator(
                isConnected: identity.isInitialized,
              ),
              const SizedBox(width: 12),
              Text(
                identity.isInitialized ? 'Ready' : 'Initializing...',
                style: const TextStyle(
                  fontSize: 16,
                  fontWeight: FontWeight.w500,
                ),
              ),
            ],
          ),
          if (identity.peerId != null) ...[
            const SizedBox(height: 12),
            Text(
              'Peer ID: ${identity.peerId!.substring(0, 20)}...',
              style: TextStyle(
                fontSize: 12,
                fontFamily: 'monospace',
                color: Colors.grey[500],
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildDisplayNameField(IdentityService identity) {
    return TextField(
      controller: _displayNameController,
      decoration: InputDecoration(
        labelText: 'Display Name (optional)',
        labelStyle: TextStyle(color: Colors.grey[400]),
        filled: true,
        fillColor: Colors.white.withOpacity(0.05),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide: BorderSide(color: Colors.white.withOpacity(0.2)),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide: BorderSide(color: Colors.white.withOpacity(0.2)),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide: const BorderSide(color: Color(0xFF00D9FF)),
        ),
      ),
      style: const TextStyle(color: Colors.white),
      onSubmitted: (value) {
        if (value.isNotEmpty) {
          identity.setDisplayName(value);
        }
      },
    );
  }

  Widget _buildActionButtons(BuildContext context) {
    return Column(
      children: [
        SizedBox(
          width: double.infinity,
          height: 56,
          child: ElevatedButton(
            onPressed: () {
              Navigator.push(
                context,
                MaterialPageRoute(builder: (_) => const CreateRoomScreen()),
              );
            },
            style: ElevatedButton.styleFrom(
              backgroundColor: const Color(0xFF00D9FF),
              foregroundColor: const Color(0xFF1A1A2E),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
            child: const Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(Icons.add_circle_outline),
                SizedBox(width: 8),
                Text(
                  'Create Room',
                  style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),
        SizedBox(
          width: double.infinity,
          height: 56,
          child: OutlinedButton(
            onPressed: () {
              Navigator.push(
                context,
                MaterialPageRoute(builder: (_) => const JoinRoomScreen()),
              );
            },
            style: OutlinedButton.styleFrom(
              foregroundColor: Colors.white,
              side: BorderSide(color: Colors.white.withOpacity(0.3)),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
            child: const Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(Icons.link),
                SizedBox(width: 8),
                Text(
                  'Join Room',
                  style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }

  @override
  void dispose() {
    _displayNameController.dispose();
    super.dispose();
  }
}

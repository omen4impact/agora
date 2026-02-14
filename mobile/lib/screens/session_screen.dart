import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/identity_service.dart';
import '../services/audio_service.dart';

class SessionScreen extends StatefulWidget {
  final String roomId;
  final String? roomName;
  final String roomLink;

  const SessionScreen({
    super.key,
    required this.roomId,
    this.roomName,
    required this.roomLink,
  });

  @override
  State<SessionScreen> createState() => _SessionScreenState();
}

class _SessionScreenState extends State<SessionScreen> with WidgetsBindingObserver {
  final List<Participant> _participants = [];
  late AudioService _audioService;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _audioService = AudioService();
    _addSelfParticipant();
    _initAudio();
  }

  void _addSelfParticipant() {
    final identity = context.read<IdentityService>();
    _participants.add(Participant(
      peerId: identity.peerId ?? 'unknown',
      name: identity.displayName ?? 'You',
      isSelf: true,
      isMixer: true,
      isMuted: false,
      latencyMs: 0,
    ));
  }

  Future<void> _initAudio() async {
    final hasPermission = await _audioService.checkPermissions();
    if (!hasPermission) {
      await _audioService.requestPermissions();
    }
    await _audioService.startAudio();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.paused) {
      // Keep audio running in background
    } else if (state == AppLifecycleState.resumed) {
      // Reconnect if needed
    }
  }

  void _toggleMute() {
    _audioService.toggleMute();
    setState(() {
      _participants.firstWhere((p) => p.isSelf).isMuted = _audioService.isMuted;
    });
  }

  void _toggleDeafen() {
    _audioService.toggleDeafen();
    setState(() {});
  }

  void _leaveRoom() async {
    await _audioService.stopAudio();
    if (mounted) {
      Navigator.pop(context);
    }
  }

  @override
  Widget build(BuildContext context) {
    return ChangeNotifierProvider.value(
      value: _audioService,
      child: Scaffold(
        body: Container(
          decoration: const BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
              colors: [Color(0xFF1A1A2E), Color(0xFF16213E)],
            ),
          ),
          child: SafeArea(
            child: Column(
              children: [
                _buildHeader(),
                Expanded(
                  child: _buildParticipantList(),
                ),
                _buildAudioLevelIndicator(),
                _buildControls(),
                _buildStatusBar(),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildHeader() {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: Colors.white.withOpacity(0.05),
        border: Border(
          bottom: BorderSide(
            color: Colors.white.withOpacity(0.1),
          ),
        ),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  widget.roomName ?? 'Voice Room',
                  style: const TextStyle(
                    fontSize: 20,
                    fontWeight: FontWeight.bold,
                    color: Colors.white,
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  'Room: ${widget.roomId.substring(0, 8)}...',
                  style: TextStyle(
                    fontSize: 12,
                    color: Colors.grey[400],
                  ),
                ),
              ],
            ),
          ),
          _buildTopologyBadge(),
        ],
      ),
    );
  }

  Widget _buildTopologyBadge() {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        color: const Color(0xFF00FF88).withOpacity(0.2),
        borderRadius: BorderRadius.circular(16),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 8,
            height: 8,
            decoration: const BoxDecoration(
              color: Color(0xFF00FF88),
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 8),
          Text(
            '${_participants.length} ${_participants.length == 1 ? 'participant' : 'participants'}',
            style: const TextStyle(
              color: Color(0xFF00FF88),
              fontSize: 12,
              fontWeight: FontWeight.bold,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildParticipantList() {
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: _participants.length,
      itemBuilder: (context, index) {
        return _ParticipantCard(
          participant: _participants[index],
          audioService: _audioService,
          onVolumeChanged: (volume) {
            debugPrint('Volume: $volume');
          },
        );
      },
    );
  }

  Widget _buildAudioLevelIndicator() {
    return Consumer<AudioService>(
      builder: (context, audio, child) {
        return Container(
          height: 4,
          margin: const EdgeInsets.symmetric(horizontal: 16),
          child: LinearProgressIndicator(
            value: audio.inputLevel,
            backgroundColor: Colors.white.withOpacity(0.1),
            valueColor: AlwaysStoppedAnimation<Color>(
              audio.inputLevel > 0.7
                  ? const Color(0xFFFF4444)
                  : audio.inputLevel > 0.3
                      ? const Color(0xFF00D9FF)
                      : const Color(0xFF00FF88),
            ),
          ),
        );
      },
    );
  }

  Widget _buildControls() {
    return Container(
      padding: const EdgeInsets.all(24),
      decoration: BoxDecoration(
        color: Colors.black.withOpacity(0.2),
        borderRadius: const BorderRadius.vertical(top: Radius.circular(24)),
      ),
      child: Consumer<AudioService>(
        builder: (context, audio, child) {
          return Column(
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                children: [
                  _buildControlButton(
                    icon: audio.isMuted ? Icons.mic_off : Icons.mic,
                    label: audio.isMuted ? 'Unmute' : 'Mute',
                    color: audio.isMuted ? const Color(0xFFFF4444) : Colors.white,
                    isActive: audio.isMuted,
                    onPressed: _toggleMute,
                  ),
                  _buildControlButton(
                    icon: audio.isDeafened ? Icons.volume_off : Icons.headset,
                    label: audio.isDeafened ? 'Undeafen' : 'Deafen',
                    color: audio.isDeafened ? const Color(0xFFFF4444) : Colors.white,
                    isActive: audio.isDeafened,
                    onPressed: _toggleDeafen,
                  ),
                  _buildControlButton(
                    icon: Icons.call_end,
                    label: 'Leave',
                    color: const Color(0xFFFF4444),
                    onPressed: _leaveRoom,
                  ),
                ],
              ),
              const SizedBox(height: 12),
              Text(
                audio.state == AudioState.active
                    ? '🎤 Microphone active'
                    : audio.state == AudioState.muted
                        ? '🔇 Microphone muted'
                        : audio.state == AudioState.error
                            ? '⚠️ Audio error'
                            : 'Initializing audio...',
                style: TextStyle(
                  fontSize: 12,
                  color: Colors.grey[400],
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  Widget _buildControlButton({
    required IconData icon,
    required String label,
    required Color color,
    bool isActive = false,
    required VoidCallback onPressed,
  }) {
    return Column(
      children: [
        GestureDetector(
          onTap: onPressed,
          child: Container(
            width: 64,
            height: 64,
            decoration: BoxDecoration(
              color: isActive 
                  ? color.withOpacity(0.2)
                  : Colors.white.withOpacity(0.1),
              shape: BoxShape.circle,
              border: Border.all(
                color: color.withOpacity(0.5),
                width: 2,
              ),
            ),
            child: Icon(
              icon,
              size: 28,
              color: color,
            ),
          ),
        ),
        const SizedBox(height: 8),
        Text(
          label,
          style: TextStyle(
            fontSize: 12,
            color: color,
          ),
        ),
      ],
    );
  }

  Widget _buildStatusBar() {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          const Icon(Icons.public, size: 16, color: Color(0xFF00D9FF)),
          const SizedBox(width: 8),
          Consumer<AudioService>(
            builder: (context, audio, child) {
              return Text(
                audio.isMuted ? 'Muted' : 'Audio Active',
                style: TextStyle(
                  fontSize: 12,
                  color: audio.isMuted ? Colors.grey[600] : const Color(0xFF00FF88),
                ),
              );
            },
          ),
        ],
      ),
    );
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _audioService.dispose();
    super.dispose();
  }
}

class Participant {
  final String peerId;
  final String name;
  final bool isSelf;
  final bool isMixer;
  bool isMuted;
  final int latencyMs;

  Participant({
    required this.peerId,
    required this.name,
    required this.isSelf,
    required this.isMixer,
    required this.isMuted,
    required this.latencyMs,
  });
}

class _ParticipantCard extends StatefulWidget {
  final Participant participant;
  final AudioService audioService;
  final ValueChanged<double> onVolumeChanged;

  const _ParticipantCard({
    required this.participant,
    required this.audioService,
    required this.onVolumeChanged,
  });

  @override
  State<_ParticipantCard> createState() => _ParticipantCardState();
}

class _ParticipantCardState extends State<_ParticipantCard> {
  double _volume = 100.0;

  @override
  Widget build(BuildContext context) {
    final latencyColor = widget.participant.latencyMs < 50
        ? const Color(0xFF00FF88)
        : widget.participant.latencyMs < 100
            ? const Color(0xFFFFAA00)
            : const Color(0xFFFF4444);

    final isSpeaking = widget.participant.isSelf && 
        !widget.participant.isMuted && 
        widget.audioService.inputLevel > 0.1;

    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: Colors.white.withOpacity(0.05),
        borderRadius: BorderRadius.circular(16),
        border: isSpeaking
            ? Border.all(color: const Color(0xFF00FF88), width: 2)
            : null,
      ),
      child: Row(
        children: [
          Stack(
            children: [
              AnimatedContainer(
                duration: const Duration(milliseconds: 200),
                width: 48,
                height: 48,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  gradient: LinearGradient(
                    begin: Alignment.topLeft,
                    end: Alignment.bottomRight,
                    colors: isSpeaking
                        ? [const Color(0xFF00FF88), const Color(0xFF00D9FF)]
                        : [const Color(0xFF00D9FF), const Color(0xFF00FF88)],
                  ),
                ),
                child: Center(
                  child: Text(
                    widget.participant.name[0].toUpperCase(),
                    style: const TextStyle(
                      fontSize: 20,
                      fontWeight: FontWeight.bold,
                      color: Color(0xFF1A1A2E),
                    ),
                  ),
                ),
              ),
              if (widget.participant.isMixer)
                Positioned(
                  right: 0,
                  bottom: 0,
                  child: Container(
                    padding: const EdgeInsets.all(2),
                    decoration: const BoxDecoration(
                      color: Color(0xFF00D9FF),
                      shape: BoxShape.circle,
                    ),
                    child: const Icon(
                      Icons.star,
                      size: 12,
                      color: Color(0xFF1A1A2E),
                    ),
                  ),
                ),
            ],
          ),
          const SizedBox(width: 16),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(
                      widget.participant.name,
                      style: const TextStyle(
                        fontSize: 16,
                        fontWeight: FontWeight.bold,
                        color: Colors.white,
                      ),
                    ),
                    if (widget.participant.isSelf) ...[
                      const SizedBox(width: 8),
                      Text(
                        '(You)',
                        style: TextStyle(
                          fontSize: 14,
                          color: Colors.grey[500],
                        ),
                      ),
                    ],
                    if (widget.participant.isMuted) ...[
                      const SizedBox(width: 8),
                      const Icon(
                        Icons.mic_off,
                        size: 16,
                        color: Color(0xFFFF4444),
                      ),
                    ],
                  ],
                ),
                const SizedBox(height: 4),
                Row(
                  children: [
                    Icon(
                      Icons.circle,
                      size: 8,
                      color: latencyColor,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      '${widget.participant.latencyMs}ms',
                      style: TextStyle(
                        fontSize: 12,
                        color: latencyColor,
                      ),
                    ),
                    const SizedBox(width: 16),
                    Text(
                      widget.participant.isMixer ? 'Mixer' : 'Participant',
                      style: TextStyle(
                        fontSize: 12,
                        color: Colors.grey[500],
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          if (!widget.participant.isSelf) ...[
            const SizedBox(width: 8),
            SizedBox(
              width: 100,
              child: Slider(
                value: _volume,
                onChanged: (value) {
                  setState(() {
                    _volume = value;
                  });
                  widget.onVolumeChanged(value);
                },
                activeColor: const Color(0xFF00D9FF),
                inactiveColor: Colors.white.withOpacity(0.1),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

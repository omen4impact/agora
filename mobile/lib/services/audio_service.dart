import 'dart:async';
import 'package:audioplayers/audioplayers.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_webrtc/flutter_webrtc.dart';

enum AudioState {
  idle,
  initializing,
  active,
  muted,
  error,
}

class AudioService extends ChangeNotifier {
  static final AudioService _instance = AudioService._internal();
  factory AudioService() => _instance;
  AudioService._internal();

  final AudioPlayer _audioPlayer = AudioPlayer();
  MediaStream? _localStream;

  AudioState _state = AudioState.idle;
  bool _isMuted = false;
  bool _isDeafened = false;
  double _inputLevel = 0.0;
  bool _isInitialized = false;

  AudioState get state => _state;
  bool get isMuted => _isMuted;
  bool get isDeafened => _isDeafened;
  double get inputLevel => _inputLevel;

  Future<void> initialize() async {
    if (_isInitialized) return;
    _isInitialized = true;
    debugPrint('AudioService initialized');
  }

  Future<bool> checkPermissions() async {
    try {
      final stream = await navigator.mediaDevices
          .getUserMedia({'audio': true, 'video': false});
      for (final track in stream.getTracks()) {
        track.stop();
      }
      return true;
    } catch (e) {
      debugPrint('Permission check failed: $e');
      return false;
    }
  }

  Future<bool> requestPermissions() async {
    return await checkPermissions();
  }

  Future<void> startAudio() async {
    if (_state == AudioState.active) return;

    _state = AudioState.initializing;
    notifyListeners();

    try {
      _localStream = await navigator.mediaDevices.getUserMedia({
        'audio': {
          'echoCancellation': true,
          'noiseSuppression': true,
          'autoGainControl': true,
          'sampleRate': 48000,
          'channelCount': 1,
        },
        'video': false,
      });

      _state = AudioState.active;
      debugPrint('Audio started via WebRTC');
      notifyListeners();
    } catch (e) {
      debugPrint('Error starting audio: $e');
      _state = AudioState.error;
      notifyListeners();
    }
  }

  Future<void> stopAudio() async {
    try {
      if (_localStream != null) {
        for (final track in _localStream!.getTracks()) {
          track.stop();
        }
        _localStream = null;
      }

      _state = AudioState.idle;
      _inputLevel = 0.0;
      debugPrint('Audio stopped');
      notifyListeners();
    } catch (e) {
      debugPrint('Error stopping audio: $e');
      _state = AudioState.error;
      notifyListeners();
    }
  }

  void toggleMute() {
    _isMuted = !_isMuted;
    _state = _isMuted ? AudioState.muted : AudioState.active;
    if (_localStream != null) {
      for (final track in _localStream!.getAudioTracks()) {
        track.enabled = !_isMuted;
      }
    }
    notifyListeners();
  }

  void toggleDeafen() {
    _isDeafened = !_isDeafened;
    if (_isDeafened) {
      _audioPlayer.setVolume(0.0);
    } else {
      _audioPlayer.setVolume(1.0);
    }
    notifyListeners();
  }

  void setNoiseThreshold(double threshold) {
    debugPrint('Noise threshold set to: $threshold');
  }

  Future<void> playTestSound() async {
    debugPrint('Playing test sound');
  }

  Future<void> playAudioFromPath(String path) async {
    if (_isDeafened) return;
    try {
      await _audioPlayer.play(DeviceFileSource(path));
    } catch (e) {
      debugPrint('Error playing audio: $e');
    }
  }

  @override
  void dispose() {
    stopAudio();
    _audioPlayer.dispose();
    super.dispose();
  }
}

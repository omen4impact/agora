import 'dart:async';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:web_socket_channel/web_socket_channel.dart';
import 'package:flutter_webrtc/flutter_webrtc.dart';

class SignalingMessage {
  final String type;
  final Map<String, dynamic> data;

  SignalingMessage({required this.type, required this.data});

  factory SignalingMessage.fromJson(Map<String, dynamic> json) {
    return SignalingMessage(
      type: json['type'] as String,
      data: Map<String, dynamic>.from(json)..remove('type'),
    );
  }

  Map<String, dynamic> toJson() => {'type': type, ...data};
}

class PeerInfo {
  final String peerId;
  final String? displayName;
  final int joinedAt;

  PeerInfo({
    required this.peerId,
    this.displayName,
    required this.joinedAt,
  });

  factory PeerInfo.fromJson(Map<String, dynamic> json) {
    return PeerInfo(
      peerId: json['peer_id'] as String,
      displayName: json['display_name'] as String?,
      joinedAt: json['joined_at'] as int,
    );
  }
}

class WebRTCService extends ChangeNotifier {
  static final WebRTCService _instance = WebRTCService._internal();
  factory WebRTCService() => _instance;
  WebRTCService._internal();

  WebSocketChannel? _signalingChannel;
  StreamSubscription? _signalingSubscription;

  final Map<String, RTCPeerConnection> _peerConnections = {};
  final Map<String, PeerInfo> _peers = {};

  MediaStream? _localStream;
  String? _currentRoomId;
  String? _localPeerId;
  String? _displayName;

  bool _isConnected = false;
  bool _isMuted = false;
  bool _isDeafened = false;

  bool get isConnected => _isConnected;
  bool get isMuted => _isMuted;
  bool get isDeafened => _isDeafened;
  String? get currentRoomId => _currentRoomId;
  String? get localPeerId => _localPeerId;
  Map<String, PeerInfo> get peers => Map.unmodifiable(_peers);

  final StreamController<SignalingMessage> _messageController =
      StreamController<SignalingMessage>.broadcast();
  Stream<SignalingMessage> get messages => _messageController.stream;

  final StreamController<MediaStream> _remoteStreamController =
      StreamController<MediaStream>.broadcast();
  Stream<MediaStream> get remoteStreams => _remoteStreamController.stream;

  Future<void> connectToSignaling(String serverUrl) async {
    if (_isConnected) return;

    try {
      _signalingChannel = WebSocketChannel.connect(
        Uri.parse('$serverUrl/ws?peer_id=${_localPeerId ?? ''}'),
      );

      _signalingSubscription = _signalingChannel!.stream.listen(
        _handleSignalingMessage,
        onError: (error) {
          debugPrint('Signaling error: $error');
          _isConnected = false;
          notifyListeners();
        },
        onDone: () {
          debugPrint('Signaling connection closed');
          _isConnected = false;
          notifyListeners();
        },
      );

      _isConnected = true;
      debugPrint('Connected to signaling server: $serverUrl');
      notifyListeners();
    } catch (e) {
      debugPrint('Failed to connect to signaling: $e');
      rethrow;
    }
  }

  void _handleSignalingMessage(dynamic message) {
    try {
      final json = jsonDecode(message as String) as Map<String, dynamic>;
      final msg = SignalingMessage.fromJson(json);

      debugPrint('Received signaling message: ${msg.type}');

      switch (msg.type) {
        case 'peer_list':
          _handlePeerList(msg.data);
          break;
        case 'peer_joined':
          _handlePeerJoined(msg.data);
          break;
        case 'peer_left':
          _handlePeerLeft(msg.data);
          break;
        case 'sdp_offer':
          _handleSdpOffer(msg.data);
          break;
        case 'sdp_answer':
          _handleSdpAnswer(msg.data);
          break;
        case 'ice_candidate':
          _handleIceCandidate(msg.data);
          break;
      }

      _messageController.add(msg);
    } catch (e) {
      debugPrint('Error handling signaling message: $e');
    }
  }

  void _send(SignalingMessage message) {
    if (_signalingChannel != null && _isConnected) {
      _signalingChannel!.sink.add(jsonEncode(message.toJson()));
    }
  }

  Future<void> joinRoom(String roomId, {String? displayName}) async {
    _currentRoomId = roomId;
    _displayName = displayName;

    _send(SignalingMessage(
      type: 'join',
      data: {
        'room_id': roomId,
        'peer_id': _localPeerId,
        'display_name': displayName,
      },
    ));
  }

  Future<void> leaveRoom() async {
    if (_currentRoomId == null) return;

    _send(SignalingMessage(
      type: 'leave',
      data: {
        'room_id': _currentRoomId,
        'peer_id': _localPeerId,
      },
    ));

    for (final pc in _peerConnections.values) {
      await pc.close();
    }
    _peerConnections.clear();
    _peers.clear();
    _currentRoomId = null;
    notifyListeners();
  }

  void _handlePeerList(Map<String, dynamic> data) {
    final peers = data['peers'] as List;
    for (final peer in peers) {
      final info = PeerInfo.fromJson(peer as Map<String, dynamic>);
      if (info.peerId != _localPeerId) {
        _peers[info.peerId] = info;
        _createPeerConnection(info.peerId);
      }
    }
    notifyListeners();
  }

  void _handlePeerJoined(Map<String, dynamic> data) {
    final peer = data['peer'] as Map<String, dynamic>;
    final info = PeerInfo.fromJson(peer);
    if (info.peerId != _localPeerId) {
      _peers[info.peerId] = info;
      _createPeerConnection(info.peerId);
      _createOffer(info.peerId);
    }
    notifyListeners();
  }

  void _handlePeerLeft(Map<String, dynamic> data) {
    final peerId = data['peer_id'] as String;
    _peers.remove(peerId);
    _peerConnections[peerId]?.close();
    _peerConnections.remove(peerId);
    notifyListeners();
  }

  Future<void> _createPeerConnection(String peerId) async {
    if (_peerConnections.containsKey(peerId)) return;

    final config = {
      'iceServers': [
        {'urls': 'stun:stun.l.google.com:19302'},
        {'urls': 'stun:stun1.l.google.com:19302'},
      ],
    };

    final pc = await createPeerConnection(config);

    pc.onIceCandidate = (candidate) {
      _send(SignalingMessage(
        type: 'ice_candidate',
        data: {
          'from': _localPeerId,
          'to': peerId,
          'candidate': candidate.candidate ?? '',
          'sdp_mid': candidate.sdpMid,
          'sdp_mline_index': candidate.sdpMLineIndex,
        },
      ));
    };

    pc.onTrack = (event) {
      if (event.track.kind == 'audio') {
        _remoteStreamController.add(event.streams[0]);
      }
    };

    if (_localStream != null) {
      for (final track in _localStream!.getTracks()) {
        pc.addTrack(track, _localStream!);
      }
    }

    _peerConnections[peerId] = pc;
  }

  Future<void> _createOffer(String peerId) async {
    final pc = _peerConnections[peerId];
    if (pc == null) return;

    final offer = await pc.createOffer();
    await pc.setLocalDescription(offer);

    _send(SignalingMessage(
      type: 'sdp_offer',
      data: {
        'from': _localPeerId,
        'to': peerId,
        'sdp': offer.sdp,
      },
    ));
  }

  Future<void> _handleSdpOffer(Map<String, dynamic> data) async {
    final from = data['from'] as String;
    final sdp = data['sdp'] as String;

    if (!_peerConnections.containsKey(from)) {
      await _createPeerConnection(from);
    }

    final pc = _peerConnections[from]!;
    await pc.setRemoteDescription(RTCSessionDescription(sdp, 'offer'));

    final answer = await pc.createAnswer();
    await pc.setLocalDescription(answer);

    _send(SignalingMessage(
      type: 'sdp_answer',
      data: {
        'from': _localPeerId,
        'to': from,
        'sdp': answer.sdp,
      },
    ));
  }

  Future<void> _handleSdpAnswer(Map<String, dynamic> data) async {
    final from = data['from'] as String;
    final sdp = data['sdp'] as String;

    final pc = _peerConnections[from];
    if (pc != null) {
      await pc.setRemoteDescription(RTCSessionDescription(sdp, 'answer'));
    }
  }

  Future<void> _handleIceCandidate(Map<String, dynamic> data) async {
    final from = data['from'] as String;
    final candidate = data['candidate'] as String;
    final sdpMid = data['sdp_mid'] as String?;
    final sdpMlineIndex = data['sdp_mline_index'] as int?;

    final pc = _peerConnections[from];
    if (pc != null) {
      await pc.addCandidate(RTCIceCandidate(candidate, sdpMid, sdpMlineIndex));
    }
  }

  Future<void> startAudio() async {
    if (_localStream != null) return;

    try {
      final media = await navigator.mediaDevices.getUserMedia({
        'audio': {
          'echoCancellation': true,
          'noiseSuppression': true,
          'autoGainControl': true,
          'sampleRate': 48000,
          'channelCount': 1,
        },
        'video': false,
      });

      _localStream = media;
      _localPeerId = DateTime.now().millisecondsSinceEpoch.toString();

      for (final pc in _peerConnections.values) {
        for (final track in media.getTracks()) {
          pc.addTrack(track, media);
        }
      }

      debugPrint('Audio started');
      notifyListeners();
    } catch (e) {
      debugPrint('Failed to start audio: $e');
      rethrow;
    }
  }

  Future<void> stopAudio() async {
    if (_localStream == null) return;

    for (final track in _localStream!.getTracks()) {
      track.stop();
    }
    _localStream = null;
    debugPrint('Audio stopped');
    notifyListeners();
  }

  void toggleMute() {
    if (_localStream == null) return;

    _isMuted = !_isMuted;
    for (final track in _localStream!.getAudioTracks()) {
      track.enabled = !_isMuted;
    }
    notifyListeners();
  }

  void toggleDeafen() {
    _isDeafened = !_isDeafened;
    notifyListeners();
  }

  Future<void> disconnect() async {
    await leaveRoom();
    await stopAudio();

    await _signalingSubscription?.cancel();
    await _signalingChannel?.sink.close();

    _signalingChannel = null;
    _signalingSubscription = null;
    _isConnected = false;
    _localPeerId = null;

    notifyListeners();
  }

  @override
  void dispose() {
    disconnect();
    _messageController.close();
    _remoteStreamController.close();
    super.dispose();
  }
}

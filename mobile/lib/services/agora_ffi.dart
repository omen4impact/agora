import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';

final DynamicLibrary _lib = _openLibrary();

DynamicLibrary _openLibrary() {
  if (Platform.isAndroid) {
    return DynamicLibrary.open('libagora_core.so');
  } else if (Platform.isIOS) {
    return DynamicLibrary.process();
  } else if (Platform.isMacOS) {
    return DynamicLibrary.open('libagora_core.dylib');
  } else if (Platform.isLinux) {
    return DynamicLibrary.open('libagora_core.so');
  } else if (Platform.isWindows) {
    return DynamicLibrary.open('agora_core.dll');
  }
  throw UnsupportedError('Unsupported platform');
}

class AgoraFFI {
  static final AgoraFFI _instance = AgoraFFI._internal();
  factory AgoraFFI() => _instance;
  AgoraFFI._internal();

  final _init =
      _lib.lookupFunction<Pointer<Utf8> Function(), Pointer<Utf8> Function()>(
          'agora_init');

  final _freeString = _lib.lookupFunction<Void Function(Pointer<Utf8>),
      void Function(Pointer<Utf8>)>('agora_free_string');

  final _generateRoomId =
      _lib.lookupFunction<Pointer<Utf8> Function(), Pointer<Utf8> Function()>(
          'agora_generate_room_id');

  final _createRoomLink = _lib.lookupFunction<
      Pointer<Utf8> Function(Pointer<Utf8>),
      Pointer<Utf8> Function(Pointer<Utf8>)>('agora_create_room_link');

  final _parseRoomLink = _lib.lookupFunction<
      Pointer<Utf8> Function(Pointer<Utf8>),
      Pointer<Utf8> Function(Pointer<Utf8>)>('agora_parse_room_link');

  final _version =
      _lib.lookupFunction<Pointer<Utf8> Function(), Pointer<Utf8> Function()>(
          'agora_version');

  final _detectNat =
      _lib.lookupFunction<_NATInfo Function(), _NATInfo Function()>(
          'agora_detect_nat');

  final _testMixer = _lib.lookupFunction<_MixerInfo Function(IntPtr),
      _MixerInfo Function(int)>('agora_test_mixer');

  String _readString(Pointer<Utf8> ptr) {
    try {
      return ptr.toDartString();
    } finally {
      _freeString(ptr);
    }
  }

  String init() {
    return _readString(_init());
  }

  String generateRoomId() {
    return _readString(_generateRoomId());
  }

  String createRoomLink(String roomId) {
    final roomIdPtr = roomId.toNativeUtf8();
    try {
      return _readString(_createRoomLink(roomIdPtr));
    } finally {
      calloc.free(roomIdPtr);
    }
  }

  String? parseRoomLink(String link) {
    final linkPtr = link.toNativeUtf8();
    try {
      final result = _readString(_parseRoomLink(linkPtr));
      if (result.startsWith('ERROR:')) {
        return null;
      }
      return result;
    } finally {
      calloc.free(linkPtr);
    }
  }

  String version() {
    return _readString(_version());
  }

  NATInfo detectNAT() {
    final native = _detectNat();
    return NATInfo(
      natType: _natTypeToString(native.natType),
      canHolePunch: native.canHolePunch,
      description: _readString(Pointer.fromAddress(native.description)),
    );
  }

  MixerInfo testMixer(int participants) {
    final native = _testMixer(participants);
    return MixerInfo(
      topology: _topologyToString(native.topology),
      participantCount: native.participantCount,
      isMixer: native.isMixer,
    );
  }

  static String _natTypeToString(int type) {
    return switch (type) {
      0 => 'Unknown',
      1 => 'OpenInternet',
      2 => 'FullCone',
      3 => 'RestrictedCone',
      4 => 'PortRestricted',
      5 => 'Symmetric',
      _ => 'Unknown',
    };
  }

  static String _topologyToString(int topology) {
    return switch (topology) {
      0 => 'FullMesh',
      1 => 'SFU',
      _ => 'Unknown',
    };
  }
}

final class _NATInfo extends Struct {
  @IntPtr()
  external int natType;

  @Bool()
  external bool canHolePunch;

  @IntPtr()
  external int description;
}

class NATInfo {
  final String natType;
  final bool canHolePunch;
  final String description;

  NATInfo({
    required this.natType,
    required this.canHolePunch,
    required this.description,
  });
}

final class _MixerInfo extends Struct {
  @IntPtr()
  external int topology;

  @IntPtr()
  external int participantCount;

  @Bool()
  external bool isMixer;
}

class MixerInfo {
  final String topology;
  final int participantCount;
  final bool isMixer;

  MixerInfo({
    required this.topology,
    required this.participantCount,
    required this.isMixer,
  });
}

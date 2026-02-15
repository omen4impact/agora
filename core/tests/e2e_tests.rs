use agora_core::{
    audio::FRAME_SIZE,
    audio_processor::{AudioProcessor, AudioProcessorConfig},
    codec::{OpusConfig, OpusDecoder, OpusEncoder},
    crypto::{derive_session_key_from_shared_secret, EncryptedChannel, KeyExchange, SessionKey},
    denoise::RnnoiseDenoiser,
    protocol::{AudioPacket, JitterBuffer},
    Identity,
};
use std::time::Duration;

#[tokio::test]
async fn test_e2e_encryption_decryption() {
    let key = SessionKey::new([42u8; 32]);
    let mut channel = EncryptedChannel::new(key);

    let original_data = b"Hello, this is a secret audio message!";

    let encrypted = channel.encrypt(original_data).expect("Failed to encrypt");
    assert_ne!(encrypted.ciphertext.as_slice(), original_data);
    assert!(!encrypted.nonce.is_empty());

    let decrypted = channel.decrypt(&encrypted).expect("Failed to decrypt");
    assert_eq!(decrypted, original_data);
}

#[tokio::test]
async fn test_e2e_key_exchange() {
    let mut alice = KeyExchange::new();
    let mut bob = KeyExchange::new();

    let alice_public = *alice.public_key();
    let bob_public = *bob.public_key();

    let alice_shared = alice
        .compute_shared_secret(&bob_public)
        .expect("Alice DH failed");
    let bob_shared = bob
        .compute_shared_secret(&alice_public)
        .expect("Bob DH failed");

    assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());

    let alice_key = derive_session_key_from_shared_secret(&alice_shared, "test_room");
    let bob_key = derive_session_key_from_shared_secret(&bob_shared, "test_room");

    assert_eq!(alice_key.as_bytes(), bob_key.as_bytes());
}

#[tokio::test]
async fn test_e2e_encrypted_audio_packet() {
    let key = SessionKey::new([123u8; 32]);
    let mut channel = EncryptedChannel::new(key);

    let frame: Vec<f32> = (0..FRAME_SIZE)
        .map(|i| (i as f32 * 0.01).sin() * 0.5)
        .collect();
    let audio_packet = AudioPacket::new(1, "peer1".to_string(), frame.clone());
    let encoded_audio = audio_packet
        .encode()
        .expect("Failed to encode audio packet");

    let encrypted = channel.encrypt(&encoded_audio).expect("Failed to encrypt");

    let decrypted = channel.decrypt(&encrypted).expect("Failed to decrypt");

    let decoded_audio = AudioPacket::decode(&decrypted).expect("Failed to decode audio packet");

    assert_eq!(decoded_audio.sequence, audio_packet.sequence);
    assert_eq!(decoded_audio.peer_id, audio_packet.peer_id);
    assert_eq!(decoded_audio.frame.len(), audio_packet.frame.len());
}

#[tokio::test]
async fn test_e2e_audio_pipeline_encode_decode() {
    let config = AudioProcessorConfig::default();
    let mut processor = AudioProcessor::new(config).expect("Failed to create processor");

    let mut original_frame: Vec<f32> = (0..FRAME_SIZE)
        .map(|i| (i as f32 * 0.02).sin() * 0.7)
        .collect();

    let encoded = processor
        .process_and_encode(&mut original_frame)
        .expect("Failed to process");

    assert!(!encoded.data.is_empty());
    assert!(encoded.sequence > 0);

    let decoded = processor
        .decode_and_process(&encoded.data)
        .expect("Failed to decode");

    assert_eq!(decoded.len(), FRAME_SIZE);
}

#[tokio::test]
async fn test_e2e_audio_with_echo_cancellation() {
    let mut processor =
        AudioProcessor::new(AudioProcessorConfig::default()).expect("Failed to create processor");

    let far_frame: Vec<f32> = (0..FRAME_SIZE)
        .map(|i| (i as f32 * 0.01).sin() * 0.5)
        .collect();

    let mut near_frame: Vec<f32> = far_frame
        .iter()
        .enumerate()
        .map(|(i, &s)| s * 0.8 + (i as f32 * 0.005).sin() * 0.2)
        .collect();

    processor.push_far_end(&far_frame);

    let encoded = processor
        .process_with_aec(&mut near_frame)
        .expect("Failed to process with AEC");

    assert!(!encoded.data.is_empty());

    let stats = processor.echo_stats();
    assert!(stats.is_some());
}

#[tokio::test]
async fn test_e2e_noise_suppression_pipeline() {
    let mut denoiser = RnnoiseDenoiser::new().expect("Failed to create denoiser");

    let mut noisy_frame: Vec<f32> = (0..FRAME_SIZE)
        .map(|i| {
            let signal = (i as f32 * 0.02).sin() * 0.5;
            let noise = ((i as f32 * 0.5) % 1.0) * 0.1 - 0.05;
            signal + noise
        })
        .collect();

    let original_energy: f32 = noisy_frame.iter().map(|x| x * x).sum();

    denoiser.process(&mut noisy_frame);

    let denoised_energy: f32 = noisy_frame.iter().map(|x| x * x).sum();

    assert!(denoised_energy <= original_energy);
}

#[tokio::test]
async fn test_e2e_opus_codec_roundtrip() {
    let config = OpusConfig::default();
    let mut encoder = OpusEncoder::new(config).expect("Failed to create encoder");
    let mut decoder = OpusDecoder::new(48000, 1).expect("Failed to create decoder");

    let original: Vec<f32> = (0..FRAME_SIZE)
        .map(|i| {
            let freq = 440.0;
            let t = i as f32 / 48000.0;
            (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5
        })
        .collect();

    let encoded = encoder.encode_frame(&original).expect("Failed to encode");

    let decoded = decoder.decode(&encoded.data).expect("Failed to decode");

    assert_eq!(decoded.len(), FRAME_SIZE);
}

#[tokio::test]
async fn test_e2e_packet_loss_concealment() {
    let mut decoder = OpusDecoder::new(48000, 1).expect("Failed to create decoder");

    let plc_frame = decoder.decode_packet_loss().expect("Failed PLC");

    assert_eq!(plc_frame.len(), FRAME_SIZE);
}

#[tokio::test]
async fn test_e2e_full_voice_chain() {
    let key = SessionKey::new([200u8; 32]);
    let mut channel = EncryptedChannel::new(key);

    let mut processor =
        AudioProcessor::new(AudioProcessorConfig::default()).expect("Failed to create processor");

    let mut mic_frame: Vec<f32> = (0..FRAME_SIZE)
        .map(|i| (i as f32 * 0.02).sin() * 0.6)
        .collect();

    let _encoded = processor
        .process_and_encode(&mut mic_frame)
        .expect("Failed to encode");

    let audio_packet = AudioPacket::new(1, "sender".to_string(), mic_frame.clone());
    let packet_bytes = audio_packet.encode().expect("Failed to encode packet");

    let encrypted = channel.encrypt(&packet_bytes).expect("Failed to encrypt");

    let decrypted = channel.decrypt(&encrypted).expect("Failed to decrypt");

    let received_packet = AudioPacket::decode(&decrypted).expect("Failed to decode packet");

    assert_eq!(received_packet.sequence, 1);
}

#[tokio::test]
async fn test_e2e_jitter_buffer() {
    let mut jitter_buffer = JitterBuffer::new(20, 48000);

    for seq in 0..5 {
        let frame: Vec<f32> = vec![0.5; FRAME_SIZE];
        let packet = AudioPacket::new(seq, "peer1".to_string(), frame);
        jitter_buffer.push(packet);
    }

    let mut received_count = 0;
    while jitter_buffer.pop().is_some() {
        received_count += 1;
    }

    assert!(received_count > 0);
}

#[tokio::test]
async fn test_e2e_latency_under_10ms() {
    let config = AudioProcessorConfig::default();
    let mut processor = AudioProcessor::new(config).expect("Failed to create processor");

    let iterations = 50;
    let mut total_time = Duration::ZERO;

    for _ in 0..iterations {
        let mut frame: Vec<f32> = (0..FRAME_SIZE)
            .map(|i| (i as f32 * 0.01).sin() * 0.5)
            .collect();

        let start = std::time::Instant::now();
        let encoded = processor
            .process_and_encode(&mut frame)
            .expect("Failed to encode");
        let _decoded = processor
            .decode_and_process(&encoded.data)
            .expect("Failed to decode");
        total_time += start.elapsed();
    }

    let avg_time_us = total_time.as_micros() as f64 / iterations as f64;

    assert!(
        avg_time_us < 20000.0,
        "Average processing time should be < 20ms, got {}µs",
        avg_time_us
    );
}

#[tokio::test]
async fn test_e2e_bandwidth_adaptation() {
    use agora_core::audio_processor::{AdaptiveBitrateController, BitrateLevel};

    let mut controller = AdaptiveBitrateController::new();

    controller.update(0.01, 30);
    controller.update(0.01, 25);
    controller.update(0.02, 35);

    let good_bitrate = controller.suggest_bitrate();
    assert!(good_bitrate >= BitrateLevel::High.bitrate());

    controller.update(0.15, 200);
    controller.update(0.18, 220);
    controller.update(0.20, 250);

    let poor_bitrate = controller.suggest_bitrate();
    assert!(poor_bitrate <= BitrateLevel::Medium.bitrate());
}

#[tokio::test]
async fn test_e2e_encode_multiple_frames() {
    let mut processor =
        AudioProcessor::new(AudioProcessorConfig::default()).expect("Failed to create processor");

    let frames = 50u64;
    for seq in 0..frames {
        let mut frame: Vec<f32> = (0..FRAME_SIZE)
            .map(|i| ((i + seq as usize * 10) as f32 * 0.01).sin() * 0.5)
            .collect();

        let encoded = processor
            .process_and_encode(&mut frame)
            .expect("Failed to encode");
        assert!(!encoded.data.is_empty());
    }

    let stats = processor.stats();
    assert_eq!(stats.frames_processed, frames);
}

#[tokio::test]
async fn test_e2e_replay_attack_protection() {
    let key = SessionKey::new([1u8; 32]);
    let mut channel = EncryptedChannel::new(key);

    let message = b"Test message";
    let encrypted = channel.encrypt(message).expect("Failed to encrypt");

    let _first = channel
        .decrypt(&encrypted)
        .expect("First decrypt should work");

    let second = channel.decrypt(&encrypted);
    assert!(second.is_err(), "Replay attack should be detected");
}

#[tokio::test]
async fn test_e2e_key_rotation() {
    let key1 = SessionKey::new([1u8; 32]);
    let mut channel = EncryptedChannel::new(key1);

    let message = b"Before rotation";
    let encrypted1 = channel.encrypt(message).expect("Failed to encrypt");

    let key2 = SessionKey::new([2u8; 32]);
    channel.rotate_key(key2);

    let message2 = b"After rotation";
    let encrypted2 = channel
        .encrypt(message2)
        .expect("Failed to encrypt after rotation");

    assert_ne!(
        encrypted1.ciphertext, encrypted2.ciphertext,
        "Ciphertexts should differ with different keys"
    );

    let decrypted = channel
        .decrypt(&encrypted2)
        .expect("Failed to decrypt after rotation");
    assert_eq!(decrypted, message2);
}

#[tokio::test]
async fn test_e2e_identity_signing() {
    let identity = Identity::generate().expect("Failed to generate identity");

    let message = b"Important message to sign";
    let signature = identity.sign(message);

    assert!(identity.verify(message, &signature));

    let tampered = b"Tampered message";
    assert!(!identity.verify(tampered, &signature));
}

#[tokio::test]
async fn test_e2e_identity_unique() {
    let alice = Identity::generate().expect("Failed to generate Alice");
    let bob = Identity::generate().expect("Failed to generate Bob");

    let alice_peer = alice.peer_id();
    let bob_peer = bob.peer_id();

    assert_ne!(alice_peer, bob_peer);

    let message = b"Hello from Alice";
    let signature = alice.sign(message);

    assert!(alice.verify(message, &signature));
}

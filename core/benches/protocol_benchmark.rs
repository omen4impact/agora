use agora_core::protocol::{AudioPacket, ControlMessage, ControlMessageType, JitterBuffer};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn generate_audio_frame(samples: usize) -> Vec<f32> {
    (0..samples)
        .map(|i| ((i as f32 / samples as f32) * 2.0 - 1.0) * 0.5)
        .collect()
}

fn bench_audio_packet_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_packet_creation");

    for samples in [480, 960, 1920].iter() {
        let frame = generate_audio_frame(*samples);
        group.throughput(Throughput::Elements(*samples as u64));
        group.bench_with_input(BenchmarkId::from_parameter(samples), samples, |b, _| {
            b.iter(|| black_box(AudioPacket::new(42, "peer_123".to_string(), frame.clone())));
        });
    }
    group.finish();
}

fn bench_audio_packet_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_packet_encode");

    for samples in [480, 960, 1920].iter() {
        let frame = generate_audio_frame(*samples);
        let packet = AudioPacket::new(42, "peer_123".to_string(), frame);
        group.throughput(Throughput::Elements(*samples as u64));
        group.bench_with_input(BenchmarkId::from_parameter(samples), samples, |b, _| {
            b.iter(|| black_box(packet.encode().unwrap()));
        });
    }
    group.finish();
}

fn bench_audio_packet_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_packet_decode");

    for samples in [480, 960, 1920].iter() {
        let frame = generate_audio_frame(*samples);
        let packet = AudioPacket::new(42, "peer_123".to_string(), frame);
        let encoded = packet.encode().unwrap();
        group.throughput(Throughput::Elements(*samples as u64));
        group.bench_with_input(BenchmarkId::from_parameter(samples), samples, |b, _| {
            b.iter(|| black_box(AudioPacket::decode(&encoded).unwrap()));
        });
    }
    group.finish();
}

fn bench_audio_packet_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_packet_roundtrip");

    let frame = generate_audio_frame(960);
    let packet = AudioPacket::new(42, "peer_123".to_string(), frame);

    group.bench_function("encode_decode_960", |b| {
        b.iter(|| {
            let encoded = packet.encode().unwrap();
            let decoded = AudioPacket::decode(&encoded).unwrap();
            black_box((encoded, decoded))
        });
    });

    group.finish();
}

fn bench_control_message_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("control_message_creation");

    group.bench_function("join_room", |b| {
        b.iter(|| {
            black_box(ControlMessage::join_room(
                "room_123".to_string(),
                "peer_456".to_string(),
            ))
        });
    });

    group.bench_function("leave_room", |b| {
        b.iter(|| {
            black_box(ControlMessage::leave_room(
                "room_123".to_string(),
                "peer_456".to_string(),
            ))
        });
    });

    group.bench_function("mute_changed", |b| {
        b.iter(|| black_box(ControlMessage::mute_changed("peer_456".to_string(), true)));
    });

    group.finish();
}

fn bench_control_message_encode_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("control_message_encode_decode");

    let msg = ControlMessage::join_room("room_123".to_string(), "peer_456".to_string());

    group.bench_function("encode", |b| {
        b.iter(|| black_box(msg.encode().unwrap()));
    });

    let encoded = msg.encode().unwrap();
    group.bench_function("decode", |b| {
        b.iter(|| black_box(ControlMessage::decode(&encoded).unwrap()));
    });

    group.finish();
}

fn bench_jitter_buffer_push_pop(c: &mut Criterion) {
    let mut group = c.benchmark_group("jitter_buffer");

    let frame = generate_audio_frame(960);

    group.bench_function("push", |b| {
        b.iter(|| {
            let mut buffer = JitterBuffer::new(100, 48000);
            for seq in 0..10 {
                let packet = AudioPacket::new(seq, "peer_123".to_string(), frame.clone());
                buffer.push(packet);
            }
            black_box(buffer)
        });
    });

    group.bench_function("pop", |b| {
        let mut buffer = JitterBuffer::new(100, 48000);
        for seq in 0..10 {
            let packet = AudioPacket::new(seq, "peer_123".to_string(), frame.clone());
            buffer.push(packet);
        }
        b.iter(|| black_box(buffer.pop()));
    });

    group.finish();
}

fn bench_jitter_buffer_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("jitter_buffer_throughput");

    let frame = generate_audio_frame(960);

    group.bench_function("push_100_packets", |b| {
        b.iter(|| {
            let mut buffer = JitterBuffer::new(200, 48000);
            for seq in 0..100 {
                let packet = AudioPacket::new(seq, "peer_123".to_string(), frame.clone());
                buffer.push(packet);
            }
            black_box(buffer)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_audio_packet_creation,
    bench_audio_packet_encode,
    bench_audio_packet_decode,
    bench_audio_packet_roundtrip,
    bench_control_message_creation,
    bench_control_message_encode_decode,
    bench_jitter_buffer_push_pop,
    bench_jitter_buffer_throughput,
);

criterion_main!(benches);

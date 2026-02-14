use agora_core::codec::{OpusConfig, OpusDecoder, OpusEncoder, OpusMode, OPUS_FRAME_SIZE};
use agora_core::denoise::{RnnoiseDenoiser, RNNOISE_FRAME_SIZE};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn generate_audio_frame(samples: usize) -> Vec<f32> {
    (0..samples)
        .map(|i| ((i as f32 / samples as f32) * 2.0 - 1.0) * 0.5)
        .collect()
}

fn bench_opus_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("opus_encode");

    let config = OpusConfig::default();
    let mut encoder = OpusEncoder::new(config).unwrap();

    let frame = generate_audio_frame(OPUS_FRAME_SIZE);
    group.throughput(Throughput::Elements(OPUS_FRAME_SIZE as u64));
    group.bench_function("encode_960_samples", |b| {
        b.iter(|| black_box(encoder.encode(&frame).unwrap()));
    });
    group.finish();
}

fn bench_opus_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("opus_decode");

    let config = OpusConfig::default();
    let mut encoder = OpusEncoder::new(config.clone()).unwrap();
    let mut decoder = OpusDecoder::new(48000, 1).unwrap();

    let frame = generate_audio_frame(OPUS_FRAME_SIZE);
    let encoded = encoder.encode(&frame).unwrap();

    group.throughput(Throughput::Elements(OPUS_FRAME_SIZE as u64));
    group.bench_function("decode_960_samples", |b| {
        b.iter(|| black_box(decoder.decode(&encoded).unwrap()));
    });

    group.finish();
}

fn bench_opus_encode_decode_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("opus_roundtrip");

    let config = OpusConfig::default();
    let mut encoder = OpusEncoder::new(config.clone()).unwrap();
    let mut decoder = OpusDecoder::new(48000, 1).unwrap();

    let frame = generate_audio_frame(OPUS_FRAME_SIZE);

    group.bench_function("encode_decode_960_samples", |b| {
        b.iter(|| {
            let encoded = encoder.encode(&frame).unwrap();
            let decoded = decoder.decode(&encoded).unwrap();
            black_box((encoded, decoded))
        });
    });

    group.finish();
}

fn bench_opus_bitrate_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("opus_bitrate_modes");

    let frame = generate_audio_frame(OPUS_FRAME_SIZE);

    for (name, mode) in [
        ("voip", OpusMode::Voip),
        ("audio", OpusMode::Audio),
        ("low_delay", OpusMode::LowDelay),
    ] {
        let config = OpusConfig::default().with_mode(mode);
        let mut encoder = OpusEncoder::new(config).unwrap();

        group.bench_function(name, |b| {
            b.iter(|| black_box(encoder.encode(&frame).unwrap()));
        });
    }

    group.finish();
}

fn bench_opus_bitrate_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("opus_bitrate_levels");

    let frame = generate_audio_frame(OPUS_FRAME_SIZE);

    for (name, bitrate) in [
        ("low_16k", 16000),
        ("medium_32k", 32000),
        ("high_64k", 64000),
        ("max_128k", 128000),
    ] {
        let config = OpusConfig::default().with_bitrate(bitrate);
        let mut encoder = OpusEncoder::new(config).unwrap();

        group.bench_function(name, |b| {
            b.iter(|| black_box(encoder.encode(&frame).unwrap()));
        });
    }

    group.finish();
}

fn bench_rnnoise_denoise(c: &mut Criterion) {
    let mut group = c.benchmark_group("rnnoise_denoise");

    let mut denoiser = RnnoiseDenoiser::new().unwrap();
    let frame = generate_audio_frame(OPUS_FRAME_SIZE);

    group.throughput(Throughput::Elements(OPUS_FRAME_SIZE as u64));
    group.bench_function("denoise_960_samples", |b| {
        b.iter(|| {
            let mut frame_copy = frame.clone();
            denoiser.process(&mut frame_copy);
            black_box(frame_copy)
        });
    });
    group.finish();
}

fn bench_combined_audio_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("combined_audio_pipeline");

    let frame = generate_audio_frame(OPUS_FRAME_SIZE);

    let config = OpusConfig::default();
    let mut encoder = OpusEncoder::new(config.clone()).unwrap();
    let mut decoder = OpusDecoder::new(48000, 1).unwrap();
    let mut denoiser = RnnoiseDenoiser::new().unwrap();

    group.bench_function("full_pipeline_960_samples", |b| {
        b.iter(|| {
            let mut frame_copy = frame.clone();
            denoiser.process(&mut frame_copy);
            let encoded = encoder.encode(&frame_copy).unwrap();
            let decoded = decoder.decode(&encoded).unwrap();
            black_box((frame_copy, encoded, decoded))
        });
    });

    group.finish();
}

fn bench_encoder_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("encoder_creation");

    let config = OpusConfig::default();

    group.bench_function("create_opus_encoder", |b| {
        b.iter(|| black_box(OpusEncoder::new(config.clone()).unwrap()));
    });

    group.bench_function("create_opus_decoder", |b| {
        b.iter(|| black_box(OpusDecoder::new(48000, 1).unwrap()));
    });

    group.bench_function("create_rnnoise_denoiser", |b| {
        b.iter(|| black_box(RnnoiseDenoiser::new().unwrap()));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_opus_encode,
    bench_opus_decode,
    bench_opus_encode_decode_roundtrip,
    bench_opus_bitrate_modes,
    bench_opus_bitrate_levels,
    bench_rnnoise_denoise,
    bench_combined_audio_pipeline,
    bench_encoder_creation,
);

criterion_main!(benches);

use agora_core::crypto::{
    EncryptedChannel, EncryptedMessage, KeyExchange, SessionKey, SessionKeyManager,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn generate_test_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

fn bench_encryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption");

    let key = SessionKey::new([0u8; 32]);

    for size in [100, 480, 960, 1920, 3840].iter() {
        let data = generate_test_data(*size * 4);
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut channel = EncryptedChannel::new(key.clone());
                black_box(channel.encrypt(&data).unwrap())
            });
        });
    }
    group.finish();
}

fn bench_decryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("decryption");

    let key = SessionKey::new([0u8; 32]);

    for size in [100, 480, 960, 1920, 3840].iter() {
        let data = generate_test_data(*size * 4);
        let mut channel = EncryptedChannel::new(key.clone());
        let encrypted = channel.encrypt(&data).unwrap();
        group.throughput(Throughput::Bytes(encrypted.to_bytes().len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut ch = EncryptedChannel::new(key.clone());
                black_box(ch.decrypt(&encrypted).unwrap())
            });
        });
    }
    group.finish();
}

fn bench_key_exchange(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_exchange");

    let key_exchange = KeyExchange::new();
    let peer_public = *key_exchange.public_key();

    group.bench_function("x25519_keypair_generation", |b| {
        b.iter(|| black_box(KeyExchange::new()));
    });

    group.bench_function("x25519_shared_secret", |b| {
        b.iter(|| {
            let mut ke = KeyExchange::new();
            black_box(ke.compute_shared_secret(&peer_public).unwrap())
        });
    });

    group.finish();
}

fn bench_session_key_derivation(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_key_derivation");

    let key = SessionKey::new([0u8; 32]);

    group.bench_function("derive_for_peer", |b| {
        b.iter(|| black_box(key.derive_for_peer(b"peer_123")));
    });

    group.bench_function("clone_key", |b| {
        b.iter(|| black_box(key.clone()));
    });

    group.finish();
}

fn bench_combined_crypto_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("crypto_pipeline");

    let key = SessionKey::new([0u8; 32]);
    let data = generate_test_data(960 * 4);

    group.bench_function("encrypt_decrypt_960_samples_f32", |b| {
        b.iter(|| {
            let mut channel = EncryptedChannel::new(key.clone());
            let encrypted = channel.encrypt(&data).unwrap();
            let decrypted = channel.decrypt(&encrypted).unwrap();
            black_box((encrypted, decrypted))
        });
    });

    group.finish();
}

fn bench_encrypted_message_serde(c: &mut Criterion) {
    let mut group = c.benchmark_group("encrypted_message_serde");

    let key = SessionKey::new([0u8; 32]);
    let mut channel = EncryptedChannel::new(key);

    let data = generate_test_data(960 * 4);
    let encrypted = channel.encrypt(&data).unwrap();
    let bytes = encrypted.to_bytes();

    group.bench_function("to_bytes", |b| {
        b.iter(|| black_box(encrypted.to_bytes()));
    });

    group.bench_function("from_bytes", |b| {
        b.iter(|| black_box(EncryptedMessage::from_bytes(&bytes).unwrap()));
    });

    group.finish();
}

fn bench_session_key_manager_room_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_key_manager");

    group.bench_function("create_room", |b| {
        b.iter(|| {
            let mut manager = SessionKeyManager::new();
            let key = SessionKey::new([0u8; 32]);
            black_box(manager.create_room("test_room", key))
        });
    });

    let mut manager = SessionKeyManager::new();
    let key = SessionKey::new([0u8; 32]);
    manager.create_room("test_room", key);

    let data = generate_test_data(960);

    group.bench_function("encrypt_decrypt", |b| {
        b.iter(|| {
            let encrypted = manager.encrypt("test_room", &data).unwrap();
            let decrypted = manager.decrypt("test_room", &encrypted).unwrap();
            black_box((encrypted, decrypted))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_encryption,
    bench_decryption,
    bench_key_exchange,
    bench_session_key_derivation,
    bench_combined_crypto_pipeline,
    bench_encrypted_message_serde,
    bench_session_key_manager_room_operations,
);

criterion_main!(benches);

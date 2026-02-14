use agora_core::ice::{Candidate, CandidateType, IceAgent, IceConfig, TransportType};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn bench_candidate_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ice_candidate_creation");

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 7001);

    group.bench_function("host_candidate", |b| {
        b.iter(|| black_box(Candidate::new_host(addr, 1)));
    });

    group.bench_function("srflx_candidate", |b| {
        b.iter(|| black_box(Candidate::new_server_reflexive(addr, addr, 1)));
    });

    group.finish();
}

fn bench_candidate_priority(c: &mut Criterion) {
    let mut group = c.benchmark_group("ice_candidate_priority");

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 7001);

    group.bench_function("calculate_priority", |b| {
        b.iter(|| black_box(Candidate::compute_priority(CandidateType::Host, 65535, 1)));
    });

    group.finish();
}

fn bench_candidate_sdp_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ice_sdp_generation");

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 7001);
    let candidate = Candidate::new_host(addr, 1);

    group.bench_function("to_sdp", |b| {
        b.iter(|| black_box(candidate.to_sdp()));
    });

    group.finish();
}

fn bench_ice_agent_candidate_gathering(c: &mut Criterion) {
    let mut group = c.benchmark_group("ice_agent");

    let config = IceConfig::default();

    group.bench_function("create_agent", |b| {
        b.iter(|| black_box(IceAgent::new(Some(config.clone()))));
    });

    group.finish();
}

fn bench_nat_type_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("nat_detection");

    use agora_core::nat::NatType;

    group.bench_function("nat_type_can_hole_punch", |b| {
        b.iter(|| black_box(NatType::FullCone.can_hole_punch()));
    });

    group.bench_function("nat_type_description", |b| {
        b.iter(|| black_box(NatType::Symmetric.description()));
    });

    group.finish();
}

fn bench_upnp_discovery(c: &mut Criterion) {
    let mut group = c.benchmark_group("upnp");

    use agora_core::upnp::UpnpConfig;

    group.bench_function("create_config", |b| {
        b.iter(|| black_box(UpnpConfig::default()));
    });

    group.finish();
}

fn bench_tcp_hole_punch_config(c: &mut Criterion) {
    let mut group = c.benchmark_group("tcp_hole_punch");

    use agora_core::tcp_punch::TcpHolePunchConfig;

    group.bench_function("create_config", |b| {
        b.iter(|| black_box(TcpHolePunchConfig::default()));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_candidate_creation,
    bench_candidate_priority,
    bench_candidate_sdp_generation,
    bench_ice_agent_candidate_gathering,
    bench_nat_type_detection,
    bench_upnp_discovery,
    bench_tcp_hole_punch_config,
);

criterion_main!(benches);

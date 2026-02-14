use lazy_static::lazy_static;
use prometheus::{
    register_counter, register_gauge, register_int_gauge, Counter, Encoder, Gauge, IntGauge,
    TextEncoder,
};
use std::time::Instant;

lazy_static! {
    pub static ref NODE_UPTIME_SECONDS: Gauge =
        register_gauge!("agora_node_uptime_seconds", "Node uptime in seconds").unwrap();
    pub static ref CONNECTIONS_TOTAL: IntGauge =
        register_int_gauge!("agora_connections_total", "Total number of P2P connections").unwrap();
    pub static ref CONNECTIONS_INCOMING: IntGauge = register_int_gauge!(
        "agora_connections_incoming",
        "Number of incoming connections"
    )
    .unwrap();
    pub static ref CONNECTIONS_OUTGOING: IntGauge = register_int_gauge!(
        "agora_connections_outgoing",
        "Number of outgoing connections"
    )
    .unwrap();
    pub static ref ROOMS_ACTIVE: IntGauge =
        register_int_gauge!("agora_rooms_active", "Number of active rooms being served").unwrap();
    pub static ref PARTICIPANTS_TOTAL: IntGauge = register_int_gauge!(
        "agora_participants_total",
        "Total number of participants across all rooms"
    )
    .unwrap();
    pub static ref AUDIO_PACKETS_IN: Counter = register_counter!(
        "agora_audio_packets_received_total",
        "Total audio packets received"
    )
    .unwrap();
    pub static ref AUDIO_PACKETS_OUT: Counter =
        register_counter!("agora_audio_packets_sent_total", "Total audio packets sent").unwrap();
    pub static ref AUDIO_BYTES_IN: Counter = register_counter!(
        "agora_audio_bytes_received_total",
        "Total audio bytes received"
    )
    .unwrap();
    pub static ref AUDIO_BYTES_OUT: Counter =
        register_counter!("agora_audio_bytes_sent_total", "Total audio bytes sent").unwrap();
    pub static ref BYTES_ENCODED: Counter =
        register_counter!("agora_bytes_encoded_total", "Total bytes encoded by Opus").unwrap();
    pub static ref BYTES_DECODED: Counter =
        register_counter!("agora_bytes_decoded_total", "Total bytes decoded by Opus").unwrap();
    pub static ref DHT_PEERS: IntGauge =
        register_int_gauge!("agora_dht_peers", "Number of peers in DHT routing table").unwrap();
    pub static ref NAT_TYPE: Gauge = register_gauge!(
        "agora_nat_type",
        "Detected NAT type (0=unknown, 1=public, 2=full_cone, 3=symmetric)"
    )
    .unwrap();
    pub static ref MIXER_ROLE: Gauge = register_gauge!(
        "agora_mixer_role",
        "Current mixer role (0=none, 1=participant, 2=mixer)"
    )
    .unwrap();
}

pub struct NodeMetrics {
    start_time: Instant,
}

impl NodeMetrics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    pub fn update_uptime(&self) {
        NODE_UPTIME_SECONDS.set(self.start_time.elapsed().as_secs_f64());
    }

    pub fn set_connections(&self, total: i64, incoming: i64, outgoing: i64) {
        CONNECTIONS_TOTAL.set(total);
        CONNECTIONS_INCOMING.set(incoming);
        CONNECTIONS_OUTGOING.set(outgoing);
    }

    pub fn set_rooms(&self, count: i64) {
        ROOMS_ACTIVE.set(count);
    }

    pub fn set_participants(&self, count: i64) {
        PARTICIPANTS_TOTAL.set(count);
    }

    pub fn inc_audio_packets_in(&self, bytes: usize) {
        AUDIO_PACKETS_IN.inc();
        AUDIO_BYTES_IN.inc_by(bytes as f64);
    }

    pub fn inc_audio_packets_out(&self, bytes: usize) {
        AUDIO_PACKETS_OUT.inc();
        AUDIO_BYTES_OUT.inc_by(bytes as f64);
    }

    pub fn set_dht_peers(&self, count: i64) {
        DHT_PEERS.set(count);
    }

    pub fn set_nat_type(&self, nat_type: u8) {
        NAT_TYPE.set(nat_type as f64);
    }

    pub fn set_mixer_role(&self, role: u8) {
        MIXER_ROLE.set(role as f64);
    }

    pub fn gather() -> Vec<u8> {
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        buffer
    }
}

impl Default for NodeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

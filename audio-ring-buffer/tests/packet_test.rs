
use audio_ring_buffer::packet::{
    decode, decode_samples, encode, PacketHeader,
    FRAME_SAMPLES, PACKET_SIZE,
};

#[test]
fn packet_roundtrip() {
    let header = PacketHeader {
        flags: 0,
        sequence: 42,
        sample_index: 20160,
        timestamp_ns: 123456789,
    };

    let mut samples = [0.0_f32; FRAME_SAMPLES];

    for (i, sample) in samples.iter_mut().enumerate() {
        *sample = i as f32 / FRAME_SAMPLES as f32;
    }

    let encoded = encode(header, &samples);

    assert_eq!(encoded.len(), PACKET_SIZE);

    let decoded = decode(&encoded).unwrap();

    assert_eq!(decoded.header, header);

    let recovered = decode_samples(decoded.payload).unwrap();

    assert_eq!(recovered, samples);
}

#[test]
fn invalid_magic_is_rejected() {
    let header = PacketHeader {
        flags: 0,
        sequence: 0,
        sample_index: 0,
        timestamp_ns: 0,
    };

    let samples = [0.0_f32; FRAME_SAMPLES];
    let mut bytes = encode(header, &samples);

    bytes[0] = b'X';

    assert!(decode(&bytes).is_err());
}

#[test]
fn invalid_length_is_rejected() {
    assert!(decode(&[0; 10]).is_err());
}

#[test]
fn preserve_float_bits() {
    let header = PacketHeader {
        flags: 0,
        sequence: 1,
        sample_index: 0,
        timestamp_ns: 0,
    };

    let mut samples = [0.0_f32; FRAME_SAMPLES];
    samples[0] = -0.0;
    samples[1] = f32::INFINITY;
    samples[2] = f32::from_bits(0x7fc0_0001);

    let encoded = encode(header, &samples);
    let packet = decode(&encoded).unwrap();
    let restored = decode_samples(packet.payload).unwrap();

    for (a, b) in samples.iter().zip(restored.iter()) {
        assert_eq!(a.to_bits(), b.to_bits());
    }
}

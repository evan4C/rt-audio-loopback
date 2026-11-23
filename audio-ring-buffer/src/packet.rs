/// 1 frame lasts 10ms, i.e. 1 frame = 480 samples if sampling rate = 48kHz
pub const FRAME_SAMPLES: usize = 480;

/// self-defined header size for debugging and monitoring
/// 4(magic) + 1(version) + 1(flags) + 2(header length) + 8(sequence) + 8(sample index) + 8(timestamp) = 32
pub const HEADER_SIZE: usize = 32;

/// one sample is stored as one float32 number, i.e. 4 bytes
pub const PACKET_SIZE: usize = HEADER_SIZE + FRAME_SAMPLES * 4;

/// self-defined protocol name
const MAGIC: [u8; 4] = *b"RTAU";

/// self-defined protocol version
const VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketHeader {
    pub flags: u8,
    pub sequence: u64,
    pub sample_index: u64,
    pub timestamp_ns: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PacketError {
    InvalidLength,
    InvalidMagic,
    UnsupportedVersion,
    InvalidHeaderLength,
}

pub struct AudioPacket<'a> {
    pub header: PacketHeader,
    pub payload: &'a [u8],
}

/// compose a packet by concatenate header and samples
pub fn encode(
    header: PacketHeader,
    samples: &[f32; FRAME_SAMPLES],
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(PACKET_SIZE);

    packet.extend_from_slice(&MAGIC);
    packet.push(VERSION);
    packet.push(header.flags);

    packet.extend_from_slice(
        &(HEADER_SIZE as u16).to_le_bytes(),
    );
    packet.extend_from_slice(
        &header.sequence.to_le_bytes(),
    );
    packet.extend_from_slice(
        &header.sample_index.to_le_bytes(),
    );
    packet.extend_from_slice(
        &header.timestamp_ns.to_le_bytes(),
    );

    for sample in samples {
        packet.extend_from_slice(
            &sample.to_bits().to_le_bytes(),
        );
    }

    debug_assert_eq!(packet.len(), PACKET_SIZE);
    packet
}

/// check and decode headers 
pub fn decode(data: &[u8]) -> Result<AudioPacket<'_>, PacketError> {
    if data.len() != PACKET_SIZE {
        return Err(PacketError::InvalidLength);
    }

    if data[..4] != MAGIC {
        return Err(PacketError::InvalidMagic);
    }

    if data[4] != VERSION {
        return Err(PacketError::UnsupportedVersion);
    }

    let header_len =
        u16::from_le_bytes(data[6..8].try_into().unwrap());

    if header_len as usize != HEADER_SIZE {
        return Err(PacketError::InvalidHeaderLength);
    }

    let sequence =
        u64::from_le_bytes(data[8..16].try_into().unwrap());

    let sample_index =
        u64::from_le_bytes(data[16..24].try_into().unwrap());

    let timestamp_ns =
        u64::from_le_bytes(data[24..32].try_into().unwrap());

    Ok(AudioPacket {
        header: PacketHeader {
            flags: data[5],
            sequence,
            sample_index,
            timestamp_ns,
        },
        payload: &data[HEADER_SIZE..],
    })
}

/// decode data samples
pub fn decode_samples(
    payload: &[u8],
) -> Result<[f32; FRAME_SAMPLES], PacketError> {
    if payload.len() != FRAME_SAMPLES * 4 {
        return Err(PacketError::InvalidLength);
    }

    let mut samples = [0.0_f32; FRAME_SAMPLES];

    for (dst, bytes) in samples
        .iter_mut()
        .zip(payload.chunks_exact(4))
    {
        let bits = u32::from_le_bytes(
            bytes.try_into().unwrap(),
        );

        *dst = f32::from_bits(bits);
    }

    Ok(samples)
}

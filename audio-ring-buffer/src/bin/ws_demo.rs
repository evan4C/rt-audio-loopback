
use std::error::Error;

use futures_util::{SinkExt, StreamExt};
use tokio::time::{interval, Duration, MissedTickBehavior};
use tokio_tungstenite::{
    connect_async,
    tungstenite::Message,
};

use audio_ring_buffer::{
    ring_buffer,
    AudioFramer,
    metrics::LatencyClock,
    packet::{
        decode, decode_samples, encode,
        PacketHeader, FRAME_SAMPLES,
    },
};

const SERVER_URL: &str = "ws://127.0.0.1:8765";
const SAMPLE_RATE: usize = 48_000;
const TOTAL_FRAMES: u64 = 100;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Connect to the Echo Server.
    let (mut ws, _) = connect_async(SERVER_URL).await?;
    println!("Connected to {SERVER_URL}");

    // 2. Create the audio pipeline.
    let (mut producer, mut consumer) = ring_buffer(4096);
    let mut framer = AudioFramer::<FRAME_SAMPLES>::new();
    let clock = LatencyClock::new();

    let mut sequence = 0_u64;
    let mut sample_index = 0_u64;
    let mut rtt_sum_ms = 0.0;

    // Simulate a 10 ms audio capture period.
    let mut ticker = interval(Duration::from_millis(10));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    while sequence < TOTAL_FRAMES {
        ticker.tick().await;

        // 3. Simulate 480 captured samples.
        let mut captured = [0.0_f32; FRAME_SAMPLES];

        for (i, sample) in captured.iter_mut().enumerate() {
            let position = sample_index as usize + i;
            *sample = (position % SAMPLE_RATE) as f32
                / SAMPLE_RATE as f32;
        }

        // 4. Write variable-sized chunks into Ring Buffer.
        // 128 + 256 + 96 = 480 samples.
        let mut offset = 0;

        for chunk_size in [128, 256, 96] {
            let end = offset + chunk_size;
            let chunk = &captured[offset..end];

            let written = producer.push(chunk);

            assert_eq!(
                written,
                chunk.len(),
                "Ring Buffer overflow"
            );

            offset = end;
        }

        // 5. Read a complete 10 ms frame.
        let frame = framer
            .poll_frame(&mut consumer)
            .expect("Expected 480 samples");

        // 6. Encode the frame.
        let header = PacketHeader {
            flags: 0,
            sequence,
            sample_index,
            timestamp_ns: clock.timestamp_ns(),
        };

        let packet = encode(header, frame);

        // 7. Send the packet over WebSocket.
        ws.send(Message::Binary(packet.into())).await?;

        // 8. Wait for the Echo Server response.
        let response = ws
            .next()
            .await
            .ok_or("Server closed connection")??;

        let Message::Binary(bytes) = response else {
            return Err("Expected binary audio packet".into());
        };

        // 9. Decode the returned packet.
        let received = decode(&bytes)
            .expect("Invalid packet received");

        // 10. Verify metadata.
        assert_eq!(received.header.sequence, sequence);
        assert_eq!(received.header.sample_index, sample_index);

        // 11. Verify PCM contents.
        let restored = decode_samples(received.payload)
            .expect("Invalid PCM payload");

        for (a, b) in frame.iter().zip(restored.iter()) {
            assert_eq!(a.to_bits(), b.to_bits());
        }

        // 12. Measure RTT.
        let rtt = clock
            .rtt(received.header.timestamp_ns)
            .expect("Invalid timestamp");

        let rtt_ms = rtt.as_secs_f64() * 1000.0;
        rtt_sum_ms += rtt_ms;

        println!(
            "seq={:03} sample_index={:06} RTT={:.3} ms",
            sequence,
            sample_index,
            rtt_ms
        );

        sequence += 1;
        sample_index += FRAME_SAMPLES as u64;
    }

    // 13. Print summary and close normally.
    println!(
        "\nTransferred {} frames, average RTT {:.3} ms",
        sequence,
        rtt_sum_ms / sequence as f64
    );

    ws.close(None).await?;

    Ok(())
}

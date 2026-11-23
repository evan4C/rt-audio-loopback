#![forbid(unsafe_code)]

pub mod packet;
pub mod metrics;

use std::sync::{
    atomic::{AtomicU32, AtomicUsize, Ordering},
    Arc,
};

struct Inner {
    slots: Box<[AtomicU32]>,
    // next read position in range of len*2, real pos is head % len
    head: AtomicUsize,
    // next write position in range of len*2, real pos is tail % len
    tail: AtomicUsize,
}

impl Inner {
    // Cursors range over two laps, so equal slots can represent full(distance=len) or empty(distance=0).
    fn distance(&self, newer: usize, older: usize) -> usize {
        if newer >= older {
            newer - older
        } else {
            self.slots.len() * 2 - older + newer
        }
    }

    fn advance(&self, cursor: usize, count: usize) -> usize {
        let until_wrap = self.slots.len() * 2 - cursor;
        if count < until_wrap {
            cursor + count
        } else {
            count - until_wrap
        }
    }

    fn slots_from(&self, cursor: usize) -> impl Iterator<Item = &AtomicU32> {
        let (before, after) = self.slots.split_at(cursor % self.slots.len());
        after.iter().chain(before)
    }
}

pub struct Producer {
    inner: Arc<Inner>,
}

pub struct Consumer {
    inner: Arc<Inner>,
}

/// Creates one producer and one consumer for a fixed number of mono samples.
pub fn ring_buffer(capacity: usize) -> (Producer, Consumer) {
    assert!(capacity > 0 && capacity <= isize::MAX as usize);

    let inner = Arc::new(Inner {
        slots: (0..capacity).map(|_| AtomicU32::new(0)).collect(),
        head: AtomicUsize::new(0),
        tail: AtomicUsize::new(0),
    });

    (
        Producer { inner: Arc::clone(&inner) },
        Consumer { inner },
    )
}

impl Producer {
    /// Writes the prefix that fits and returns its length, without allocating.
    pub fn push(&mut self, samples: &[f32]) -> usize {
        let tail = self.inner.tail.load(Ordering::Relaxed);
        let head = self.inner.head.load(Ordering::Acquire);
        let free = self.inner.slots.len() - self.inner.distance(tail, head);
        let count = samples.len().min(free);

        for (sample, slot) in samples[..count].iter().zip(self.inner.slots_from(tail)) {
            slot.store(sample.to_bits(), Ordering::Relaxed);
        }

        self.inner.tail.store(self.inner.advance(tail, count), Ordering::Release);
        count
    }
}

impl Consumer {
    /// Reads the available prefix into caller-provided storage.
    pub fn pop(&mut self, output: &mut [f32]) -> usize {
        let head = self.inner.head.load(Ordering::Relaxed);
        let tail = self.inner.tail.load(Ordering::Acquire);
        let count = output.len().min(self.inner.distance(tail, head));

        for (sample, slot) in output[..count].iter_mut().zip(self.inner.slots_from(head)) {
            *sample = f32::from_bits(slot.load(Ordering::Relaxed));
        }

        self.inner.head.store(self.inner.advance(head, count), Ordering::Release);
        count
    }
}

/// Assembles fixed-size mono PCM frames from a single consumer.
pub struct AudioFramer<const N: usize> {
    samples: [f32; N],
    filled: usize,
}

impl<const N: usize> AudioFramer<N> {
    pub fn new() -> Self {
        assert!(N > 0, "audio frames must contain at least one sample");
        Self { samples: [0.0; N], filled: 0 }
    }

    /// Polls for one frame from the ring buffer. An incomplete frame is retained for the next call.
    /// The returned frame borrows the framer's storage until the next poll.
    pub fn poll_frame(&mut self, consumer: &mut Consumer) -> Option<&[f32; N]> {
        self.filled += consumer.pop(&mut self.samples[self.filled..]);
        if self.filled == N {
            self.filled = 0;
            Some(&self.samples)
        } else {
            None
        }
    }

    /// Return how many samples are polled from the ring buffer
    pub fn pending_samples(&self) -> usize {
        self.filled
    }
}

impl<const N: usize> Default for AudioFramer<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_power_of_two_cursor_wraps_while_full() {
        let (mut producer, mut consumer) = ring_buffer(3);
        assert_eq!(producer.push(&[0.0, 1.0, 2.0, 99.0]), 3);
        assert_eq!(producer.push(&[99.0]), 0);
        let mut prefix = [0.0; 2];
        assert_eq!(consumer.pop(&mut prefix), 2);
        assert_eq!(prefix, [0.0, 1.0]);
        assert_eq!(producer.push(&[3.0, 4.0]), 2);
        assert_eq!(consumer.pop(&mut prefix), 2);
        assert_eq!(prefix, [2.0, 3.0]);
        assert_eq!(producer.push(&[5.0, 6.0]), 2);
        let mut rest = [0.0; 3];
        assert_eq!(consumer.pop(&mut rest), 3);
        assert_eq!(rest, [4.0, 5.0, 6.0]);
        assert_eq!(consumer.pop(&mut rest), 0);
    }

    #[test]
    fn variable_chunks_keep_the_partial_frame() {
        let (mut producer, mut consumer) = ring_buffer(600);
        let mut framer = AudioFramer::<480>::new();
        let input: Vec<f32> = (0..1440).map(|sample| sample as f32).collect();
        let mut written = 0;
        let mut emitted = 0;

        for size in [128, 256, 128, 512, 256] {
            assert_eq!(producer.push(&input[written..written + size]), size);
            written += size;
            while let Some(frame) = framer.poll_frame(&mut consumer) {
                assert_eq!(&frame[..], &input[emitted..emitted + 480]);
                emitted += 480;
            }
        }

        assert_eq!(emitted, 960);
        assert_eq!(framer.pending_samples(), 320);
        assert!(framer.poll_frame(&mut consumer).is_none());
        assert_eq!(framer.pending_samples(), 320);
        assert_eq!(producer.push(&input[written..]), 160);
        assert_eq!(&framer.poll_frame(&mut consumer).unwrap()[..], &input[960..]);
        assert_eq!(framer.pending_samples(), 0);
        assert!(framer.poll_frame(&mut consumer).is_none());
    }

    #[test]
    fn concurrent_transfer_preserves_order_through_many_wraps() {
        let (mut producer, mut consumer) = ring_buffer(31);
        let writer = std::thread::spawn(move || {
            for i in 0..10_000 {
                while producer.push(&[i as f32]) == 0 {
                    std::thread::yield_now();
                }
            }
        });

        let mut output = [0.0];
        let mut first_mismatch = None;
        for i in 0..10_000 {
            while consumer.pop(&mut output) == 0 {
                std::thread::yield_now();
            }
            if output[0] != i as f32 {
                first_mismatch.get_or_insert((i, output[0]));
            }
        }
        writer.join().unwrap();
        assert_eq!(first_mismatch, None);
    }
}

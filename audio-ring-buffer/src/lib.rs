
use std::sync::{
    atomic::{AtomicU32, AtomicUsize, Ordering},
    Arc,
};

struct Inner {
    buffer: Box<[AtomicU32]>,
    capacity: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
}

pub struct Producer {
    inner: Arc<Inner>,
}

pub struct Consumer {
    inner: Arc<Inner>,
}

pub fn ring_buffer(capacity: usize) -> (Producer, Consumer) {
    assert!(capacity > 0);
    assert!(capacity <= isize::MAX as usize);

    let buffer = (0..capacity)
        .map(|_| AtomicU32::new(0))
        .collect::<Vec<_>>()
        .into_boxed_slice();

    let inner = Arc::new(Inner {
        buffer,
        capacity,
        head: AtomicUsize::new(0),
        tail: AtomicUsize::new(0),
    });

    (
        Producer {
            inner: Arc::clone(&inner),
        },
        Consumer { inner },
    )
}

impl Producer {
    /// Returns the number of samples written.
    /// Never blocks and never allocates.
    pub fn push(&mut self, samples: &[f32]) -> usize {
        let tail = self.inner.tail.load(Ordering::Relaxed);
        let head = self.inner.head.load(Ordering::Acquire);

        let used = tail.wrapping_sub(head);
        let free = self.inner.capacity - used;
        let count = samples.len().min(free);

        for (i, sample) in samples[..count].iter().enumerate() {
            let index = tail.wrapping_add(i) % self.inner.capacity;

            self.inner.buffer[index].store(
                sample.to_bits(),
                Ordering::Relaxed,
            );
        }

        self.inner.tail.store(
            tail.wrapping_add(count),
            Ordering::Release,
        );

        count
    }
}

impl Consumer {
    /// Returns the number of samples read.
    /// The caller provides the output memory.
    pub fn pop(&mut self, output: &mut [f32]) -> usize {
        let head = self.inner.head.load(Ordering::Relaxed);
        let tail = self.inner.tail.load(Ordering::Acquire);

        let available = tail.wrapping_sub(head);
        let count = output.len().min(available);

        for (i, sample) in output[..count].iter_mut().enumerate() {
            let index = head.wrapping_add(i) % self.inner.capacity;

            let bits = self.inner.buffer[index]
                .load(Ordering::Relaxed);

            *sample = f32::from_bits(bits);
        }

        self.inner.head.store(
            head.wrapping_add(count),
            Ordering::Release,
        );

        count
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_push_pop() {
        let (mut producer, mut consumer) = ring_buffer(8);

        assert_eq!(producer.push(&[1.0, 2.0, 3.0]), 3);

        let mut output = [0.0; 3];

        assert_eq!(consumer.pop(&mut output), 3);
        assert_eq!(output, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn wraparound() {
        let (mut producer, mut consumer) = ring_buffer(5);

        assert_eq!(
            producer.push(&[1.0, 2.0, 3.0, 4.0]),
            4
        );

        let mut first = [0.0; 3];
        assert_eq!(consumer.pop(&mut first), 3);

        assert_eq!(
            producer.push(&[5.0, 6.0, 7.0]),
            3
        );

        let mut second = [0.0; 4];
        assert_eq!(consumer.pop(&mut second), 4);
        assert_eq!(second, [4.0, 5.0, 6.0, 7.0]);
    }

    #[test]
    fn buffer_full() {
        let (mut producer, mut consumer) = ring_buffer(3);

        assert_eq!(producer.push(&[1.0, 2.0, 3.0]), 3);
        assert_eq!(producer.push(&[4.0]), 0);

        let mut output = [0.0; 3];
        assert_eq!(consumer.pop(&mut output), 3);
        assert_eq!(output, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn concurrent_transfer() {
        let (mut producer, mut consumer) = ring_buffer(1024);

        let writer = std::thread::spawn(move || {
            for i in 0..100_000 {
                let sample = [i as f32];

                while producer.push(&sample) == 0 {
                    std::thread::yield_now();
                }
            }
        });

        let reader = std::thread::spawn(move || {
            for i in 0..100_000 {
                let mut output = [0.0];

                while consumer.pop(&mut output) == 0 {
                    std::thread::yield_now();
                }

                assert_eq!(output[0], i as f32);
            }
        });

        writer.join().unwrap();
        reader.join().unwrap();
    }
}


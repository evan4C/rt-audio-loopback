
import numpy as np


class AudioRingBuffer:
    """Bounded mono Float32 PCM ring buffer.

    Prototype implementation; not thread-safe.
    """

    def __init__(self, capacity: int):
        if capacity <= 0:
            raise ValueError("capacity must be positive")

        self.capacity = capacity
        self.buffer = np.zeros(capacity, dtype=np.float32)

        self.read_pos = 0
        self.write_pos = 0
        self.size = 0

    @property
    def available(self) -> int:
        return self.size

    @property
    def free(self) -> int:
        return self.capacity - self.size

    def write(self, samples: np.ndarray) -> bool:
        """Write all samples or reject the entire block."""

        samples = np.asarray(samples, dtype=np.float32).reshape(-1)
        n = len(samples)

        if n > self.free:
            return False

        first = min(n, self.capacity - self.write_pos)

        self.buffer[self.write_pos:self.write_pos + first] = (
            samples[:first]
        )

        remaining = n - first

        if remaining:
            self.buffer[:remaining] = samples[first:]

        self.write_pos = (self.write_pos + n) % self.capacity
        self.size += n

        return True

    def read(self, n: int) -> np.ndarray | None:
        """Read exactly n samples, or return None."""

        if n < 0:
            raise ValueError("n must be non-negative")

        if n > self.available:
            return None

        result = np.empty(n, dtype=np.float32)

        first = min(n, self.capacity - self.read_pos)

        result[:first] = self.buffer[
            self.read_pos:self.read_pos + first
        ]

        remaining = n - first

        if remaining:
            result[first:] = self.buffer[:remaining]

        self.read_pos = (self.read_pos + n) % self.capacity
        self.size -= n

        return result

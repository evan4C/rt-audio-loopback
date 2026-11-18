
import numpy as np

from rt_audio_loopback.ring_buffer import AudioRingBuffer


def test_write_read():
    rb = AudioRingBuffer(capacity=10)

    assert rb.write(np.array([1, 2, 3], dtype=np.float32))
    assert rb.available == 3

    result = rb.read(3)

    np.testing.assert_array_equal(
        result,
        np.array([1, 2, 3], dtype=np.float32),
    )

    assert rb.available == 0


def test_wraparound():
    rb = AudioRingBuffer(capacity=5)

    assert rb.write(np.array([1, 2, 3, 4], dtype=np.float32))

    np.testing.assert_array_equal(
        rb.read(3),
        [1, 2, 3],
    )

    assert rb.write(np.array([5, 6, 7], dtype=np.float32))

    np.testing.assert_array_equal(
        rb.read(4),
        [4, 5, 6, 7],
    )


def test_overflow():
    rb = AudioRingBuffer(capacity=4)

    assert rb.write(np.array([1, 2, 3, 4], dtype=np.float32))
    assert not rb.write(np.array([5], dtype=np.float32))

    assert rb.available == 4


def test_audio_framing():
    rb = AudioRingBuffer(capacity=2000)

    chunks = [
        np.arange(0, 128, dtype=np.float32),
        np.arange(128, 384, dtype=np.float32),
        np.arange(384, 512, dtype=np.float32),
        np.arange(512, 1024, dtype=np.float32),
        np.arange(1024, 1280, dtype=np.float32),
    ]

    frames = []

    for chunk in chunks:
        assert rb.write(chunk)

        while rb.available >= 480:
            frames.append(rb.read(480))

    assert len(frames) == 2
    assert rb.available == 320

    np.testing.assert_array_equal(
        np.concatenate(frames),
        np.arange(960, dtype=np.float32),
    )

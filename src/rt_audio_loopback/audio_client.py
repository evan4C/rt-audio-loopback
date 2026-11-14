
import asyncio
import queue
import threading

import numpy as np
import sounddevice as sd
import websockets


SAMPLE_RATE = 48000
CHANNELS = 1
BLOCK_SIZE = 0

# 约 100 ms 的单声道 Float32 PCM 数据
MAX_BUFFER_SAMPLES = SAMPLE_RATE // 10

capture_queue = queue.Queue(maxsize=64)
playback_queue = queue.Queue(maxsize=64)

stop_event = threading.Event()

stats = {
    "capture_drops": 0,
    "playback_drops": 0,
    "underflows": 0,
}


def audio_callback(indata, outdata, frames, time_info, status):
    if status:
        print("Audio status:", status)

    # capture microphone data into capture queue
    try:
        capture_queue.put_nowait(indata.copy().tobytes())
    except queue.Full:
        stats["capture_drops"] += 1

    # play microphone data from playback queue received from server
    try:
        pcm_bytes = playback_queue.get_nowait()
        samples = np.frombuffer(pcm_bytes, dtype=np.float32)

        outdata.fill(0)
        count = min(frames, len(samples))
        outdata[:count, 0] = samples[:count]

    except queue.Empty:
        outdata.fill(0)
        stats["underflows"] += 1


# send captured microphone data to server
async def send_audio(ws):
    while True:
        try:
            pcm = await asyncio.to_thread(capture_queue.get, True, 0.1)
        except queue.Empty:
            continue
        await ws.send(pcm)

# receive microphone data from server and save to playback queue
async def receive_audio(ws):
    async for message in ws:
        if not isinstance(message, bytes):
            continue

        try:
            playback_queue.put_nowait(message)
        except queue.Full:
            stats["playback_drops"] += 1

# print status data for debugging
async def print_stats():
    while True:
        await asyncio.sleep(2)

        print(
            f"Capture drops: {stats['capture_drops']} | "
            f"Playback drops: {stats['playback_drops']} | "
            f"Underflows: {stats['underflows']}"
        )


async def main():
    async with websockets.connect("ws://127.0.0.1:8765", max_size=None) as ws:

        print("Connected to WebSocket server")

        with sd.Stream(
            samplerate=SAMPLE_RATE,
            blocksize=BLOCK_SIZE,
            channels=CHANNELS,
            dtype="float32",
            latency="low",
            callback=audio_callback,
        ):
            print("Audio streaming started")
            print("Press Ctrl+C to stop")

            tasks = [
                asyncio.create_task(send_audio(ws)),
                asyncio.create_task(receive_audio(ws)),
                asyncio.create_task(print_stats()),
            ]

            try:
                await asyncio.gather(*tasks)
            finally:
                for task in tasks:
                    task.cancel()

                await asyncio.gather(
                    *tasks,
                    return_exceptions=True,
                )


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nStopped")

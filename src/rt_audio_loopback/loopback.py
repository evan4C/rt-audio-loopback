
import argparse
import time

import sounddevice as sd
import click

def flist_devices():
    """List all available audio devices."""


@click.command()
@click.option("--list-devices", "-l", is_flag=True)
@click.option("--input", "-i", type=int, default=None)
@click.option("--output", "-o", type=int, default=None)
@click.option("--rate", "-r", type=int, default=48000)
@click.option("--blocksize", "-b", type=int, default=480)
def run(list_devices: bool, input: int, output: int, rate: int, blocksize: int):

    devices = sd.query_devices()
    # Step 1: List audio devices
    if list_devices:
        print("\nAvailable audio devices:\n")

        for index, device in enumerate(devices):
            print(f"[{index}] {device['name']}")
            print(f"    Inputs:  {device['max_input_channels']}")
            print(f"    Outputs: {device['max_output_channels']}")
            print(f"    Default sample rate: {device['default_samplerate']} Hz")
            print()

    # Step 2: Select input and output devices
    default_input, default_output = sd.default.device

    input_device = input if input is not None else default_input
    output_device = output if output is not None else default_output

    input_info = sd.query_devices(input_device)
    output_info = sd.query_devices(output_device)

    if input_info["max_input_channels"] < 1:
        raise RuntimeError("Input device has no microphone channel.")

    if output_info["max_output_channels"] < 1:
        raise RuntimeError("Output device has no playback channel.")

    output_channels = min(2, int(output_info["max_output_channels"]))

    print("\nStarting audio loopback...")
    print(f"Input:  {input_info['name']}")
    print(f"Output: {output_info['name']}")
    print(f"Sample rate: {rate} Hz")
    print(f"Block size: {blocksize} frames")
    print(f"Block duration: {blocksize / rate * 1000:.2f} ms")

    # Step 3: Audio statistics
    callback_count = 0
    overflow_count = 0
    underflow_count = 0

    def audio_callback(indata, outdata, frames, time_info, status):
        nonlocal callback_count
        nonlocal overflow_count
        nonlocal underflow_count

        callback_count += 1

        if status:
            if status.input_overflow:
                overflow_count += 1

            if status.output_underflow:
                underflow_count += 1

        # Copy mono microphone audio to headphone output.
        # Broadcasting duplicates mono audio into stereo channels.
        outdata[:] = indata[:, :1]

    # Step 4: Start real-time audio stream
    try:
        with sd.Stream(
            device=(input_device, output_device),
            samplerate=rate,
            blocksize=blocksize,
            channels=(1, output_channels),
            dtype="float32",
            latency="low",
            callback=audio_callback,
        ) as stream:

            print("\nAudio loopback is running!")
            print("Speak into your microphone.")
            print("Press Ctrl+C to stop.\n")

            print(f"Input latency:  {stream.latency[0] * 1000:.2f} ms")
            print(f"Output latency: {stream.latency[1] * 1000:.2f} ms")

            while True:
                time.sleep(1)
                print(f"\rCallbacks: {callback_count:6d} | Input overflows: {overflow_count:4d} | Output underflows: {underflow_count:4d}", end="", flush=True)

    except KeyboardInterrupt:
        print("\n\nAudio loopback stopped.")

    except sd.PortAudioError as error:
        print(f"\nAudio error: {error}")
        raise


if __name__ == "__main__":
    run()

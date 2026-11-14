import asyncio
import json

from websockets.asyncio.client import connect


async def main() -> None:
    async with connect("ws://127.0.0.1:8765") as ws:
        event = {"type": "greetings", "text": "hello"}
        await ws.send(json.dumps(event, ensure_ascii=False))

        reply = await asyncio.wait_for(ws.recv(), timeout=3)
        print("JSON:", json.loads(reply))

        await ws.send(b"\x01\x02\x03")
        print("binary:", await ws.recv())


if __name__ == "__main__":
    asyncio.run(main())
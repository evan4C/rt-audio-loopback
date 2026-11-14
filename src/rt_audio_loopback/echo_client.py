import asyncio
import json

from websockets.asyncio.client import connect

async def send_messages(ws, max_msg):
    sequence = 0
    while sequence <= max_msg:
        await ws.send(json.dumps({"type": "tick", "sequence": sequence}))
        sequence += 1
        await asyncio.sleep(1)


async def receive_messages(ws):
    async for message in ws:
        print("server reply:", message)


async def main():
    async with connect("ws://127.0.0.1:8765") as ws:
        tasks = {
            asyncio.create_task(send_messages(ws, 30)),
            asyncio.create_task(receive_messages(ws)),
        }

        try:
            done, _ = await asyncio.wait(
                tasks, return_when=asyncio.FIRST_COMPLETED
            )
            for task in done:
                task.result()
        finally:
            await ws.close()
            for task in tasks:
                task.cancel()
            await asyncio.gather(*tasks, return_exceptions=True)


if __name__ == "__main__":
    asyncio.run(main())
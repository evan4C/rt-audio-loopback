
import asyncio
from websockets.asyncio.server import ServerConnection, serve


async def echo(ws: ServerConnection):
    print("Client connected")

    try:
        async for message in ws:
            print(f"Received: {message!r}")
            await ws.send(message)
    finally:
        print("Client disconnected")


async def main() -> None:
    async with serve(echo, "127.0.0.1", 8765, max_size=None) as server:
        print("Echo server running on ws://127.0.0.1:8765")
        await server.serve_forever()


if __name__ == "__main__":
    asyncio.run(main())

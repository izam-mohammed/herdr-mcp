import asyncio, os
from mcp.server.mcpserver import MCPServer as FastMCP

mcp = FastMCP("bench")

@mcp.tool(description="ping herdr")
async def ping() -> str:
    r, w = await asyncio.open_unix_connection(os.environ["HERDR_SOCKET_PATH"])
    w.write(b'{"id":"1","method":"ping","params":{}}\n'); await w.drain()
    line = await r.readline(); w.close()
    return line.decode().rstrip()

mcp.run()

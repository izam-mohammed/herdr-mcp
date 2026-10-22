import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import net from "node:net";

const ping = () => new Promise((resolve, reject) => {
  const c = net.createConnection(process.env.HERDR_SOCKET_PATH);
  let buf = "";
  c.on("data", (d) => { buf += d; const i = buf.indexOf("\n"); if (i >= 0) { c.end(); resolve(buf.slice(0, i)); } });
  c.on("error", reject);
  c.write('{"id":"1","method":"ping","params":{}}\n');
});
const server = new McpServer({ name: "bench", version: "0" });
server.registerTool("ping", { description: "ping herdr" }, async () => ({ content: [{ type: "text", text: await ping() }] }));
await server.connect(new StdioServerTransport());

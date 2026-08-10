#!/usr/bin/env node
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import fs from "fs";
import os from "os";
import path from "path";
import { spawn } from "child_process";
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const tokenFile = path.join(os.homedir(), '.glyph', 'mcp_token');

const getApiToken = () => {
  try {
    return fs.readFileSync(tokenFile, 'utf8').trim();
  } catch (err) {
    throw new Error("Glyph MCP Token not found. Is Glyph running?");
  }
};

const GLYPH_API = 'http://127.0.0.1:15354';

async function glyphFetch(endpoint, method = 'GET', body = null) {
  const fetchAttempt = async () => {
    const options = {
      method,
      headers: {
        'Authorization': `Bearer ${getApiToken()}`,
        'Content-Type': 'application/json'
      }
    };
    if (body) options.body = JSON.stringify(body);
    return await fetch(`${GLYPH_API}${endpoint}`, options);
  };

  let res;
  try {
    res = await fetchAttempt();
  } catch (err) {
    if (err.cause?.code === 'ECONNREFUSED' || err.code === 'ECONNREFUSED' || err.message.includes('ECONNREFUSED') || err.message.includes('fetch failed')) {
      const isAsar = __dirname.includes('app.asar');
      let child;
      if (isAsar) {
        let exePath = path.join(__dirname.split('resources')[0], 'Glyph.exe');
        child = spawn(exePath, [], { detached: true, stdio: 'ignore', shell: true });
      } else {
        const rootDir = path.resolve(__dirname, '../../');
        const npmCmd = process.platform === 'win32' ? 'npm.cmd' : 'npm';
        child = spawn(npmCmd, ['run', 'dev'], { cwd: rootDir, detached: true, stdio: 'ignore', shell: true });
      }
      child.unref();

      let attempts = 0;
      while (attempts < 20) {
        await new Promise(r => setTimeout(r, 500));
        try {
          res = await fetchAttempt();
          break;
        } catch (e) {
          attempts++;
        }
      }
      if (!res) throw new Error("Could not start Glyph automatically. Please launch it manually.");
    } else {
      throw err;
    }
  }

  const data = await res.json();
  if (!res.ok) throw new Error(data.error || 'API Error');
  return data;
}

const server = new Server(
  { name: "glyph-mcp", version: "1.0.0" },
  { capabilities: { tools: {} } }
);

server.setRequestHandler(ListToolsRequestSchema, async () => {
  return {
    tools: [
      {
        name: "glyph_list_servers",
        description: "List all saved servers in Glyph Vault.",
        inputSchema: { type: "object", properties: {} }
      },
      {
        name: "glyph_connect_server",
        description: "Connect to a saved server and open its window in Glyph.",
        inputSchema: {
          type: "object",
          properties: {
            serverId: { type: "string" }
          },
          required: ["serverId"]
        }
      },
      {
        name: "glyph_execute_command",
        description: "Execute a shell command visually on a connected server.",
        inputSchema: {
          type: "object",
          properties: {
            serverId: { type: "string" },
            command: { type: "string" }
          },
          required: ["serverId", "command"]
        }
      },
      {
        name: "glyph_read_file",
        description: "Read a remote file via SFTP.",
        inputSchema: {
          type: "object",
          properties: {
            serverId: { type: "string" },
            path: { type: "string" }
          },
          required: ["serverId", "path"]
        }
      },
      {
        name: "glyph_write_file",
        description: "Write a remote file via SFTP.",
        inputSchema: {
          type: "object",
          properties: {
            serverId: { type: "string" },
            path: { type: "string" },
            content: { type: "string" }
          },
          required: ["serverId", "path", "content"]
        }
      },
      {
        name: "glyph_start_tunnel",
        description: "Start a local port forwarding tunnel via a connected server.",
        inputSchema: {
          type: "object",
          properties: {
            serverId: { type: "string" },
            localPort: { type: "number" },
            remoteHost: { type: "string" },
            remotePort: { type: "number" },
            protocol: { type: "string", enum: ["tcp", "udp"] }
          },
          required: ["serverId", "localPort", "remoteHost", "remotePort"]
        }
      },
      {
        name: "glyph_stop_tunnel",
        description: "Stop an active port forwarding tunnel.",
        inputSchema: {
          type: "object",
          properties: {
            serverId: { type: "string" },
            localPort: { type: "number" }
          },
          required: ["serverId", "localPort"]
        }
      },
      {
        name: "glyph_list_secrets",
        description: "List secrets in the server's vault.",
        inputSchema: {
          type: "object",
          properties: { serverId: { type: "string" } },
          required: ["serverId"]
        }
      },
      {
        name: "glyph_add_secret",
        description: "Add a secret to the server's vault.",
        inputSchema: {
          type: "object",
          properties: {
            serverId: { type: "string" },
            name: { type: "string" },
            value: { type: "string" }
          },
          required: ["serverId", "name", "value"]
        }
      },
      {
        name: "glyph_list_containers",
        description: "List Docker containers on the server.",
        inputSchema: {
          type: "object",
          properties: { serverId: { type: "string" } },
          required: ["serverId"]
        }
      },
      {
        name: "glyph_list_commands",
        description: "List saved snippet commands for the server.",
        inputSchema: {
          type: "object",
          properties: { serverId: { type: "string" } },
          required: ["serverId"]
        }
      },
      {
        name: "glyph_add_command",
        description: "Add a snippet command for the server.",
        inputSchema: {
          type: "object",
          properties: {
            serverId: { type: "string" },
            name: { type: "string" },
            cmd: { type: "string" }
          },
          required: ["serverId", "name", "cmd"]
        }
      },
      {
        name: "glyph_remove_command",
        description: "Remove a snippet command for the server by its ID.",
        inputSchema: {
          type: "object",
          properties: {
            serverId: { type: "string" },
            id: { type: "number" }
          },
          required: ["serverId", "id"]
        }
      }
    ]
  };
});

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  try {
    const { name, arguments: args } = request.params;

    if (name === "glyph_list_servers") {
      const servers = await glyphFetch('/api/servers');
      return { content: [{ type: "text", text: JSON.stringify(servers, null, 2) }] };
    }

    if (name === "glyph_connect_server") {
      const res = await glyphFetch('/api/connect', 'POST', args);
      return { content: [{ type: "text", text: "Connected successfully." }] };
    }

    if (name === "glyph_execute_command") {
      const res = await glyphFetch('/api/execute', 'POST', args);
      return { content: [{ type: "text", text: res.output }] };
    }

    if (name === "glyph_read_file") {
      const res = await glyphFetch('/api/sftp/read', 'POST', args);
      return { content: [{ type: "text", text: res.content }] };
    }

    if (name === "glyph_write_file") {
      await glyphFetch('/api/sftp/write', 'POST', args);
      return { content: [{ type: "text", text: "File written successfully." }] };
    }

    if (name === "glyph_start_tunnel") {
      const res = await glyphFetch('/api/tunnels/start', 'POST', args);
      return { content: [{ type: "text", text: `Tunnel started: Local port ${res.localPort} -> ${res.remoteHost}:${res.remotePort} (${res.protocol})` }] };
    }

    if (name === "glyph_stop_tunnel") {
      await glyphFetch('/api/tunnels/stop', 'POST', args);
      return { content: [{ type: "text", text: `Tunnel stopped on local port ${args.localPort}` }] };
    }

    if (name === "glyph_list_secrets") {
      const secrets = await glyphFetch('/api/secrets/list', 'POST', args);
      return { content: [{ type: "text", text: JSON.stringify(secrets, null, 2) }] };
    }

    if (name === "glyph_add_secret") {
      await glyphFetch('/api/secrets/add', 'POST', args);
      return { content: [{ type: "text", text: "Secret added successfully." }] };
    }

    if (name === "glyph_list_containers") {
      const res = await glyphFetch('/api/containers/list', 'POST', args);
      return { content: [{ type: "text", text: res.output }] };
    }

    if (name === "glyph_list_commands") {
      const res = await glyphFetch('/api/commands/list', 'POST', args);
      return { content: [{ type: "text", text: JSON.stringify(res.commands, null, 2) }] };
    }

    if (name === "glyph_add_command") {
      await glyphFetch('/api/commands/add', 'POST', args);
      return { content: [{ type: "text", text: "Command added successfully." }] };
    }

    if (name === "glyph_remove_command") {
      await glyphFetch('/api/commands/remove', 'POST', args);
      return { content: [{ type: "text", text: "Command removed successfully." }] };
    }

    throw new Error("Unknown tool");
  } catch (error) {
    return {
      content: [{ type: "text", text: `Error: ${error.message}` }],
      isError: true
    };
  }
});

async function run() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("Glyph MCP server running on stdio");
}

run().catch(console.error);

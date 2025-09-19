#!/usr/bin/env node

/*
server.js - <short description TODO>
Copyright (C) 2022 Strudel contributors - see <https://codeberg.org/uzu/strudel/src/branch/main/packages/osc/server.js>
This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details. You should have received a copy of the GNU Affero General Public License along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

import OSC from 'osc-js';

const args = process.argv.slice(2);
function getArgValue(flag) {
  const i = args.indexOf(flag);
  if (i !== -1) {
    const nextIsFlag = args[i + 1]?.startsWith('--') ?? true;
    if (nextIsFlag) return true;
    return args[i + 1];
  }
}

let udpClientPort = Number(getArgValue('--port')) || 57120;
let debug = Number(getArgValue('--debug')) || 0;

const config = {
  receiver: 'ws', // @param {string} Where messages sent via 'send' method will be delivered to, 'ws' for Websocket clients, 'udp' for udp client
  udpServer: {
    host: 'localhost', // @param {string} Hostname of udp server to bind to
    port: 57121, // @param {number} Port of udp client for messaging
    // enabling the following line will receive tidal messages:
    // port: 57120, // @param {number} Port of udp client for messaging
    exclusive: false, // @param {boolean} Exclusive flag
  },
  udpClient: {
    host: 'localhost', // @param {string} Hostname of udp client for messaging
    port: udpClientPort, // @param {number} Port of udp client for messaging
  },
  wsServer: {
    host: 'localhost', // @param {string} Hostname of WebSocket server
    port: 8080, // @param {number} Port of WebSocket server
  },
};

const osc = new OSC({ plugin: new OSC.BridgePlugin(config) });

if (debug) {
  osc.on('*', (message) => {
    const { address, args } = message;
    let str = '';
    for (let i = 0; i < args.length; i += 2) {
      str += `${args[i]}: ${args[i + 1]} `;
    }
    console.log(`${address} ${str}`);
  });
}

osc.on('error', (message) => {
  if (message.toString().includes('EADDRINUSE')) {
    console.log(`------ ERROR -------
a server is already running on port 57121! to stop it:
1. run "lsof -ti :57121 | xargs kill -9" (macos / linux)
2. re-run the osc server
`);
  } else {
    console.log(message);
  }
});

osc.open();

console.log('osc client running on port', config.udpClient.port);
console.log('osc server running on port', config.udpServer.port);
console.log('websocket server running on port', config.wsServer.port);
if (debug) {
  console.log('debug logs enabled. incoming messages will appear below');
}

#!/usr/bin/env node

const { startServer } = require('@apexjs/core');
const path = require('path');

const args = process.argv.slice(2);
let root = '.';
let port = 3000;

// Simple arg parsing
if (args.length > 0) {
    if (args.includes('--port')) {
        const idx = args.indexOf('--port');
        if (idx !== -1 && args[idx + 1]) {
            port = parseInt(args[idx + 1]);
        }
    }
    
    // Assume last arg is directory if not a flag, or explicitly named
    // For MVP, if arg doesn't start with -, treat as root
    const rootArg = args.find(a => !a.startsWith('-') && args[args.indexOf(a)-1] !== '--port');
    if (rootArg) {
        root = rootArg;
    }
}

// Convert to absolute path
const absRoot = path.resolve(process.cwd(), root);

console.log(`Starting Nexus on port ${port} serving ${absRoot}`);
startServer(absRoot, port);

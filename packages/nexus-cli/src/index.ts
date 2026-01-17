#!/usr/bin/env node

import { Command } from 'commander';
import { startDevServer } from './commands/dev';
import { buildProduction } from './commands/build';
import pc from 'picocolors';

const program = new Command();

program
    .name('nexus')
    .description('Next-generation build tool for scale')
    .version('0.1.0');

program
    .command('dev')
    .description('Start development server')
    .option('-p, --port <port>', 'Port to run server on', '3000')
    .option('-c, --config <path>', 'Path to config file', 'nexus.config.ts')
    .action(async (options) => {
        console.log(pc.cyan('\n🚀 Nexus Dev Server\n'));
        await startDevServer(options);
    });

program
    .command('build')
    .description('Build for production')
    .option('-c, --config <path>', 'Path to config file', 'nexus.config.ts')
    .action(async (options) => {
        console.log(pc.cyan('\n📦 Building for production...\n'));
        await buildProduction(options);
    });

program
    .command('init')
    .description('Initialize a new Nexus project')
    .argument('[template]', 'Template to use (react, vanilla)', 'react')
    .action((template) => {
        console.log(pc.cyan(`\n✨ Initializing ${template} project...\n`));
        console.log(pc.dim('  [Not yet implemented - Issue #34]'));
    });

program.parse();

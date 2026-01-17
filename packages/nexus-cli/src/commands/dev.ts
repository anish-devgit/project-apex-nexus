import { spawn } from 'child_process';
import path from 'path';
import pc from 'picocolors';

interface DevOptions {
    port: string;
    config: string;
}

export async function startDevServer(options: DevOptions): Promise<void> {
    console.log(pc.dim(`  Port: ${options.port}`));
    console.log(pc.dim(`  Config: ${options.config}\n`));

    // For v0.1, we spawn the Rust binary
    // TODO: Build napi-rs bridge in Issue #37
    const kernelPath = path.resolve(__dirname, '../../../crates/kernel');

    console.log(pc.yellow('⚠️  Starting Rust kernel (requires cargo)...\n'));

    const child = spawn('cargo', ['run', '--release'], {
        cwd: kernelPath,
        stdio: 'inherit',
        env: {
            ...process.env,
            NEXUS_PORT: options.port,
            NEXUS_CONFIG: options.config,
        },
    });

    child.on('error', (err) => {
        console.error(pc.red('Failed to start Nexus kernel:'), err);
        process.exit(1);
    });

    child.on('exit', (code) => {
        if (code !== 0) {
            console.error(pc.red(`\nKernel exited with code ${code}`));
            process.exit(code || 1);
        }
    });

    // Handle graceful shutdown
    process.on('SIGINT', () => {
        console.log(pc.dim('\n\nShutting down...'));
        child.kill();
        process.exit(0);
    });
}

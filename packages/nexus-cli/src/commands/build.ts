import pc from 'picocolors';

interface BuildOptions {
    config: string;
}

export async function buildProduction(options: BuildOptions): Promise<void> {
    console.log(pc.dim(`  Config: ${options.config}\n`));

    // For v0.1, we delegate to Rollup
    // TODO: Implement production Virtual Chunking in v0.2
    console.log(pc.yellow('⚠️  v0.1 delegates to Rollup for production builds\n'));
    console.log(pc.dim('  [Not yet implemented - Issue #33]'));
    console.log(pc.dim('  For now, use: npx rollup -c\n'));
}

/* Nexus Core - Native Binding Loader */
/* Uses optionalDependencies pattern for cross-platform support */

const { platform, arch } = process;

function getBinding() {
  const platformArch = `${platform}-${arch}`;
  
  switch (platformArch) {
    case 'win32-x64':
      return require('@apexjs/core-win32-x64-msvc');
    case 'darwin-x64':
      return require('@apexjs/core-darwin-x64');
    case 'darwin-arm64':
      return require('@apexjs/core-darwin-arm64');
    case 'linux-x64':
      return require('@apexjs/core-linux-x64-gnu');
    default:
      throw new Error(
        `Unsupported platform: ${platform}-${arch}. ` +
        `Nexus supports: win32-x64, darwin-x64, darwin-arm64, linux-x64. ` +
        `Please open an issue at https://github.com/user/project-apex/issues ` +
        `if you need support for this platform.`
      );
  }
}

let binding;
try {
  binding = getBinding();
} catch (err) {
  throw new Error(
    `Failed to load Nexus native binding for ${platform}-${arch}. ` +
    `This usually means the pre-built binary for your platform is missing. ` +
    `Original error: ${err.message}`
  );
}

module.exports.startServer = binding.startServer;

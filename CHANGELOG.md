+ `v0.9.3`
  - Migrated error handling to `anyhow::Result` across server, client, child processes, timers, and tunnels
  - Included server version in `info` command response, and added support for querying `info version`

+ `v0.9.2`
  - Added `SKILL.md` agent skill guide and cheatsheet for AI coding assistants

+ `v0.9.1`
  - Breaking change: `stop all` now stops the child process named `"all"`; added `--all` / `-A` flags to stop all running processes
  - Removed internal magic keyword `"all"` in `Kindergarten::stop`, resolving naming conflicts with managed child processes
  - Comprehensive README redesign: added 60-second Quick Start, CI/CD recipes (`trystart`), and production operations guide (log outlets with `logrotate`, security hardening, Systemd integration)
  - Fixed output documentation to reflect actual supported modes (`create` and `append`)

+ `v0.9.0`
  - Replaced custom RSA encryption with Noise Protocol Framework (`Noise_XX_25519_ChaChaPoly_BLAKE2s`) for end-to-end mutual authentication and forward secrecy
  - Enhanced SSH-agent tunneling (`ssh://user@host`) via direct-tcpip channels, composable with Noise encryption
  - Added `keygen` CLI command and flags to generate 32-byte Curve25519 keypairs and derive public keys
  - Refactored CLI argument parsing to Clap v4 while maintaining backwards compatibility for 0.x preposition syntax
  - Upgraded codebase to Rust edition 2024

+ `v0.8.5`
  - config has its own location path
  - run the child with `.current_dir` and the **config location path**

+ `v0.8.4`
  - Update README
  
+ `v0.8.3`
  - Change to Rust 2021

+ `v0.8.2`
  - Change crossing compiling command in README

+ `v0.8.0`
  - Add SSH agent tunnel feature

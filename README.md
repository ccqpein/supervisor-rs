# supervisor-rs

A lightweight, secure, and asynchronous process supervisor daemon and client written in Rust.

[![Crates.io](https://img.shields.io/crates/v/supervisor-rs.svg)](https://crates.io/crates/supervisor-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

`supervisor-rs` manages background services and processes on local and remote servers. It provides process lifecycle management (start, stop, restart, auto-restart intervals, and lifecycle hooks) paired with modern secure networking options, including end-to-end encryption via the Noise Protocol and SSH-agent tunneling.

---

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
  - [Installation](#installation)
  - [60-Second Setup](#60-second-setup)
- [Architecture & Core Concepts](#architecture--core-concepts)
- [Configuration](#configuration)
  - [Server Configuration (`server.yml`)](#server-configuration-serveryml)
  - [Child Process Configuration (`<child>.yml`)](#child-process-configuration-childyml)
  - [Startup Modes (`mode`)](#startup-modes-mode)
  - [Periodic Execution (Repeat Feature)](#periodic-execution-repeat-feature)
  - [Lifecycle Hooks](#lifecycle-hooks)
- [Server Daemon Reference](#server-daemon-reference)
  - [Server CLI Options](#server-cli-options)
  - [Command Examples](#server-command-examples)
  - [Runtime Configuration Reloads](#runtime-configuration-reloads)
- [Client CLI Reference](#client-cli-reference)
  - [Client Commands](#client-commands)
  - [Client Options](#client-options)
  - [Command Examples & Multi-Host Dispatch](#command-examples--multi-host-dispatch)
  - [Legacy Syntax Notice](#legacy-syntax-notice)
- [Networking & Security](#networking--security)
  - [Custom Listener Address & IPv6 Support](#custom-listener-address--ipv6-support)
  - [SSH-Agent Tunneling](#ssh-agent-tunneling)
  - [Noise Protocol Encryption (Noise_XX)](#noise-protocol-encryption-noise_xx)
    - [1. Generating Keypairs](#1-generating-keypairs)
    - [2. Server Configuration](#2-server-configuration)
    - [3. Client Configuration & Server Verification](#3-client-configuration--server-verification)
    - [4. Authorization Behavior: Encryption-Only vs. Mutual Authentication](#4-authorization-behavior-encryption-only-vs-mutual-authentication)
- [CI/CD & Automation Recipes](#cicd--automation-recipes)
  - [Idempotent Deployments with `trystart`](#idempotent-deployments-with-trystart)
  - [Fleet Deployment Script (SSH Tunnels)](#fleet-deployment-script-ssh-tunnels)
  - [GitHub Actions Workflow Example](#github-actions-workflow-example)
- [Operations & Production Guide](#operations--production-guide)
  - [Crash & Failure Behavior](#crash--failure-behavior)
  - [Log Outlets & Log Rotation (`logrotate`)](#log-outlets--log-rotation-logrotate)
  - [Security Hardening & Key Permissions](#security-hardening--key-permissions)
  - [Systemd Service Integration](#systemd-service-integration)
  - [Cross-Compiling (musl-cross)](#cross-compiling-musl-cross)

---

## Features

- **Configuration-Driven Process Management**: Define processes via simple YAML files located across one or more load paths.
- **Dynamic Hot-Reloading**: Update child process definitions at runtime without restarting the server daemon.
- **Flexible Startup Modes**: Choose between `quiet` (manual control), `half` (start designated processes), or `full` (start all discovered processes).
- **Interval Timers & Periodic Actions**: Automatically re-run or restart child tasks on configured second intervals.
- **Pre-hook & Post-hook Lifecycle Chaining**: Execute pre-flight and cleanup commands automatically around process lifecycles.
- **End-to-End Encryption (Noise Protocol)**: Authenticated, encrypted communication (`Noise_XX_25519_ChaChaPoly_BLAKE2s`) with mutual authentication and forward secrecy.
- **SSH-Agent Tunneling**: Control remote daemons securely over SSH port 22 (`ssh://user@host`) using your existing SSH keys, keeping supervisor ports unexposed to the public internet.
- **Multi-Host Command Dispatch**: Target multiple local or remote servers simultaneously with a single client command.

---

## Quick Start

### Installation

Install directly via Cargo:

```bash
cargo install supervisor-rs
```

### 60-Second Setup

#### 1. Define a Child Process
Create a directory for your managed processes and define your first child configuration (e.g. `web.yml`):

```bash
mkdir -p /tmp/supervisor/apps
```

Save `/tmp/supervisor/apps/web.yml`:

```yaml
command: python3 -m http.server 8080
output:
  - stdout: /tmp/supervisor/web.log
    mode: append
  - stderr: /tmp/supervisor/web_error.log
    mode: append
```

#### 2. Create the Server Configuration
Save `/tmp/supervisor/server.yml`:

```yaml
loadpaths:
  - /tmp/supervisor/apps
mode: half
startup:
  - web
listener_addr: 127.0.0.1
```

#### 3. Start the Daemon and Manage via Client
Start the server daemon:

```bash
supervisor-rs-server /tmp/supervisor/server.yml
```

In another terminal, inspect and control your process using the client:

```bash
# Check status of running processes
supervisor-rs-client check

# Restart the process
supervisor-rs-client restart web

# Stop the process
supervisor-rs-client stop web
```

---

## Architecture & Core Concepts

- **Daemon (`supervisor-rs-server`)**: Runs persistently in the background. It scans configured `loadpaths` for child YAML files and manages active processes inside an internal process table (the *Kindergarten*).
- **Controller (`supervisor-rs-client`)**: Sends control commands (`start`, `restart`, `stop`, `check`, etc.) over TCP or SSH tunnels to one or more server daemons.
- **Child Identification**: Each child process is uniquely identified by the **filename** (without `.yml`) of its configuration file. Process names across all configured `loadpaths` must be unique.
- **Working Directory**: Each child process runs with its working directory set to the folder where its configuration YAML file resides.

---

## Configuration

### Server Configuration (`server.yml`)

The server configuration file governs the daemon's runtime behavior, discovery paths, and networking.

```yaml
# server.yml
loadpaths:
  - /tmp/supervisor/apps
  - /etc/supervisor/services

mode: "half"
startup:
  - web
  - worker

# Networking
listener_addr: 127.0.0.1
# ipv6: false

# Noise protocol encryption (optional)
noise: true
noise_key: /etc/supervisor/server.key
noise_authorized_keys:
  - /etc/supervisor/authorized_clients/
```

| Field | Type | Description |
| :--- | :--- | :--- |
| `loadpaths` | List of strings | Directories scanned by the server for child YAML configuration files. |
| `mode` | String | Startup mode: `"quiet"`, `"half"`, or `"full"`. Default is `"quiet"`. |
| `startup` | List of strings | When `mode` is `"half"`, children in this list are automatically started on boot. |
| `listener_addr` | String | IP address for the server to bind to (e.g. `127.0.0.1` or `0.0.0.0`). Default is `0.0.0.0`. |
| `ipv6` | Boolean | When `listener_addr` is omitted, binds to `::` instead of `0.0.0.0` if `true`. |
| `noise` | Boolean | Enable Noise Protocol encryption (`true` or `false`). |
| `noise_key` | String | Path to server's 32-byte Curve25519 private key (hex or raw binary). |
| `noise_authorized_keys` | String / List | Path to authorized client public keys file or directory. |

---

### Child Process Configuration (`<child>.yml`)

Each supervised process is configured in its own YAML file located in one of the server's `loadpaths`.

```yaml
# Example: /tmp/supervisor/apps/demo.yml (Child name is "demo")
command: /usr/local/bin/my-service --port 8080

output:
  - stdout: /var/log/demo.stdout.log
    mode: append
  - stderr: /var/log/demo.stderr.log
    mode: create

repeat:
  action: restart
  seconds: 60

hooks:
  - prehook: echo "Starting demo service..."
  - posthook: echo "Demo service stopped."
```

| Field | Description |
| :--- | :--- |
| `command` | The executable path and arguments to run. |
| `output` | List of output streams (`stdout` / `stderr`) with target file path and mode (`create` or `append`). If omitted, streams inherit the server daemon's standard I/O. |
| `repeat` | Optional periodic execution settings (`action` and interval in `seconds`). |
| `hooks` | Lifecycle hooks: `prehook` (runs before start/restart) and `posthook` (runs after stop). |

---

### Startup Modes (`mode`)

The server daemon supports three startup behaviors:

- **`quiet` (Default)**: The daemon registers all `loadpaths` but does **not** automatically start any child processes. Processes must be started explicitly via the client.
- **`half`**: The daemon automatically starts only the processes explicitly listed in the `startup` array. If an entry in `startup` is missing from `loadpaths`, it is safely skipped.
- **`full`**: The daemon starts **every** valid child configuration found across all `loadpaths`. Every YAML file in `loadpaths` must be a valid child configuration.

---

### Periodic Execution (Repeat Feature)

If a child configuration includes a `repeat` block, `supervisor-rs` initializes a timer after starting the child:

```yaml
repeat:
  action: restart
  seconds: 30
```

- **`action`**: Client command to execute when the timer fires (`restart`, `start`, or `trystart`). Defaults to `restart` if omitted.
- **`seconds`**: Interval duration in seconds (must be greater than 0).
- **PID Verification**: When the timer triggers, it verifies that the child's process ID matches the ID recorded when the timer was scheduled. If the process was replaced or stopped out-of-band, outdated timer actions are safely aborted.
- **Stopping Repeat**: To discontinue scheduled timers, remove or comment out the `repeat` block in the child YAML. Because configurations are reloaded before each execution, the subsequent cycle will not schedule a new timer.

---

### Lifecycle Hooks

Children can declare lifecycle hooks:

```yaml
command: /usr/local/bin/worker
hooks:
  - prehook: /usr/local/bin/setup-worker-env
  - posthook: /usr/local/bin/cleanup-worker-data
```

- **`prehook`**: Runs immediately before the child process starts or restarts. If prehooks form a chain, they execute sequentially. Recursive dependency cycles are rejected.
- **`posthook`**: Runs immediately after the child process terminates or is stopped.

---

## Server Daemon Reference

### Server CLI Options

Run `supervisor-rs-server [CONFIG]` or `supervisor-rs-server -c <CONFIG_PATH>`. If no configuration path is provided, the server defaults to `/tmp/server.yml`.

```
Usage: supervisor-rs-server [OPTIONS] [CONFIG]

Arguments:
  [CONFIG]  Optional positional configuration file path [default: /tmp/server.yml]

Options:
  -c, --config <CONFIG_PATH>              Specify configuration file path via flag
  -n, --noise                             Enable Noise protocol encryption via CLI
      --noise-key <KEY_PATH>              Path to server Noise private key
      --noise-authorized-keys <KEYS_PATH> Path to authorized client public keys directory or file
      --keygen                            Generate a new Noise Curve25519 keypair and exit
  -h, --help                              Print help information
  -V, --version                           Print version information
```

### Server Command Examples

```bash
# Run server with default config (/tmp/server.yml)
supervisor-rs-server

# Run server with a positional configuration path
supervisor-rs-server /etc/supervisor/server.yml

# Run server using the --config flag
supervisor-rs-server -c /etc/supervisor/server.yml

# Run server with Noise encryption enabled via CLI
supervisor-rs-server -c /etc/supervisor/server.yml \
  --noise \
  --noise-key /etc/supervisor/server.key \
  --noise-authorized-keys /etc/supervisor/authorized_clients/

# Generate a new Noise Curve25519 keypair
supervisor-rs-server --keygen
```

### Runtime Configuration Reloads

The server daemon holds a reference to its configuration file path and dynamically reloads configuration contents upon receiving child commands:
- **Load Paths**: You can add new directories to `loadpaths` in `server.yml` while the server is running. When you instruct the server to start a child located in a new path, the server re-reads `server.yml` and discovers the new child.
- **Child Definitions**: Modifying a `<child>.yml` file takes effect on the next `start`, `restart`, or `trystart` invocation.

---

## Client CLI Reference

`supervisor-rs-client` dispatches management commands to local or remote `supervisor-rs-server` daemons.

```bash
supervisor-rs-client <COMMAND> [OPTIONS]
```

### Client Commands

| Command | Arguments | Behavior |
| :--- | :--- | :--- |
| `check` | `[CHILD]` *(optional)* | Returns status and summary of running processes. If a child process is no longer alive, it is automatically purged from the table. |
| `start` | `<CHILD>` | Starts a new child process. If the child process panics or exits immediately, the start command reports initiation success; use `check` to verify live status. |
| `restart` | `<CHILD>` | Restarts an already running child process. Fails if the child is not currently active (use `start` or `trystart` instead). |
| `trystart` | `<CHILD>` | **Idempotent CI/CD command**: If the child is already running, it restarts it; if it is not running, it starts it. |
| `stop` | `<CHILD>` \| `-A, --all` | Stops a running child process by name (e.g. `stop all` stops the child named "all"), or supply `--all` / `-A` to terminate all managed processes. |
| `info` | `[CHILD]` *(optional)* | Retrieves runtime information, load paths, and configuration state from the server. |
| `kill` | `[CHILD]` *(optional)* | Terminates the server daemon itself and returns its final status response. |
| `keygen` | `[OUT]` *(optional)* | Generates a new Noise Curve25519 keypair. If an output prefix is supplied, creates `<out>.key` and `<out>.pub`. |

> [!WARNING]
> **Breaking Change (`stop` behavior):**
> Previously, `supervisor-rs-client stop all` was a magic reserved command that stopped all running processes.
> `all` is no longer a magic keyword. Running `supervisor-rs-client stop all` now attempts to stop the child process whose config is named `"all"`.
> To terminate all running child processes, you must use `supervisor-rs-client stop --all` (or `supervisor-rs-client stop -A`).

### Client Options

| Option | Description |
| :--- | :--- |
| `-o, --on <HOST>` | Target host address(es) (e.g. `127.0.0.1`, `192.168.1.1`, or `ssh://user@host`). Can be repeated or comma-separated. |
| `-n, --noise` | Enable end-to-end Noise Protocol encryption. |
| `-k, --noise-key <KEY_PATH>` | Path to client Noise private key (default: `~/.supervisor/client.key` or `./client.key`). |
| `--noise-authorized-keys <KEYS_PATH>` | Path to authorized server public keys directory or file (default: `./authorized_keys`). |
| `-h, --help` | Print help information (supports subcommand help: `supervisor-rs-client start --help`). |
| `-V, --version` | Print version information. |

### Command Examples & Multi-Host Dispatch

```bash
# Check status of local processes
supervisor-rs-client check

# Start or restart a child locally
supervisor-rs-client start web
supervisor-rs-client restart web

# CI/CD deployment: start if stopped, restart if running
supervisor-rs-client trystart web

# Stop a single process
supervisor-rs-client stop web

# Stop all processes
supervisor-rs-client stop --all
supervisor-rs-client stop -A

# NOTE (Breaking Change):
# supervisor-rs-client stop all
# The command above now stops the child process named "all"!
# It no longer stops all children. To terminate all processes, use --all or -A instead.

# Query status on a remote host over direct TCP
supervisor-rs-client check -o 192.168.1.10

# Dispatch command to multiple remote hosts simultaneously
supervisor-rs-client restart web -o 192.168.1.10 -o 192.168.1.11
supervisor-rs-client restart web --on "192.168.1.10, 192.168.1.11"

# Generate a Noise keypair for the client
supervisor-rs-client keygen ~/.supervisor/client

# Send encrypted commands using Noise Protocol
supervisor-rs-client restart web \
  -o 192.168.1.10 \
  --noise \
  -k ~/.supervisor/client.key \
  --noise-authorized-keys ~/.supervisor/authorized_keys/
```

### Legacy Syntax Notice

> [!NOTE]
> The preposition syntax `on <HOST>` is supported for backwards compatibility with 0.x releases:
> ```bash
> supervisor-rs-client restart web on 127.0.0.1
> supervisor-rs-client restart web on 192.168.1.10 on 192.168.1.11
> supervisor-rs-client restart web on "192.168.1.10, 192.168.1.11"
> ```

---

## Networking & Security

`supervisor-rs` supports three tiers of network transport:
1. **Direct TCP (IPv4 / IPv6)**: Fast, lightweight local or private-network access.
2. **SSH-Agent Tunneling**: Zero-configuration firewall traversal over SSH port 22 using existing SSH keys.
3. **Noise Protocol Encryption**: Cryptographically authenticated, forward-secret, end-to-end encrypted transport.

---

### Custom Listener Address & IPv6 Support

By default, `supervisor-rs-server` listens on `0.0.0.0:33889`. You can bind to a specific interface or enable IPv6 in `server.yml`:

```yaml
# Bind only to localhost (recommended when using SSH tunnels)
listener_addr: "127.0.0.1"

# Or bind to a specific IPv6 address
# listener_addr: "::1"

# Or bind to all IPv6 interfaces (only active when listener_addr is omitted)
# ipv6: true
```

When `listener_addr` is omitted and `ipv6: true` is configured, the server binds to `:::33889`.

---

### SSH-Agent Tunneling

When managing instances across public clouds or untrusted networks, exposing port `33889` directly is often undesired. The **SSH-agent tunnel feature** allows `supervisor-rs-client` to route commands through an SSH direct-tcpip channel (port 22) using your active SSH agent:

```
[supervisor-rs-client] ──(SSH Port 22 Tunnel)──> [sshd on Remote] ──(127.0.0.1:33889)──> [supervisor-rs-server]
```

#### Key Advantages
- **Zero Firewall Overhead**: No need to open port `33889` on security groups or firewalls; only standard SSH (port 22) is required.
- **No Client Binary on Server**: Only `supervisor-rs-server` must run on the remote host; `supervisor-rs-client` runs locally.
- **Layered Security**: Can be combined with the Noise Protocol for end-to-end authenticated encryption *inside* the SSH tunnel.

#### Setup

1. **Server Machine**:
   - Bind daemon locally in `server.yml`:
     ```yaml
     listener_addr: "127.0.0.1"
     ```
   - Start `supervisor-rs-server`.

2. **Client Machine**:
   - Ensure you can connect to the remote host with SSH key authentication.
   - Load your private key into your local SSH agent:
     ```bash
     ssh-add ~/.ssh/id_ed25519
     ```

#### Tunnel Usage

Target remote hosts using the `ssh://username@ipaddress` format:

```bash
# Check status over plain SSH tunnel
supervisor-rs-client check -o ssh://ubuntu@192.168.3.3

# Restart a process over SSH tunnel
supervisor-rs-client restart web -o ssh://ubuntu@192.168.3.3

# Combine SSH tunnel with Noise end-to-end encryption
supervisor-rs-client restart web \
  -o ssh://ubuntu@192.168.3.3 \
  --noise \
  -k ~/.supervisor/client.key
```

---

### Noise Protocol Encryption (Noise_XX)

`supervisor-rs` provides end-to-end authenticated encryption using the **Noise Protocol Framework** (`Noise_XX_25519_ChaChaPoly_BLAKE2s`).

- **Mutual Authentication**: Both client and server authenticate each other using 32-byte Curve25519 static public keys.
- **Identity Hiding**: Public keys are never transmitted in cleartext; they are encrypted during the handshake.
- **Forward Secrecy**: Ephemeral Diffie-Hellman keys are generated per session, preventing future decryption of recorded traffic if static keys are compromised.
- **AEAD Tamper Protection**: All payload frames are authenticated and encrypted using ChaCha20-Poly1305.

#### 1. Generating Keypairs

Keypairs can be generated using either the client or server CLI:

```bash
# Generate keys and output hex to stdout
supervisor-rs-client keygen
# or
supervisor-rs-server --keygen

# Generate and save keys directly to files (<prefix>.key and <prefix>.pub)
supervisor-rs-client keygen ~/.supervisor/client
# Creates:
#   ~/.supervisor/client.key  (Private key, hex format - keep secure on client)
#   ~/.supervisor/client.pub  (Public key, hex format - copy to server)

supervisor-rs-client keygen /etc/supervisor/server
# Creates:
#   /etc/supervisor/server.key  (Server private key)
#   /etc/supervisor/server.pub  (Server public key)
```

**Derive public key from an existing private key:**
```bash
# Print public key hex to stdout:
supervisor-rs-client keygen --pubkey-from ~/.supervisor/client.key

# Derive and write to client.pub:
supervisor-rs-client keygen ~/.supervisor/client --pubkey-from ~/.supervisor/client.key
```

#### 2. Server Configuration

To authorize clients on the server, place client public keys (`.pub` files) into an authorized directory:

```bash
mkdir -p /etc/supervisor/authorized_clients/
cp client_laptop.pub /etc/supervisor/authorized_clients/
```

Enable Noise in `server.yml`:

```yaml
noise: true
noise_key: /etc/supervisor/server.key
noise_authorized_keys:
  - /etc/supervisor/authorized_clients/   # Directory of authorized client .pub files
```

Or configure via server flags:

```bash
supervisor-rs-server -c /etc/supervisor/server.yml \
  --noise \
  --noise-key /etc/supervisor/server.key \
  --noise-authorized-keys /etc/supervisor/authorized_clients/
```

The server accepts authorized public keys from:
- A directory of `.pub` files (each containing a 64-character hex key).
- A single text file with hex keys listed one per line (`#` comments supported).

If a connecting client is unrecognized, the server terminates the handshake with `PermissionDenied`.

#### 3. Client Configuration & Server Verification

In mutual authentication (`Noise_XX`), the server sends its static public key to the client during the handshake. The client verifies this against its authorized server keys list to prevent Man-in-the-Middle (MITM) attacks:

```bash
# Place server's public key in client's authorized_keys directory
mkdir -p ~/.supervisor/authorized_keys
cp server.pub ~/.supervisor/authorized_keys/

# Connect with full mutual verification
supervisor-rs-client restart web \
  -o 192.168.1.10 \
  --noise \
  -k ~/.supervisor/client.key \
  --noise-authorized-keys ~/.supervisor/authorized_keys/
```

#### 4. Authorization Behavior: Encryption-Only vs. Mutual Authentication

> [!IMPORTANT]
> **Authorized Keys Verification Matrix**
>
> | Side | When `authorized_keys` is EMPTY / OMITTED | When `authorized_keys` is CONFIGURED |
> | :--- | :--- | :--- |
> | **Client** | **Accepts ANY server** (Traffic is encrypted with forward secrecy, but server identity is unverified). | **Rejects** any server whose public key is not in the authorized list (`PermissionDenied`). |
> | **Server** | **Accepts ANY client** (Traffic is encrypted with forward secrecy, but any client with a valid Noise key can connect). | **Rejects** any client whose public key is not in the authorized list (`PermissionDenied`). |
>
> - **Encryption-Only Mode**: Provide private keys (`-k` on client, `noise_key` on server) without specifying authorized keys. All communications are encrypted with ChaCha20-Poly1305 and forward secrecy without whitelisting.
> - **Mutual Authentication Mode**: Supply `authorized_keys` on both sides. Both client and server strictly verify each other's identities before processing commands.

---

## CI/CD & Automation Recipes

### Idempotent Deployments with `trystart`

In automated deployment pipelines (CI/CD, Ansible, or custom deployment scripts), scripts typically do not know whether a service is currently running, stopped, or being provisioned for the first time:
- **`start`** fails if the process is already running.
- **`restart`** fails if the process is not yet running (e.g. initial server provisioning, or if the process exited unexpectedly).
- **`trystart` is completely idempotent**:
  - If the process is currently **running**, `supervisor-rs` reloads its configuration and **restarts** it.
  - If the process is **not running**, `supervisor-rs` **starts** it.

This guarantees your continuous deployment step will always succeed without brittle conditional checks.

```bash
# Safely start or reload your web service during deployment
supervisor-rs-client trystart web
```

---

### Fleet Deployment Script (SSH Tunnels)

You can manage a fleet of servers over SSH without exposing port `33889` on external firewalls. The following bash deployment script updates and verifies an application across multiple remote nodes:

```bash
#!/usr/bin/env bash
set -euo pipefail

APP_NAME="web"
SERVERS=(
  "ssh://ubuntu@10.0.1.11"
  "ssh://ubuntu@10.0.1.12"
  "ssh://ubuntu@10.0.1.13"
)

echo "==> Deploying $APP_NAME to server fleet..."

for HOST in "${SERVERS[@]}"; do
  echo "--> Target: $HOST"

  # Idempotently start or restart the service
  supervisor-rs-client trystart "$APP_NAME" -o "$HOST"

  # Verify the child is healthy and running
  if supervisor-rs-client check "$APP_NAME" -o "$HOST" | grep -q "$APP_NAME"; then
    echo "    Status: OK"
  else
    echo "    Error: $APP_NAME failed to run on $HOST" >&2
    exit 1
  fi
done

echo "==> Deployment completed successfully across all nodes."
```

---

### GitHub Actions Workflow Example

Below is an example GitHub Actions workflow (`.github/workflows/deploy.yml`) that deploys a service over an SSH tunnel whenever changes are pushed to `main`:

```yaml
name: Deploy Production Service

on:
  push:
    branches: [ main ]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout Repository
        uses: actions/checkout@v4

      - name: Set up SSH Agent
        uses: webfactory/ssh-agent@v0.9.0
        with:
          ssh-private-key: ${{ secrets.PROD_SSH_PRIVATE_KEY }}

      - name: Install supervisor-rs-client
        run: cargo install supervisor-rs --bin supervisor-rs-client

      - name: Deploy Service via SSH Tunnel
        env:
          PROD_HOST: ${{ secrets.PROD_HOST_IP }}
        run: |
          # Idempotently deploy application
          supervisor-rs-client trystart api -o "ssh://ubuntu@$PROD_HOST"

          # Verify service is running
          supervisor-rs-client check api -o "ssh://ubuntu@$PROD_HOST"
```

---

## Operations & Production Guide

### Crash & Failure Behavior

Understanding process lifecycle boundaries during unexpected events is critical for production operations:

- **Daemon Termination / Kill**: If `supervisor-rs-server` terminates unexpectedly, crashes, or is killed via `supervisor-rs-client kill`, **running child processes are NOT killed**. They are detached and safely reparented to the system init process (PID 1).
- **Graceful Host Shutdown**: If you need to stop all managed processes before shutting down the daemon or host, issue `stop --all` first:
  ```bash
  # 1. Gracefully terminate all child applications
  supervisor-rs-client stop --all

  # 2. Terminate the supervisor daemon
  supervisor-rs-client kill
  ```
- **Zombie Process Prevention**: Child processes that exit on their own are tracked inside the internal table. Whenever a client executes `check`, `start`, or `restart`, the server automatically reaps defunct processes and clears them from memory.

---

### Log Outlets & Log Rotation (`logrotate`)

`supervisor-rs` separates logging into two distinct outlets:

1. **Daemon Operational Logs**:
   - The daemon emits timestamped operational messages directly to standard I/O.
   - When running under Systemd, these messages are automatically routed to the system journal:
     ```bash
     journalctl -u supervisor -f
     ```

2. **Child Process Log Outlets (`stdout` and `stderr`)**:
   - Each child process routes output directly to configured log files:
     - `create`: Truncates existing log files upon each start.
     - `append`: Appends to existing log files without data loss.

#### Managing Child Logs with `logrotate`

Because child processes hold open file descriptors when logging in `append` mode, use `logrotate`'s `copytruncate` directive to rotate logs without requiring service restarts:

Create `/etc/logrotate.d/supervisor`:

```ini
/var/log/supervisor/*.log {
    daily
    missingok
    rotate 14
    compress
    delaycompress
    notifempty
    copytruncate
}
```

---

### Security Hardening & Key Permissions

When deploying in production, ensure strict file permissions on your configuration files and cryptographic keys:

1. **Restrict Private Key Files**:
   Noise Curve25519 private keys must be accessible only by the process owner:
   ```bash
   # On Server
   chmod 600 /etc/supervisor/server.key

   # On Client
   chmod 600 ~/.supervisor/client.key
   ```

2. **Restrict Configuration & Authorized Keys Directories**:
   Ensure write access to authorized keys is restricted to administrative users:
   ```bash
   chmod 700 /etc/supervisor/authorized_clients/
   chmod 700 ~/.supervisor/authorized_keys/
   chmod 644 /etc/supervisor/server.yml
   ```

3. **Run as an Unprivileged Service User**:
   Do not run `supervisor-rs-server` as `root` unless child processes require privileged network ports (< 1024) or root-level capabilities. Create a dedicated system user and set ownership accordingly.

---

### Systemd Service Integration

To manage `supervisor-rs-server` as a persistent background service, create `/etc/systemd/system/supervisor.service`:

```ini
[Unit]
Description=supervisor-rs process supervisor daemon
After=network.target

[Service]
Type=simple
TimeoutStartSec=0
ExecStart=/usr/local/bin/supervisor-rs-server /etc/supervisor/server.yml
Restart=on-failure
RestartSec=5s
User=ubuntu
Group=ubuntu
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

Enable and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now supervisor
```

---

### Cross-Compiling (musl-cross)

To produce static Linux binaries on macOS:

```bash
# 1. Install musl-cross via Homebrew
brew tap filosottile/musl-cross
brew install FiloSottile/musl-cross/musl-cross

# 2. Configure target linker in ~/.cargo/config
cat << 'EOF' >> ~/.cargo/config
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"
EOF

# 3. Build static release binaries
env CC_x86_64_unknown_linux_musl=x86_64-linux-musl-gcc \
  cargo build --target=x86_64-unknown-linux-musl --release
```

# supervisor-rs #

`supervisor-rs` used to be a manager of handle application running. `supervisor-rs` can start/restart/stop processing.

- [Usage](#usage)
  - [Server Side](#server-side)
  - [Client Side](#client-side)
  - [Noise Protocol Encryption](#noise-protocol-encryption)
    - [1. Generating Keys](#1-generating-keys)
    - [2. Server Configuration](#2-server-configuration)
    - [3. Client Configuration & Server Verification](#3-client-configuration--server-verification)
    - [4. Authorization Behavior: Encryption-Only vs. Mutual Authentication](#4-authorization-behavior-encryption-only-vs-mutual-authentication)
  - [Startup-with feature](#startup-with-feature)
  - [Repeat feature](#repeat-feature)
    - [How to stop repeat](#how-to-stop-repeat)
  - [Hooks feature](#hooks-feature)
  - [Custom Listener address & IPV6 support](#custom-listener-address--ipv6-support)
  - [SSH-agent tunnel feature](#ssh-agent-tunnel-feature)
  - [What if accident happens](#what-if-accident-happens)
- [Cross compiling](#cross-compiling)
- [Systemd integration](#systemd-integration)

**Features**:

+ Start different processing depend on particular yaml file when startup
+ Start processing when have new config in load path
+ Startup with particular server config
+ Restart processing
+ Stop processing
+ End-to-end encrypted client/server communication via Noise Protocol (Noise_XX)

**Design**:

1. server/client mode
2. server start -> load config files from loadpaths (if it is not quiet mode) -> do job
3. start/stop/restart/tryrestart special processing (client side)

**Config yaml files format**:

Example of server.yaml:

```yaml
# server side config
loadpaths:
  - /tmp/client/
  - /tmp/second/path
  
mode: "half"
startup:
  - child1
  - child2
  
# Noise encryption (optional)
noise: true
noise_key: /tmp/server.key
noise_authorized_keys:
  - /tmp/authorized_keys/

#ipv6: true
listener_addr: 127.0.0.1
```

| Fields                 | Usage                                                                                                                                                                  |
|:----------------------:|:----------------------------------------------------------------------------------------------------------------------------------------------------------------------:|
| loadpaths              | List of paths of all children config files.                                                                                                                            |
| mode                   | Startup mode. Values can be "quiet", "half", or "full"                                                                                                                 |
| startup                | When the `mode` is "half", children in this list will start                                                                                                            |
| noise                  | Enable Noise protocol encryption. Values can be `true` or `false`                                                                                                      |
| noise_key              | Path to server's 32-byte Curve25519 private key (hex or raw binary)                                                                                                    |
| noise_authorized_keys  | Path (or list of paths) to authorized client public keys directory or file                                                                                            |
| listener_addr          | Address of server side is listening                                                                                                                                    |
| ipv6                   | Only used when `listener_addr` isn't given. Values can be `true` or `false`. supervisor-rs server side will listen "::" instead of "0.0.0.0" when this field is `true` |


Example of child's config yaml:

```yaml
#each child config in loadpath of server config
command: /tmp/test
output:
  - stdout: aaaaaa
    mode: create

  - stderr: nnnnn
    mode: append

repeat:
  action: restart
  seconds: 5

hooks:
  - prehook: start child
  - posthook: start child
```

## Usage ##

You can download compiled binary file directly.

You can install from cargo.io, run `cargo install supervisor-rs`.

### Server Side ###

Run `supervisor-rs-server [CONFIG]` or `supervisor-rs-server -c <CONFIG_PATH>` in shell. You can specify a server config YAML file path. If no config path is given, `supervisor-rs` defaults to finding `/tmp/server.yml`.

**CLI options:**

- `[CONFIG]`: Optional positional configuration file path (default: `/tmp/server.yml`)
- `-c, --config <CONFIG_PATH>`: Specify configuration file path via flag
- `-n, --noise`: Enable Noise protocol encryption via CLI
- `--noise-key <KEY_PATH>`: Path to server Noise private key
- `--noise-authorized-keys <KEYS_PATH>`: Path to authorized client public keys directory or file
- `--keygen`: Generate a new Noise Curve25519 keypair and exit
- `-h, --help`: Print help information
- `-V, --version`: Print version information

**Command demo:**

```bash
# Run server with default config (/tmp/server.yml)
supervisor-rs-server

# Run server with positional config path
supervisor-rs-server ./test/server.yml

# Run server with --config flag
supervisor-rs-server -c ./test/server.yml
supervisor-rs-server --config ./test/server.yml

# Run server with Noise encryption enabled via CLI
supervisor-rs-server -c ./test/server.yml --noise --noise-key /tmp/server.key --noise-authorized-keys /tmp/authorized_keys/

# Generate a new Noise keypair
supervisor-rs-server --keygen
```

After server application start, if `mode` is **full**, then all **application yaml files under loadpath of server config** will be ran by application. So, that means every yaml files in there should be legal application config file, or server cannot start.

Server side's default mode is `quiet`, means server will record `loadpaths`, but won't start children automatically.

Each sub-processing is named with **filename** of yaml file. If have multi-loadpath, make sure **no yaml files have same name**. 

**Change server's config while runtime**

After server start, config path is strict but the content of config is negotiable. It means you can change what config is, but you cannot change where is it. 

For example, you just want to add a new `loadpaths` to server. You can easily change `loadpaths` in server's config. However, server does not make it change immediately because no necessary. 

Then you put a child config inside the `loadpath` you just add, then let it start ([Go to Client Side for usage](#client-side)). Server will re-read configuration and start it. Every children run in the same path (current path of children processing) that the config located in.

Technically, server only keep configuration path, and read it again when server need to operate children.

### Client Side ###

`supervisor-rs-client` is used to send commands to the server side daemon.

**Usage:**

```bash
supervisor-rs-client <COMMAND> [OPTIONS]
```

**Options:**

- `-o, --on <HOST>`: Remote host address(es) to send command to (e.g. `127.0.0.1`, `192.168.1.1`, or `ssh://user@host`). Can be specified multiple times or comma-separated.
- `-n, --noise`: Enable Noise protocol encryption
- `-k, --noise-key <KEY_PATH>`: Path to client Noise private key (default: `~/.supervisor/client.key` or `./client.key`)
- `--noise-authorized-keys <KEYS_PATH>`: Path to authorized server public keys directory or file (default: `./authorized_keys`)
- `-h, --help`: Print help information (supports both global help and subcommand help, e.g. `supervisor-rs-client start --help`)
- `-V, --version`: Print version information

> [!NOTE]
> **Legacy Syntax Notice:**
> The `on <HOST>` preposition syntax is legacy (0.x) syntax supported for backwards compatibility:
> - `supervisor-rs-client restart child0 on 127.0.0.1` *(legacy)*
> - `supervisor-rs-client restart child0 on 192.168.1.1 on 192.168.1.2` *(legacy)*
> - `supervisor-rs-client restart child0 on "192.168.1.1, 192.168.1.2"` *(legacy)*

**Commands:**

| command  | arguments       | behavior                                                                                                                                                                                                                                                                                   |
| ---      | ---             | ---                                                                                                                                                                                                                                                                                        |
| restart  | `<CHILD>`       | restart child on server. this child has to be running (server application). Otherwise, use start instead                                                                                                                                                                                   |
| start    | `<CHILD>`       | start new child. This command can start one-time command, or new config just put in loadpath(s). And, start does not care what's happen in child itself. If it start and panic immediately, supervisor will return success message anyway. Use `check` command to check if it runs or not. |
| stop     | `<CHILD>`       | stop running child. Have to supply child name. If want to stop all children, use `stop all`                                                                                                                                                                                                |
| check    | `[CHILD]` (opt) | return summary of all children who are **running**. If children are not running, no matter what reason, they will be cleaned from kindergarden's table.                                                                                                                                    |
| trystart | `<CHILD>`       | special command for CI/CD to start child processings. `restart` only works when child is running; `start` only works when child is not running. `trystart` will run child processing anyway, if it is running, restart; if it is not running, start it.                                    |
| kill     | `[CHILD]` (opt) | kill will terminate server and return last words from server                                                                                                                                                                                                                               |
| info     | `[CHILD]` (opt) | get general information of server self                                                                                                                                                                                                                                                     |
| keygen   | `[OUT]` (opt)   | generate a new Noise Curve25519 keypair, optionally saving `<out>.key` and `<out>.pub`                                                                                                                                                                                                     |

**Examples:**

```bash
# Restart child0 locally
supervisor-rs-client restart child0

# Restart child0 on a remote host
supervisor-rs-client restart child0 -o 192.168.1.1
# [Legacy] preposition syntax:
supervisor-rs-client restart child0 on 192.168.1.1

# Send to multiple hosts
supervisor-rs-client restart child0 -o 192.168.1.1 -o 192.168.1.2
supervisor-rs-client restart child0 --on "192.168.1.1, 192.168.1.2"
# [Legacy] preposition syntax:
supervisor-rs-client restart child0 on 192.168.1.1 on 192.168.1.2

# Check status of running children
supervisor-rs-client check
supervisor-rs-client check -o 192.168.1.1
# [Legacy] preposition syntax:
supervisor-rs-client check on 192.168.1.1

# Stop all children
supervisor-rs-client stop all

# Generate a new Noise keypair for the client
supervisor-rs-client keygen
# Or write to files (creates client.key and client.pub):
supervisor-rs-client keygen ~/.supervisor/client

# Send encrypted command with Noise protocol
supervisor-rs-client restart child0 -o 192.168.1.1 --noise -k ~/.supervisor/client.key --noise-authorized-keys ~/.supervisor/authorized_keys/
```

### Noise Protocol Encryption ###

`supervisor-rs` supports end-to-end encrypted client/server communication using the **Noise Protocol Framework** (`Noise_XX_25519_ChaChaPoly_BLAKE2s`).

**Security Benefits:**
- **Mutual Authentication**: Both the client and the server authenticate each other's 32-byte Curve25519 static public keys.
- **Identity Hiding**: Both client and server static public keys are transmitted encrypted across the wire.
- **Forward Secrecy**: Fresh ephemeral Diffie-Hellman keys are negotiated for every connection, ensuring past sessions cannot be decrypted if keys are later compromised.
- **Tamper-proof Framing**: Every message is protected with ChaCha20-Poly1305 AEAD authenticated encryption.

#### 1. Generating Keys ####

You can generate 32-byte Curve25519 keypairs directly via the client or server CLI:

```bash
# Generate keys and print hex to stdout
supervisor-rs-client keygen
# or
supervisor-rs-server --keygen

# Generate keys and save directly to files (<prefix>.key and <prefix>.pub)
supervisor-rs-client keygen ~/.supervisor/client
# Creates:
#   ~/.supervisor/client.key  (Private key, hex format - keep this secret on client)
#   ~/.supervisor/client.pub  (Public key, hex format - copy this to server's authorized_clients/)

supervisor-rs-client keygen /etc/supervisor/server
# Creates:
#   /etc/supervisor/server.key  (Server private key)
#   /etc/supervisor/server.pub  (Server public key)
```

**Extract public key from an existing private key:**
If you only have `client.key` and need its public key:
```bash
# Print public key hex to stdout:
supervisor-rs-client keygen --pubkey-from ~/.supervisor/client.key

# Or derive and save directly to client.pub:
supervisor-rs-client keygen ~/.supervisor/client --pubkey-from ~/.supervisor/client.key
```

#### 2. Server Configuration ####

To authorize a client on the server, place the client's public key (`client.pub`) into the server's `authorized_clients/` directory:

```bash
# On the server machine:
mkdir -p /etc/supervisor/authorized_clients/
cp /path/to/client.pub /etc/supervisor/authorized_clients/client_laptop.pub
```

Enable Noise encryption in `server.yml`:

```yaml
# server.yml
loadpaths:
  - /tmp/client/

noise: true
noise_key: /etc/supervisor/server.key
noise_authorized_keys:
  - /etc/supervisor/authorized_clients/   # Directory of authorized client .pub files
```

Alternatively, enable and configure via server CLI flags:

```bash
supervisor-rs-server -c /tmp/server.yml --noise --noise-key /etc/supervisor/server.key --noise-authorized-keys /etc/supervisor/authorized_clients/
```

The server reads authorized client public keys from:
- A directory containing `.pub` files (each containing a 64-character hex key).
- A single file containing hex keys (one per line, `#` comments supported).

If a client attempts to connect with an unrecognized public key, the server rejects the handshake with `PermissionDenied`.

#### 3. Client Configuration & Server Verification ####

**Does the client side also need an `authorized_keys/` folder?**

**Yes, for full verification!** In mutual authentication (`Noise_XX`), the server sends its static public key to the client during the handshake. To protect against Man-in-the-Middle (MITM) attacks and rogue servers, the client verifies the server's public key against an authorized server keys directory or file.

If the server's public key is not in the client's `authorized_keys`, the client terminates the connection immediately.

**Sending commands with Noise:**

```bash
# Put server public key into client's authorized_keys folder
mkdir -p ~/.supervisor/authorized_keys
cp /path/to/server.pub ~/.supervisor/authorized_keys/

# Send encrypted command with full mutual verification
supervisor-rs-client restart child0 \
  -o 192.168.1.1 \
  --noise \
  -k ~/.supervisor/client.key \
  --noise-authorized-keys ~/.supervisor/authorized_keys/

# Send encrypted command without server whitelisting (encryption-only)
supervisor-rs-client restart child0 \
  -o 192.168.1.1 \
  --noise \
  -k ~/.supervisor/client.key
```

#### 4. Authorization Behavior: Encryption-Only vs. Mutual Authentication ####

> [!IMPORTANT]
> **What happens if `authorized_keys` is omitted or empty?**
>
> | Side | When `authorized_keys` is EMPTY | When `authorized_keys` is CONFIGURED |
> | :--- | :--- | :--- |
> | **Client** | **Accepts ANY server** (traffic is fully encrypted, but server identity is not verified). | **Rejects** any server whose public key is not in the list (`PermissionDenied`). |
> | **Server** | **Accepts ANY client** (traffic is fully encrypted, but any client with a valid Noise key can connect). | **Rejects** any client whose public key is not in the list (`PermissionDenied`). |
>
> - **Encryption-Only Mode**: Simply supply private keys (`-k` on client, `noise_key` on server) without specifying authorized keys. All communications are protected with ChaCha20-Poly1305 and Forward Secrecy, but no public key whitelisting is enforced.
> - **Mutual Authentication Mode**: Add public keys to `authorized_keys` on both sides. Both client and server verify each other's identities and reject any untrusted peers.

### Startup-with feature ###

If server's config `mode` is `half`, server will try to startup all children in `startup` list when it starts.

Demo:

```yaml
#server side config
loadpaths:
  - /tmp/client/
  - /tmp/second/path
  
mode: "half"
startup:
  - child1
  - child2
  - child3
```

server will try to start `child1`, `child2`, and `child3` when it startup

**QA:**

Q: if child3 not exist?

A: server will going to find children in `startup`, if some of them not exist in loadpaths, server will skip them.

Q: what if I forget write mode to half?

A: server default mode is quiet, and startup list only used in `half` mode, otherwise, `startup` won't effect anything.


### Repeat feature ###

if config of child has `repeat` field:

```yaml
#file name (child name) is demo.yml
command: /tmp/test
output:
  - stdout: aaaaaa
    mode: create

  - stderr: nnnnn
    mode: append
repeat:
  action: restart
  seconds: 5 #only support seconds now
  
```

then, when you start it, `supervisor-rs` will give a timer stand by and send back command when time is up. For example, config above will run `restart demo` every 5 seconds. 

`action`'s values are command we using in supervisor-rs-client, so you even can `stop` child with `repeat` field. But so, `supervisor-rs` won't create a timer stand by. 

Only `start`, `restart`, and `trystart` will let `supervisor-rs` create a timer, if `repeat` field exists.

If `action` is empty, `supervisor-rs` will give `restart` be default value. However, `seconds` has to have value, and it cannot be 0.


#### How to stop repeat ####

As I said above, `timer` be created right after child runs. So you cannot stop "next" action, but if you change child's config, like delete repeat field, then "next" action won't create a timer. 

This is because all `start`, `restart` and `trystart` will **reload** config of child before it does its job.

So, what will supervisor do if child has `stopped`, or `restart` manually before timer finish its waiting and send command to supervisor again, timer isn't outdated? Timer will check if child has same processing id as when it created timer. If this check passed, timer will do its job as normal, else, timer won't do anything because child current is not child before.

### Hooks feature ###

Each child can have two hooks, one `prehook`, one `posthook`. `prehook` command will run before main child `start`/`restart`. `posthook` will run after child `stop`.

If prehooks command child has anther prehook, means there is a prehooks chain, they will run one by one, and they cannot have hooks circle.

example:

```yaml
command: sleep 10
hooks:
  - prehook: start child
  - posthook: start child
```

### Custom Listener address & IPV6 support ###

If you want to use specifically listener IP address instead of `0.0.0.0` (default listener address), you can easily put `listener_addr` field in your server side config yaml file. 

Example:

```yaml
#server side config
loadpaths:
  - /tmp/client/
  - /tmp/second/path

listener_addr: "127.0.0.1" # only listen local
```

If you want use IPV6 instead of IPV4, you can just change `listener_addr` to IPV6 address you want. Or just turn `ipv6` field to `true`.

Example:

```yaml
#server side config
loadpaths:
  - /tmp/client/
  - /tmp/second/path

listener_addr: "::1" # only listen local ipv6
```

`ipv6` field only used when there is **no** `listener_addr` given, or `supervisor-rs` server side will ignore `ipv6`. If there is no `listener_addr` given, and `ipv6` is true, `supervisor-rs` will start with listen ipv6 address `::`.

### SSH-Agent Tunnel Feature ###

By default, `supervisor-rs-server` listens on `0.0.0.0:33889`. While the [Noise Protocol Encryption](#noise-protocol-encryption) provides end-to-end encryption and mutual authentication for direct TCP connections, you may not want to expose port `33889` to the public internet at all when deploying to cloud environments.

The **SSH-agent tunnel feature** allows the client to connect to the remote host securely over SSH (port 22) using your existing SSH keys, and execute supervisor commands locally on the remote machine.

#### How It Works

Instead of exposing port `33889` to the internet or executing remote shell commands, `supervisor-rs-client`:
1. Connects to port 22 of the remote machine via SSH.
2. Authenticates using your local SSH-agent.
3. Opens a direct-tcpip channel directly to `127.0.0.1:33889` on the remote server.
4. Communicates directly with the supervisor daemon through the encrypted tunnel:
   - If `--noise` is enabled, the client performs the Noise handshake and end-to-end encryption through the SSH tunnel using your local Noise keys!
   - If `--noise` is omitted, commands are sent directly through the encrypted SSH tunnel.

#### Setup

**1. Server Side:**
- No extra server configuration is needed.
- You do **not** need `supervisor-rs-client` installed on the remote machine; only `supervisor-rs-server` needs to be running.
- You can lock down the server daemon to only listen locally by setting `listener_addr: "127.0.0.1"` in `server.yml` so that external machines cannot reach port `33889` directly.

**2. Client Side:**
- Ensure you can SSH into the remote machine with key-based authentication.
- Add your SSH private key to your local SSH-agent:
  ```bash
  ssh-add ~/.ssh/id_rsa
  ```
- Optional: Add an entry to your `~/.ssh/config` if needed:
  ```ssh-config
  Host myserver
      HostName 192.168.3.3
      User ubuntu
      IdentityFile ~/.ssh/id_rsa
  ```

#### Usage

Specify the target host with the `ssh://username@ipaddress` format:

```bash
# Check status of children via SSH tunnel with Noise encryption
supervisor-rs-client check -o ssh://ubuntu@192.168.3.3 --noise -k ~/.supervisor/client.key

# Check status of children via plain SSH tunnel (server without Noise)
supervisor-rs-client check -o ssh://ubuntu@192.168.3.3

# Restart a child process via SSH tunnel
supervisor-rs-client restart child0 -o ssh://ubuntu@192.168.3.3 --noise -k ~/.supervisor/client.key

# [Legacy] preposition syntax:
supervisor-rs-client restart child0 on ssh://ubuntu@192.168.3.3
```

### What if accident happens ###

* if supervisor-rs be killed by `kill`, children won't stop, they will be taken by system.
* if supervisor-rs panic, children won't stop.

Go to log to find more information if `supervisor-rs-server` have problem

## Cross compiling ##

`brew tap filosottile/musl-cross && brew install FiloSottile/musl-cross/musl-cross`

after install `musl-cross`, `which x86_64-linux-musl-gcc` will give a result, like `/usr/local/bin/x86_64-linux-musl-gcc`.

give configuration in `~/.cargo/config`

```
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"
```

then, `env CC_x86_64_unknown_linux_musl=x86_64-linux-musl-gcc cargo build --target=x86_64-unknown-linux-musl --release`, there is no errors in my local machine.

## Systemd integration ##

This is the example of `supervisor.service`

```ini
[Unit]
Description=supervisor
After=network.target

[Service]
TimeoutStartSec=0
ExecStart=/home/ubuntu/.cargo/bin/supervisor-rs-server /home/ubuntu/supervisor/server.yml
EnvironmentFile=/home/ubuntu/supervisor/supervisor.conf
StandardOutput=append:/home/ubuntu/supervisor/log
StandardError=inherit
Restart=on-failure
User=ubuntu
Group=ubuntu
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

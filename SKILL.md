---
name: supervisor-rs
description: Manage and orchestrate background processes using the supervisor-rs client and server. Use when starting, restarting, inspecting, stopping, or creating background services locally or remotely over SSH/Noise.
---

# supervisor-rs Agent Skill

`supervisor-rs` is a lightweight process supervisor daemon (`supervisor-rs-server`) and controller CLI (`supervisor-rs-client`).

As an agent, you will primarily invoke **`supervisor-rs-client`** via shell commands to manage background applications.

---

## 1. Quick Command Cheatsheet

| Task | Command | Notes |
| :--- | :--- | :--- |
| **Check all processes** | `supervisor-rs-client check` | Returns names, PIDs, and configs of live processes. |
| **Check single process** | `supervisor-rs-client check <child>` | Reaps exited processes and returns status. |
| **Idempotent Start/Restart** | `supervisor-rs-client trystart <child>` | **Recommended for agents**. Restarts if running, starts if stopped. |
| **Start process** | `supervisor-rs-client start <child>` | Starts a new child. Fails if already running. |
| **Restart process** | `supervisor-rs-client restart <child>` | Restarts a running child. Fails if not running. |
| **Stop single process** | `supervisor-rs-client stop <child>` | Stops `<child>`. (e.g. `stop web`). |
| **Stop ALL processes** | `supervisor-rs-client stop --all` | Use `--all` or `-A`. **Do NOT use `stop all`**. |
| **Daemon information** | `supervisor-rs-client info` | Displays server `loadpaths`, mode, and config state. |
| **Terminate daemon** | `supervisor-rs-client kill` | Shuts down daemon. (Running children are reparented to init). |

---

## 2. Standard Agent Workflows

### A. Deploying or Updating a Service (Golden Pattern)
Always prefer **`trystart`** over `start` or `restart`. It is completely idempotent and safe against unknown process states:

```bash
# 1. Deploy / Reload service
supervisor-rs-client trystart <child>

# 2. Verify it is running
supervisor-rs-client check <child>
```

### B. Adding a Brand-New Service
1. Run `supervisor-rs-client info` to discover configured `loadpaths`:
   ```bash
   supervisor-rs-client info
   ```
2. Write a new `<child>.yml` inside one of the discovered `loadpaths` (see template below).
3. Start the newly created child:
   ```bash
   supervisor-rs-client start <child>
   supervisor-rs-client check <child>
   ```
   > **Note**: The server dynamically re-reads `loadpaths` on demand. You **do not** need to restart the daemon.

### C. Modifying an Existing Service
1. Edit the child's `<child>.yml` file in place.
2. Run `supervisor-rs-client trystart <child>`.
   > The daemon automatically reloads the YAML file before restarting the process.

---

## 3. Remote Operations & Multi-Host Dispatch

Target remote hosts without opening supervisor ports by routing through SSH:

```bash
# Run command over SSH-agent tunnel (port 22)
supervisor-rs-client check -o ssh://ubuntu@192.168.1.10
supervisor-rs-client trystart web -o ssh://ubuntu@192.168.1.10

# Direct TCP target (IPv4 or IPv6)
supervisor-rs-client check -o 192.168.1.10

# Dispatch to multiple hosts simultaneously
supervisor-rs-client trystart web -o ssh://ubuntu@10.0.0.1 -o ssh://ubuntu@10.0.0.2

# Noise Protocol end-to-end encrypted connection
supervisor-rs-client trystart web -o 192.168.1.10 --noise -k ~/.supervisor/client.key
```

---

## 4. Child Configuration Specification (`<child>.yml`)

File naming rule: The child name is the filename **without** `.yml` (e.g., `/path/to/apps/worker.yml` defines child `worker`).

Working directory rule: The child process runs with its working directory (`cwd`) set to the folder containing its `.yml` file.

```yaml
# Required: executable and arguments
command: /usr/bin/python3 main.py --port 8080

# Optional: Output redirection
output:
  - stdout: /var/log/worker.stdout.log
    mode: append   # "append" (retains history) or "create" (truncates on start)
  - stderr: /var/log/worker.stderr.log
    mode: append

# Optional: Periodic timer execution
repeat:
  action: restart  # "restart", "start", or "trystart"
  seconds: 60      # Interval in seconds (> 0)

# Optional: Lifecycle hooks
hooks:
  - prehook: /path/to/pre-flight-check.sh
  - posthook: /path/to/cleanup.sh
```

> **Tip**: If `output` is omitted, child stdout and stderr are inherited directly from the server daemon.

---

## 5. Starting the Server Daemon (If Not Running)

If `supervisor-rs-client` reports `"Looks like client cannot reach server side..."`:

```bash
# Start daemon with default config (/tmp/server.yml)
supervisor-rs-server &

# Or start daemon with custom configuration
supervisor-rs-server /etc/supervisor/server.yml &
```

Minimal `server.yml`:
```yaml
loadpaths:
  - /var/supervisor/apps
mode: quiet           # "quiet" (manual start), "half" (starts startup list), "full" (starts all)
listener_addr: 127.0.0.1
```

---

## 6. Critical Rules for Agents

1. **Never use `stop all` to stop everything**:
   - `supervisor-rs-client stop all` targets a process named `"all"`.
   - To stop all processes, always pass `--all` or `-A`:
     ```bash
     supervisor-rs-client stop --all
     ```

2. **Always verify after mutating**:
   - `start` returns success when the OS forks the process. If the command immediately exits or panics, `start` will still report success.
   - Always run `supervisor-rs-client check <child>` immediately after `start` or `trystart` to confirm the process is healthy.

3. **No daemon restart needed for config changes**:
   - Editing a child YAML or dropping a new YAML into `loadpaths` is detected automatically on the next command. Do not kill or restart the daemon.

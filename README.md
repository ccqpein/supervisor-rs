# README #

`supervisor-rs` used to be a manager of handle application running. `supervisor-rs` can start/restart/stop(still developing) processing.

**features**:

- [x] start different processing depend on particular yaml file when startup
- [x] start processing when have new config in load path
- [x] startup with particular server config
- [x] restart processing
- [x] stop processing
- [x] redirect stdout and stderr to log file
- [x] client should talk with server's side supervisor-rs
- [ ] help command
- [x] server/client should talk to each other (maybe not too much)
- [x] server should has check feature and can return check result to client

Compress log file maybe not good ideas, change running processing's file handle is too much work for `supervisor-rs`. 


**design**:

1. server/client mode
2. server start -> load config file -> do job
3. restart special processing (client side)

**config yaml file format**:

server.yaml:

```yaml
#server side config
loadpaths:
  - /tmp/client/
```

each command's config:

```yaml
#each child config in loadpath of server config
command: /tmp/test
output:
  - stdout: aaaaaa
    mode: create

  - stderr: nnnnn
    mode: append
```

## Usage ##

**Server Side**

Start server side application. After compiled, run `server /tmp/server.yml` in shell, you can change server config yaml file to wherever you want. If no config given, supervisor will going to find `server.yml` in `/tmp`.

After server application start, all **application yaml files under loadpath of server config** will be ran by application. So, that's means every yaml files in there should be legal application config file, or server cannot start.

Each sub-processing is named with **filename** of yaml file. 


**Client Side**

*Restart child processing*:

`client restart child0 on localhost` will restart processing `child0` on localhost;

`client restart child0 on 198.0.0.2` will restart processing `child0` on 192.0.0.2, I assume you running server side application on this host;

child name is not must for `check`/`kill`.

commands:

| command  | behavior                                                                                                                                                                                                                                                                                   |
| ---      | ---                                                                                                                                                                                                                                                                                        |
| restart  | restart child on server. this child has to be running (server application). Otherwise, use start instead                                                                                                                                                                                   |
| start    | start new child. This command can start one-time command, or new config just put in loadpath(s). And, start does not care what's happen in child itself. If it start and panic immediately, supervisor will return success message anyway. Use `check` command to check if it runs or not. |
| stop     | stop running child. Have to supply child name. If want to stop all children, use `stop all`                                                                                                                                                                                                |
| check    | return summary of all children who are **running**. If children are not running, no matter what reason, they will be cleaned from kindergarden's table.                                                                                                                                    |
| trystart | special command for CI/CD to start child processings. `restart` only works when child is running; `start` only works when child is not running. `trystart` will run child processing anyway, if it is running, restart; if it is not running, start it.                                    |




**Cross compiling**

`brew tap filosottile/musl-cross && brew install FiloSottile/musl-cross/musl-cross`

after install `musl-cross`, `which x86_64-linux-musl-gcc` will give a result, like `/usr/local/bin/x86_64-linux-musl-gcc`.

give configuration in `~/.cargo/config`

```
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"
```

then, `cargo build --target=x86_64-unknown-linux-musl`, there is no errors in my local machine.

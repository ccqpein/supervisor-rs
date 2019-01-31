# README #

`supervisor-rs` used to be a manager of handle application running. `supervisor-rs` can start/restart/stop(still developing) processing.

**features**:

- [x] start different processing depend on particular yaml file when startup
- [x] start processing when have new config in load path
- [x] startup with particular server config
- [x] restart processing
- [x] stop processing
- [x] redirect stdout and stderr to log file
- [ ] ~~compress log file when log file become huge~~
- [x] client should talk with server's side supervisor-rs

Compress log file maybe not good ideas, change running processing's file handle is too much work for `supervisor-rs`. 


**design**:

1. server/client mode
2. server start -> load config file -> do job
3. restart special processing (client side)

**config yaml file format**:

```yaml
#server side config
loadpath:
  - /tmp/client/
```

```yaml
#each child config in loadpath of server config
Command:
  - /tmp/test
Stdout:
  - /tmp/log
```

## Usage ##

**Server Side**

Start server side application. After compiled, run `server /tmp/server.yml` in shell, you can change server config yaml file to wherever you want. 


After server application start, all **application yaml files under loadpath of server config** will be ran by application. So, that's means every yaml files in there should be legal application config file.


**Client Side**

*Restart child processing*:

`client restart child0 on localhost` will restart processing `child0` on localhost;

`client restart child0 on 198.0.0.2` will restart processing `child0` on 192.0.0.2, I assume you running server side application on this host;

**Cross compiling**

`brew tap filosottile/musl-cross && brew install FiloSottile/musl-cross/musl-cross`

after install `musl-cross`, `which x86_64-linux-musl-gcc` will give a result, like `/usr/local/bin/x86_64-linux-musl-gcc`.

give configuration in `~/.cargo/config`

```
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"
```

then, `cargo build --target=x86_64-unknown-linux-musl`, there is no errors in my local machine.

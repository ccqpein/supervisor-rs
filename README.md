# README #

`supervisor-rs` used to be a manager of handle application running. `supervisor-rs` can start/restart/stop(still developing) processing redirect log depend on yaml config file.

**features**:

- [X] start different processing depend on particular yaml file when startup
- [X] start processing when have new config in load path
- [ ] startup with particular server config
- [X] restart processing
- [ ] stop processing
- [X] redirect stdout and stderr to log file
- [ ] compress log file when log file become huge

Compress log file maybe not good ideas, change running processing's file handle is too much work for `supervisor-rs`. 


**design**:

1. server/client mode
2. server start -> load config file -> do job
3. restart special command (client side)
4. refresh -> load config file again -> start new job without quit old jobs (client side)
5. stop -> stop special command but keep config in memory (client side)


**yaml file format**:

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

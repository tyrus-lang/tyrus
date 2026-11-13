```bash
tyrus build --release --pgo-generate   # instrumented binary
./target/<name>                        # run it on a real workload
tyrus pgo-merge                        # fuse the profiles into one file
tyrus build --release --pgo-use        # compile against the profile
```

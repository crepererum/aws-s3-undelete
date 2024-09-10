# AWS S3 Undeleter

## Build Minimal Linux Binary
Useful if you need to ship this to a server:

```console
$ cargo build --profile=mini --target x86_64-unknown-linux-musl
...

$ upx --best target/x86_64-unknown-linux-musl/mini/aws-s3-undelete
```

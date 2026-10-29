# AI-Teaching Codex runtime

This directory owns the complete build and packaging contract for the Codex
runtime consumed by AI-Teaching. The application repository consumes the
published image and does not compile Rust.

Build locally from the repository root:

```sh
integrations/ai-teaching/runtime/build-image.sh develop --load
```

The scratch image exposes one stable consumer interface: `/rootfs`. Consumers
copy that directory into their application image. Runtime policy, Skills,
licenses, notices, the App Server binary, toolchain version, Cargo settings and
image metadata are all versioned here.

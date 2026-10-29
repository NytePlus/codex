# AI-Teaching Codex runtime

This directory owns the complete build and packaging contract for the Codex
runtime consumed by AI-Teaching. AI-Teaching pins this repository as the
`services/codex` submodule and uses this Dockerfile as a Compose build context.
The application repository does not duplicate Rust toolchain or Cargo settings.

Build locally from the repository root:

```sh
integrations/ai-teaching/runtime/build-image.sh develop --load
```

The scratch image exposes one stable consumer interface: `/rootfs`. Consumers
copy that directory into their application image. Runtime policy, Skills,
licenses, notices, the App Server binary, toolchain version, Cargo settings and
image metadata are all versioned here. Docker caches this build independently
from the parent application's source, so only changes to this repository or its
runtime recipe require recompiling Rust.

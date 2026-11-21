# Production Docker Image

The production image is built from the repository's [`Dockerfile`](../Dockerfile).
[`Dockerfile.ci`](../Dockerfile.ci) is the separate image the release workflow
assembles from the pre-built musl release binary, and is not covered here.

## Build and run

```bash
docker build -t sanctifier .

docker run --rm \
  --read-only --cap-drop ALL --security-opt no-new-privileges \
  -v "$PWD:/workspace:ro" \
  sanctifier analyze /workspace
```

`analyze` exits `0` when no issues are found, `1` when it reports findings and
`2` on errors (see `tooling/sanctifier-cli/src/exit_codes.rs`).

HTTP mode listens on port 9100. Inside a container it must bind to all
interfaces:

```bash
docker run --rm --read-only --cap-drop ALL -p 9100:9100 sanctifier serve --bind 0.0.0.0
```

## Hardening

| Property | How |
| --- | --- |
| Minimal base | Final stage is `gcr.io/distroless/static-debian12:nonroot`: CA certificates, tzdata, `/etc/passwd`, and nothing else. No shell, no package manager, no libc. |
| No toolchain | Multi-stage build: the Rust toolchain, `musl-tools` and the build cache stay in the `builder` stage. Only `/usr/local/bin/sanctifier` is copied out. |
| Static binary | Built for `<arch>-unknown-linux-musl`, so it has no shared-library dependencies (the glibc build needs `libc`, `libm` and `libgcc_s`). |
| Non-root | `USER nonroot:nonroot` (uid/gid 65532). The binary is owned by root and not writable by the runtime user. |
| Read-only root filesystem | The CLI writes nothing to disk during `analyze`, so `--read-only` works without any tmpfs mounts. |
| Scannable dependencies | Built with `cargo auditable`, which embeds the crate list in the binary, so Trivy reports vulnerable Rust crates as well as OS packages. |

The CI job in [`.github/workflows/docker-scan.yml`](../.github/workflows/docker-scan.yml)
checks the first four properties on every change. It smoke-tests the image
with `--read-only --cap-drop ALL`, asserts the user is not root and that
`/bin/sh` is absent, then runs a Trivy scan (see below).

## Size

Measured on `linux/amd64` (uncompressed, `docker image inspect`):

| Image | Size |
| --- | --- |
| `rust:1.98-bookworm` builder stage (what a single-stage build would ship) | 2.17 GB |
| `distroless/cc-debian12:nonroot` + glibc binary (first iteration) | 49.0 MB |
| **`distroless/static-debian12:nonroot` + static musl binary (current)** | **18.0 MB** |

The first iteration on `distroless/cc` also shipped `libssl3` and `libstdc++6`.
The binary never loads either library, yet both carried HIGH CVEs that would
have needed permanent Trivy waivers. Switching to a static binary removed those
packages altogether.

## Vulnerability scanning

Trivy scans the production image on every PR or push that touches the image
inputs, weekly on a schedule, and as a required gate in `release.yml`. The
release `docker` job waits for that scan, then scans its own image and pushes
only if that scan also passes.

- Unwaived **CRITICAL** or **HIGH** findings fail the job.
- Full reports (JSON + SARIF, all severities) are uploaded as the
  `trivy-report-<sha>` artifact.
- Accepted findings go in [`.trivyignore`](../.trivyignore), each with a
  justification, tracking issue and `exp:` date.

# syntax=docker/dockerfile:1

# The binaries this copies are built by the release workflow on native runners
# and statically linked against musl, which is the whole reason distroless/static
# is enough: nothing here needs a loader, a libc, or a trust store. sqlx is on
# rustls-ring with webpki-roots (see the root Cargo.toml), so `--dsn` over TLS
# works with the Mozilla roots compiled into the binary.
#
# There is no build stage and no RUN, so nothing in this image is ever executed
# during the build. That is what lets buildx produce a multi-arch manifest
# without QEMU: COPY runs on the host, not in a foreign-arch container.
FROM gcr.io/distroless/static:nonroot

# amd64 | arm64, filled in by buildx from each --platform entry.
ARG TARGETARCH

# The mode is set here rather than trusted from whatever unpacked the release
# archive.
# docker-stage/, not dist/: `craber build -o dist` and `just site` already own
# that name, and a stale site sitting there should not look like a build input.
COPY --chmod=0755 docker-stage/$TARGETARCH/craber /usr/local/bin/craber

# 65532 is distroless' `nonroot` user. Numeric so Kubernetes' runAsNonRoot can
# verify it without resolving a name.
USER 65532:65532

# `craber build -o /work/dist` is the intended use, against a bind mount. Note
# that `craber serve` is *not* usable in a container: it binds loopback inside
# the container with no flag to change that, so a published port cannot reach it.
WORKDIR /work
ENTRYPOINT ["/usr/local/bin/craber"]

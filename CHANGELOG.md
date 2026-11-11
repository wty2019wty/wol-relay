# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Support Windows, with clean shutdown on Ctrl+C, Ctrl+Break, console close and
  system shutdown events
- Publish prebuilt binaries for Linux and Windows (x86, x86_64, ARM64) on GitHub
  releases, with SHA-256 checksums
- Publish container images for linux/386 (x86) alongside linux/amd64 and linux/arm64
- Add CI workflows: lint and tests on all supported targets, GHCR image publishing
  and binary releases

### Changed

- Move `libc` and `signal-hook` to Unix-only dependencies; Windows uses
  `windows-sys` for interface alias lookups and console control events
- Build the x86 container image from Alpine's Rust toolchain, since `rust:1-alpine`
  has no i386 variant

### Fixed

- Build a fully static binary for the x86 container image: Alpine's packaged Rust
  links dynamically by default, which broke the `scratch` runtime with
  `exec /app/wol-relay: no such file or directory`

## [0.2.0] - 2026-10-14

> The service is now written in Rust. The Go implementation remains available on the
> `release/0.1` branch, which receives security fixes only.

### Added

- Add `--version` flag
- Add log level and format selection via `-l`/`-F` flags and environment variables
- Add target selection via the `WOL_RELAY_TARGETS` environment variable
- Log skipped packets and resolved networks at debug level
- Handle SIGTERM and SIGINT for clean shutdown
- Build multi-arch Docker images (amd64, arm64) with a publish smoke test

### Changed

- Rewrite the service in Rust
- Log one flat JSON object per line with lowercase levels, matching the previous zerolog shape
- Reword log messages to a consistent lowercase style and move all variable data into structured fields (`argument`, `port`, `networks`, `signal`, `error`)
- Exit on unrecoverable receive errors so the supervisor can restart the relay
- Cache Docker dependency builds with cargo-chef
- Ignore interface addresses with unknown netmasks (macOS secondary addresses)

### Fixed

- Skip interface addresses that report an unknown netmask, which previously
  resolved to a `0.0.0.0/0` network that monitored every IPv4 source
- Drop duplicate monitored networks, which previously relayed the same packet
  multiple times

## [0.1.13] - 2026-05-10

### Added

- Accept CIDR notation as a target alongside interface names ([14c7683](https://github.com/danroc/wol-relay/commit/14c7683))

### Changed

- Clean up packet handling and startup error paths ([b8080de](https://github.com/danroc/wol-relay/commit/b8080de))
- Add golangci-lint configuration ([38fcae5](https://github.com/danroc/wol-relay/commit/38fcae5))
- Update dependencies
  - Update module `github.com/mattn/go-isatty` to v0.0.22 ([#74](https://github.com/danroc/wol-relay/pull/74), [#68](https://github.com/danroc/wol-relay/pull/68))
  - Update module `github.com/rs/zerolog` to v1.35.1 ([#72](https://github.com/danroc/wol-relay/pull/72), [#63](https://github.com/danroc/wol-relay/pull/63))
  - Update module `golang.org/x/sys` to v0.44.0 ([#76](https://github.com/danroc/wol-relay/pull/76), [#67](https://github.com/danroc/wol-relay/pull/67), [#56](https://github.com/danroc/wol-relay/pull/56), [#47](https://github.com/danroc/wol-relay/pull/47), [#39](https://github.com/danroc/wol-relay/pull/39))
  - Update golang docker tag to v1.26.3 ([#75](https://github.com/danroc/wol-relay/pull/75), [#66](https://github.com/danroc/wol-relay/pull/66), [#55](https://github.com/danroc/wol-relay/pull/55), [#46](https://github.com/danroc/wol-relay/pull/46), [#42](https://github.com/danroc/wol-relay/pull/42))
  - Update golang docker digests ([#79](https://github.com/danroc/wol-relay/pull/79), [#78](https://github.com/danroc/wol-relay/pull/78), [#77](https://github.com/danroc/wol-relay/pull/77), [#71](https://github.com/danroc/wol-relay/pull/71), [#70](https://github.com/danroc/wol-relay/pull/70), [#62](https://github.com/danroc/wol-relay/pull/62), [#61](https://github.com/danroc/wol-relay/pull/61), [#60](https://github.com/danroc/wol-relay/pull/60), [#59](https://github.com/danroc/wol-relay/pull/59), [#58](https://github.com/danroc/wol-relay/pull/58), [#57](https://github.com/danroc/wol-relay/pull/57), [#51](https://github.com/danroc/wol-relay/pull/51), [#50](https://github.com/danroc/wol-relay/pull/50), [#49](https://github.com/danroc/wol-relay/pull/49), [#40](https://github.com/danroc/wol-relay/pull/40), [#38](https://github.com/danroc/wol-relay/pull/38), [#37](https://github.com/danroc/wol-relay/pull/37), [#36](https://github.com/danroc/wol-relay/pull/36), [#35](https://github.com/danroc/wol-relay/pull/35))
  - Update `docker/build-push-action` to v7 ([#54](https://github.com/danroc/wol-relay/pull/54), [#48](https://github.com/danroc/wol-relay/pull/48), [#69](https://github.com/danroc/wol-relay/pull/69))
  - Update `docker/login-action` to v4 ([#52](https://github.com/danroc/wol-relay/pull/52), [#64](https://github.com/danroc/wol-relay/pull/64), [#43](https://github.com/danroc/wol-relay/pull/43))
  - Update `docker/metadata-action` to v6 ([#53](https://github.com/danroc/wol-relay/pull/53))
  - Update `actions/checkout` digest ([#44](https://github.com/danroc/wol-relay/pull/44))

### Fixed

- Detect self-sourced packets via local addresses to prevent relay loops when a CIDR target overlaps a local subnet ([2752788](https://github.com/danroc/wol-relay/commit/2752788))
- Force IPv4 sockets to avoid IPv4-mapped IPv6 source addresses ([ec4af01](https://github.com/danroc/wol-relay/commit/ec4af01))

## [0.1.12] - 2025-12-21

### Changed

- Simplify logging ([cdd7955](https://github.com/danroc/wol-relay/commit/cdd7955), [c55b4e0](https://github.com/danroc/wol-relay/commit/c55b4e0))
- Replace logging library ([3bbd2b2](https://github.com/danroc/wol-relay/commit/3bbd2b2))
- Update dependencies
  - Update zerolog deps ([52afec1](https://github.com/danroc/wol-relay/commit/52afec1))
  - Update golang:1.25.5 docker digest to 36b4f45 ([#33](https://github.com/danroc/wol-relay/pull/33))
  - Update golang:1.25.5 docker digest to a22b2e6 ([#32](https://github.com/danroc/wol-relay/pull/32))
  - Update golang:1.25.5 docker digest to 0ece421 ([#31](https://github.com/danroc/wol-relay/pull/31))
  - Update golang:1.25.5 docker digest to 68ee6df ([#30](https://github.com/danroc/wol-relay/pull/30))
  - Update module golang.org/x/sys to v0.39.0 ([#29](https://github.com/danroc/wol-relay/pull/29))
  - Update actions/checkout digest to 8e8c483 ([#27](https://github.com/danroc/wol-relay/pull/27))
  - Update golang docker tag to v1.25.5 ([#28](https://github.com/danroc/wol-relay/pull/28))
  - Update docker/metadata-action digest to c299e40 ([#26](https://github.com/danroc/wol-relay/pull/26))
  - Pin dependencies ([#25](https://github.com/danroc/wol-relay/pull/25))
  - Use Renovate best practices ([64e4c10](https://github.com/danroc/wol-relay/commit/64e4c10))
  - Update actions/checkout action to v6 ([#24](https://github.com/danroc/wol-relay/pull/24))

## [0.1.11] - 2025-11-11

### Changed

- Update dependencies
  - Update module golang.org/x/sys to v0.38.0 ([#22](https://github.com/danroc/wol-relay/pull/22))
  - Update golang docker tag to v1.25.4 ([#21](https://github.com/danroc/wol-relay/pull/21))
  - Update golang docker tag to v1.25.3 ([#20](https://github.com/danroc/wol-relay/pull/20))
  - Update module golang.org/x/sys to v0.37.0 ([#18](https://github.com/danroc/wol-relay/pull/18))
  - Update golang docker tag to v1.25.2 ([#19](https://github.com/danroc/wol-relay/pull/19))
  - Update golang docker tag to v1.25.1 ([#17](https://github.com/danroc/wol-relay/pull/17))
  - Update golang docker tag to v1.25.0 ([#16](https://github.com/danroc/wol-relay/pull/16))
  - Update actions/checkout action to v5 ([#15](https://github.com/danroc/wol-relay/pull/15))
  - Update module golang.org/x/sys to v0.35.0 ([#14](https://github.com/danroc/wol-relay/pull/14))
  - Update golang docker tag to v1.24.6 ([#13](https://github.com/danroc/wol-relay/pull/13))

## [0.1.10] - 2025-07-14

### Fixed

- Continue main loop on errors ([94b91e7](https://github.com/danroc/wol-repeater/commit/94b91e7))

## [0.1.9] - 2025-07-12

### Changed

- Improve log field names ([c96941f](https://github.com/danroc/wol-repeater/commit/c96941f))
- Update dependencies
  - Update golang docker tag to v1.24.5 ([#11](https://github.com/danroc/wol-repeater/pull/11))
  - Update module golang.org/x/sys to v0.34.0 ([#12](https://github.com/danroc/wol-repeater/pull/12))

## [0.1.8] - 2025-07-06

### Changed

- Rename `remote` to `source` in log messages ([15a8cbb](https://github.com/danroc/wol-repeater/commit/15a8cbb))

## [0.1.7] - 2025-07-06

### Fixed

- Call `network.String()` to fix log messages ([a362af2](https://github.com/danroc/wol-repeater/commit/a362af2))

## [0.1.6] - 2025-07-06

### Fixed

- Don't call unnecessary `String()` ([5dda3a3](https://github.com/danroc/wol-repeater/commit/5dda3a3))

## [0.1.5] - 2025-07-06

### Added

- Add error field to log messages ([dd73bde](https://github.com/danroc/wol-repeater/commit/dd73bde))

## [0.1.4] - 2025-07-06

### Added

- Add context to log messages ([490fc69](https://github.com/danroc/wol-repeater/commit/490fc69))
- Add Docker build target ([fba71e2](https://github.com/danroc/wol-repeater/commit/fba71e2))
- Add link flags to build command ([1b95620](https://github.com/danroc/wol-repeater/commit/1b95620))
- Add unit tests for `main.go` ([8052766](https://github.com/danroc/wol-repeater/commit/8052766))
- Add Makefile ([36f43d7](https://github.com/danroc/wol-repeater/commit/36f43d7))

### Fixed

- Fix lint errors ([1a3ed98](https://github.com/danroc/wol-repeater/commit/1a3ed98))

## [0.1.3] - 2025-06-19

### Added

- Add a license ([7c5db8a](https://github.com/danroc/wol-repeater/commit/7c5db8a))
- Add a CHANGELOG ([c236c51](https://github.com/danroc/wol-repeater/commit/c236c51))
- Add features, dev and standalone sections to README ([#9](https://github.com/danroc/wol-repeater/pull/9))
- Add unit tests to `wol` package ([#6](https://github.com/danroc/wol-repeater/pull/6))

### Changed

- Reduce indentation by handling error first ([467e3cb](https://github.com/danroc/wol-repeater/commit/467e3cb))
- Update dependencies
  - Update golang docker tag to v1.24.4 ([#10](https://github.com/danroc/wol-repeater/pull/10))
  - Update golang docker tag to v1.24.3 ([#5](https://github.com/danroc/wol-repeater/pull/5))
- Remove dummy example section from README ([3f1f1bd](https://github.com/danroc/wol-repeater/commit/3f1f1bd))

### Fixed

- Remove unreachable conditions ([#7](https://github.com/danroc/wol-repeater/pull/7))

## [0.1.2] - 2025-05-06

### Added

- Add usage example to README.md ([858f394](https://github.com/danroc/wol-repeater/commit/858f394))
- Add Renovate
  - Add Renovate configuration ([44822f6](https://github.com/danroc/wol-repeater/commit/44822f6))
  - Add renovate.json ([#1](https://github.com/danroc/wol-repeater/pull/1))

### Changed

- Update dependencies
  - Update module golang.org/x/sys to v0.33.0 ([#4](https://github.com/danroc/wol-repeater/pull/4))
  - Update module golang.org/x/sys to v0.32.0 ([#3](https://github.com/danroc/wol-repeater/pull/3))
- Improve comments to clarify WOL packet sending logic ([26703f4](https://github.com/danroc/wol-repeater/commit/26703f4))

### Fixed

- Check if remote is in a monitored network ([abad8d8](https://github.com/danroc/wol-repeater/commit/abad8d8))

## [0.1.1] - 2025-05-05

### Changed

- Format network in log message ([3ff6afb](https://github.com/danroc/wol-repeater/commit/3ff6afb))

## [0.1.0] - 2025-05-05

### Added

- Add Dockerfile ([fb66a97](https://github.com/danroc/wol-repeater/commit/fb66a97))
- Implement WOL relaying ([bdc9b73](https://github.com/danroc/wol-repeater/commit/bdc9b73))
- Add action to publish releases ([700181a](https://github.com/danroc/wol-repeater/commit/700181a))
- Initial commit ([a3e5665](https://github.com/danroc/wol-repeater/commit/a3e5665))

### Changed

- Format log message ([bf2248e](https://github.com/danroc/wol-repeater/commit/bf2248e))
- Remove invalid newlines ([604f805](https://github.com/danroc/wol-repeater/commit/604f805))
- Don't export helper functions ([14942f2](https://github.com/danroc/wol-repeater/commit/14942f2))
- Add `BuildPacket` function ([db3fa9c](https://github.com/danroc/wol-repeater/commit/db3fa9c))
- Change buffer size to 1024 ([9efb630](https://github.com/danroc/wol-repeater/commit/9efb630))
- Return errors in `wol.ParsePacket` ([75725c5](https://github.com/danroc/wol-repeater/commit/75725c5))
- Use logrus as logger ([8739a5b](https://github.com/danroc/wol-repeater/commit/8739a5b))
- Remove `main` binary ([a9dc4e4](https://github.com/danroc/wol-repeater/commit/a9dc4e4))

[Unreleased]: https://github.com/danroc/wol-relay/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/danroc/wol-relay/compare/v0.1.13...v0.2.0
[0.1.13]: https://github.com/danroc/wol-relay/compare/v0.1.12...v0.1.13
[0.1.12]: https://github.com/danroc/wol-relay/compare/v0.1.11...v0.1.12
[0.1.11]: https://github.com/danroc/wol-relay/compare/v0.1.10...v0.1.11
[0.1.10]: https://github.com/danroc/wol-relay/compare/v0.1.9...v0.1.10
[0.1.9]: https://github.com/danroc/wol-relay/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/danroc/wol-relay/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/danroc/wol-relay/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/danroc/wol-relay/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/danroc/wol-relay/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/danroc/wol-relay/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/danroc/wol-relay/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/danroc/wol-relay/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/danroc/wol-relay/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/danroc/wol-relay/releases/tag/v0.1.0

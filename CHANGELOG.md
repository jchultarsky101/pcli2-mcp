# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

### Changed

- **Requires pcli2 2.0 or newer.** Tool definitions were synced with the pcli2 2.0.1 command tree.
- `pcli2_asset_thumbnail` now passes `--output` (pcli2 2.0 removed `--file`); thumbnail calls failed against pcli2 2.x before this fix.
- Tool schemas and argv construction are generated from a single declarative table (`src/tools.rs`); arguments are validated (required keys, enums, numeric ranges, types) before pcli2 runs, and string-typed booleans/numbers are accepted.
- pcli2 is always spawned non-interactively with `PCLI2_NO_INPUT=1`, `PCLI2_NO_COLOR=1`, `PCLI2_NO_UPDATE_CHECK=1` and `PCLI2_ERROR_FORMAT=json`; JSON error lines from pcli2 are rendered as `LEVEL (kind): message` in tool errors.
- `pcli2` (legacy list tool) with `resource=asset` now accepts `folder_uuid` as well as `folder_path`.
- `pcli2_tenant_use` requires `name` (or the legacy alias `tenant_name`) instead of failing at pcli2.
- Long flags (`--tenant`, `--format`) are used instead of `-t`/`-f`.

### Added

- New pcli2 2.x options on existing tools: `recursive`, `checkpoint`, `format=xls` + `output` (folder geometric-match), `limit`/`threshold` (visual-match), `limit` (text-match), `reload` (folder resolve), metadata type `url`, `yes` on mutating tools.
- New tools: `pcli2_doctor`, `pcli2_tenant_clear`, `pcli2_tenant_metadata_list`, `pcli2_folder_list`, `pcli2_folder_create`, `pcli2_folder_delete`, `pcli2_folder_rename`, `pcli2_folder_move`, `pcli2_folder_download`, `pcli2_folder_upload`, `pcli2_folder_thumbnail`, `pcli2_auth_login`, `pcli2_auth_logout`, `pcli2_auth_get`, `pcli2_auth_clear_token`, `pcli2_auth_expiration`, `pcli2_asset_list`, `pcli2_asset_create`, `pcli2_asset_create_batch`, `pcli2_asset_delete`, `pcli2_asset_download`, `pcli2_asset_dependency_diff`, `pcli2_asset_geometric_match`, `pcli2_asset_similarity`, `pcli2_asset_counts`, `pcli2_asset_inventory`, `pcli2_asset_metadata_get`, `pcli2_asset_metadata_create_batch`, `pcli2_asset_metadata_inference`, `pcli2_config_validate`, `pcli2_config_export`, `pcli2_config_import`, `pcli2_cache_clear`.
- Unit test table with an argv case for every tool (fails when a tool is added without one); integration tests with an argv-echoing mock pcli2 covering `call_tool`, the thumbnail `--output` path, the child environment, and JSON error rendering.

## [0.1.15] - 2026-04-15

### Changed

- Updated README: fix stale version reference (v0.1.11 → v0.1.15), consolidate features section, remove redundant "Enhanced Features" section.

## [0.1.14] - 2026-04-15

### Added

- `pcli2_environment_use` tool → `pcli2 env use --name <name>`
- `pcli2_environment_add` tool → `pcli2 env add --name <name> [--api-url ...] [--ui-url ...] [--auth-url ...]`
- `pcli2_environment_remove` tool → `pcli2 env remove --name <name>`
- `pcli2_environment_reset` tool → `pcli2 env reset`
- `pcli2_user_list` tool → `pcli2 user list`
- `pcli2_user_get` tool → `pcli2 user get <user_id>`

### Changed

- Environment tools (`pcli2_environment_list` / `_get`) now invoke the canonical `pcli2 env ...` alias instead of `pcli2 environment ...`.

## [0.1.13] - 2026-04-15

### Changed

- **Breaking (MCP tool names):** Renamed `pcli2_config_environment_list` → `pcli2_environment_list` and `pcli2_config_environment_get` → `pcli2_environment_get` to match pcli2 v1.1.7, which promoted `environment` from a `config` subcommand to a top-level command.
- `pcli2` list tool: when `resource=asset`, `folder_path` is now required and `folder_uuid` is no longer accepted (reflects pcli2 v1.1.7 `asset list` signature). A new `recursive` option forwards `--recursive` to `asset list`.

## [0.1.12] - 2026-02-20

### Changed

- Documentation: emphasize `url` mode is recommended for LLM thumbnail workflows over `data_url` mode

## [0.1.11] - 2026-02-19

### Added

- Thumbnail cache for efficient image serving via HTTP URLs (`/thumbnail/:cache_key`)
- New tool `pcli2_thumbnail_cache_cleanup` to remove expired thumbnails
- Disk-based cache at `~/.pcli2-mcp/thumbnails/` with configurable TTL (default 24 hours)
- HTML response for `pcli2_asset_thumbnail` with embedded image URL instead of base64 data

### Changed

- `pcli2_asset_thumbnail` now returns HTML with image URL instead of base64-encoded image data
- AppState extended to include optional thumbnail cache reference

## [0.1.10] - 2026-02-19

### Fixed

- Verified correct MIME type handling for thumbnail downloads (`image/png` with PNG magic byte validation).

## [0.1.9] - 2026-02-12

### Added

- Comprehensive unit and integration tests for all modules (45+ unit tests, 10+ integration tests).
- Server start time indicator in logs for better operational awareness.
- Chrono dependency for time formatting capabilities.

### Changed

- Improved banner formatting with centered version display.
- Changed default log level from debug to info for cleaner output.
- Added clear instruction to press Ctrl+C to stop the server.
- Modularized codebase into separate files (cli, error, mcp, pcli2, server) for better maintainability.
- Fixed tracing initialization to prevent duplicate subscriber errors.

## [0.1.8] - 2026-02-11

### Added

- Request body size limits and timeouts for MCP HTTP requests.
- PCLI2 command timeouts and max output size enforcement.

### Changed

- Stricter JSON-RPC request validation for required fields and types.
- Tool schema construction refactored for consistency and reuse.

## [0.1.7] - 2026-02-11

### Added

- Added MCP tool for `pcli2 asset metadata delete`.

### Changed

- Metadata delete tool now accepts multiple names and optional `format`.
- Banner now displays the running `pcli2-mcp` version.

## [0.1.6] - 2026-02-11

### Changed

- Improved MCP request logging with clearer tool context.

## [0.1.5] - 2026-02-11

### Changed

- Logged PCLI2 commands are now shell-escaped for copy/paste (with a compact emoji prefix).

## [0.1.4] - 2026-02-11

### Added

- Oranda documentation publishing via GitHub Pages.
- Oranda configuration and docs workflow.
- Quick Start validation example for listing MCP tools with curl.

### Changed

- README restructured to introduce core concepts and step-by-step setup.
- Installation guidance now prefers pre-built binaries and references source builds as optional.

## [0.1.3] - 2026-02-09

### Added

- Added tenant state filtering via the new `--type` option for `pcli2 tenant state`.
- Streamed asset thumbnails over MCP as base64 image content with fallback data URLs.

## [0.1.2] - 2026-02-09

### Added

- Support for additional PCLI2 tools (tenant, config/environment, folder, asset, metadata).
- CLI improvements: `serve --log-level` and `config --host`.
- Mock `pcli2` integration test for tool wiring.
- Expanded README with client setup instructions and tool reference table.

### Changed

- Tool schema construction refactored for reuse and consistency.
- Removed folder thumbnail tool and asset download support.

## [0.1.1] - 2026-02-08

### Added

- Badges and NOTICE file.
- GitHub Actions CI and cargo-dist release workflow.
- README improvements and client setup instructions.

### Changed

- Remote repository renamed to `pcli2-mcp`.

## [0.1.0] - 2026-02-08

### Added

- MCP-over-HTTP server with JSON-RPC 2.0 interface.
- `pcli2` tool for listing folders or assets.
- `pcli2_geometric_match` tool for asset similarity search.

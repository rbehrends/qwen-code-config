# Qwen Code Config

Qwen Code Config is a desktop settings editor for Qwen Code settings files. It is meant for editing `~/.qwen/settings.json` without hand-editing JSON every time you want to add a model, change an API key, or adjust global options. The application presents a curated interface for the settings people tend to change most often, while preserving unrelated fields in the file when it saves.

The editor is intended for the [Qwen Code CLI](https://github.com/QwenLM/qwen-code). Qwen Code itself reads settings from a JSON settings file. This application helps users modify settings, especially adding and removing models, without manually editing the JSON file.

At the center of the editor is model management. The application can read configured models from the `openai` and `anthropic` parts of the model settings, display them in editable form, and write them back to the original settings file. It also provides editing of fallback environment variables (intended as a convenient way to store API keys), a short list of important options, and a preview of the resulting JSON.

# Build

This project is a Tauri 2 application with a Rust backend and a plain HTML, CSS, and JavaScript frontend. There is no separate frontend build step. The frontend assets are served directly from `frontend/`, so building the application is mainly a Rust and Tauri workflow.

## macOS

On macOS, install the Rust toolchain and the Tauri CLI first. The repository includes a `Justfile`, so the quickest way to build a release version of the application is:

```sh
just build
```

The same build can also be run directly with Cargo:

```sh
cargo tauri build --bundles app
```

The final macOS app bundle is written to `target/release/bundle/macos/Qwen Code Config.app`.

For local development on macOS, run:

```sh
just run
```

or:

```sh
cargo tauri dev
```

## Linux

On Linux, install the Rust toolchain, the Tauri CLI, and the system libraries required by Tauri on your distribution. You can find the current Linux prerequisites, including distribution-specific package names, at <https://v2.tauri.app/start/prerequisites/>. In practice, that means the WebKitGTK and related GTK development packages that Tauri depends on. After those prerequisites are in place, build the application with:

```sh
just build
```

or:

```sh
cargo tauri build
```

The release executable is written to `target/release/qwen-code-config`.

For local development on Linux, use:

```sh
just run
```

or:

```sh
cargo tauri dev
```

## Windows

Windows is not supported at this stage.

# Usage

The application opens a Qwen Code settings file and displays it in the UI. If the file does not exist yet, the editor can start from empty defaults. The main workflow is to edit models, environment variables, or both in their respective sections and save them back to the settings file. The current save path is shown in the header and can be modified in the `File` section.

The header also includes density and theme selectors, plus `Reload`, `Save As`, and `Save` actions. Density and theme are persisted as local UI state. On macOS and Linux, the native app menu has the same save, reload, close-window, theme, and density controls along with standard keyboard shortcuts.

Settings files can be opened by choosing a path in the `File` section, by using the native file-open integration where supported, or by dragging a JSON settings file onto the window.

The `Models` section is the main part of the editor. You can fetch model catalogs from built-in provider presets, add manual model entries when a catalog is unavailable or incomplete, remove models you no longer want, and reorder models within their protocol categories. Models can be moved with the arrow buttons on each row or by dragging them within the same protocol list. Each row can also be marked as the default model. The editor currently supports `openai` and `anthropic` protocols. Configurations for other protocols are preserved in the raw JSON but are not exposed through the model editor.

Built-in provider presets currently include OpenRouter, OpenCode Go, OpenCode Zen, Kilo Code, NVIDIA, Ollama, Ollama Cloud, and LM Studio.

The `Models` section also includes a `Fast Model` selector. This controls Qwen Code's `fastModel` setting, can inherit from the main model, and warns when the saved fast-model value does not match the configured structured-editor model list.

Selecting a model opens the model configuration dialog. From there you can change the model ID, display name, base URL, environment variable key, context window size, temperature, `top_p`, maximum token limit, reasoning mode, and reasoning effort. Anthropic entries also expose a reasoning budget token field. The application preserves provider-specific fields that it does not handle itself.

The `Environment` section allows editing of environment variables stored in the settings file under `env`. This is the place to enter API key variable names and values if you want Qwen Code to read them from `settings.json`. The environment variables can also be set directly when invoking Qwen Code from a shell. The Qwen Code settings editor treats the `env` object as a fallback store with environment variables set in the shell taking priority, but it is useful when you want the settings file to remain self-contained.

The `Options` section exposes some major top-level settings that are of particular relevance for users. At present that includes usage statistics, telemetry, and automatic updates.

The `JSON View` section shows a preview of the JSON file that will be written. It is useful for checking the exact data that will be saved to disk. The preview is updated continuously as you edit, and environment variable values are masked in the preview to avoid accidentally exposing API keys. This view is for inspection rather than raw editing.

# Backups

When the application overwrites an existing settings file, it first creates a backup copy. On macOS, these backups are stored under `~/Library/Application Support/Qwen Code Config/backups`. On Linux, they are stored under `$XDG_STATE_HOME/qwen-code-config/backups` when `XDG_STATE_HOME` is set, or under `~/.local/state/qwen-code-config/backups` otherwise.

Backups are grouped by their source path. For settings files inside a `.qwen` directory, backups are grouped by the parent directory that contains `.qwen`, so related files under the same Qwen config root share a backup folder. Each save creates a `*-before-save.json` file named with a human-readable local timestamp in `YYYYMMDD-HHMMSS-mmm` format.

Backup retention is managed automatically per backup folder. The application keeps at least the newest 50 backup files and also keeps any backups from the last 7 days, whichever results in more files being retained. Older backups outside that window are pruned after a successful save. If cleanup fails, the save still succeeds and the existing backups are left in place.

# License

Unless otherwise noted, this repository is licensed under the MIT License. See `LICENSE.txt` for the full license text.

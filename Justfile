[private]
list:
    just --list

# Build Qwen Code Config as a standalone app (release mode)
[macos]
build *args="":
    cargo tauri build --bundles app {{args}}

# Build Qwen Code Config as a standalone app (release mode)
[linux]
build *args="":
    cargo tauri build {{args}}

# Build Qwen Code Config as an AppImage (release mode)
[linux]
build-appimage *args="":
    cargo tauri build --bundles appimage {{args}}

# Build Qwen Code Config as a Debian package (release mode)
[linux]
build-deb *args="":
    cargo tauri build --bundles deb {{args}}

# Build Qwen Code Config as an RPM package (release mode)
[linux]
build-rpm *args="":
    cargo tauri build --bundles rpm {{args}}

# Compile and run Qwen Code Config directly (debug mode)
run *args="":
    cargo tauri dev {{args}}

# Run tests
test *args="":
    cargo test {{args}}

# Install Tauri tooling
install-tauri:
    cargo install tauri-cli

alias dev := run

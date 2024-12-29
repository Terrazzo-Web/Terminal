# Terrazzo PTY

PTY emulator based on <https://docs.rs/pty-process>.

I had some trouble making it work on MacOS:

See `RawPty::open()`
> Can't use CLOEXEC here because it's linux-specific

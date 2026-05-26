# blink

Move any macOS window by holding left Option + left mouse button.

## Building

To build blink, you need to have [Cargo](https://doc.rust-lang.org/cargo/) installed.

```shell
# Install cargo-bundle
$ cargo install cargo-bundle

# Build the release bundle
$ cargo bundle --release
   Compiling blink v0.1.0
    Finished `release` profile [optimized] target(s)
    Bundling Blink.app
    Finished 1 bundle at:
        target/release/bundle/osx/Blink.app

# Move the app to the Applications folder
$ mv target/release/bundle/osx/Blink.app /Applications/
```

# rzq

`rzq` is a small, fast, colourful front end for the Rizin command-line tools. It gives common binary-analysis tasks memorable names while leaving the actual parsing, hashing, and expression evaluation to Rizin.

The wrapper is written in dependency-free Rust. It starts quickly, streams tool output instead of buffering it, and includes its own bounded-memory hexdump.

## Command reference

| Command | Underlying operation |
|---|---|
| `rzq analyze FILE` | Binary info, per-section MD5, and MD5/SHA-1/SHA-256 |
| `rzq info FILE` | `rz-bin -I FILE` |
| `rzq sections FILE` | `rz-bin -K md5 -S FILE` |
| `rzq sections FILE --hash sha256` | `rz-bin -K sha256 -S FILE` |
| `rzq hash md5 FILE` | `rz-hash -a md5 FILE` |
| `rzq hash md5,sha256 FILE` | Multiple Rizin hash algorithms |
| `rzq hashes FILE` | MD5, SHA-1, and SHA-256 |
| `rzq b64 encode TEXT` | `rz-ax -E TEXT` |
| `rzq b64 decode TEXT` | `rz-ax -D TEXT` |
| `rzq int EXPR` | Evaluate an expression and print decimal |
| `rzq hex EXPR` | Evaluate an expression and print hexadecimal |
| `rzq num EXPR` | Print decimal and hexadecimal |
| `rzq hexdump FILE` | First 256 bytes as a colour hexdump |
| `rzq hexdump FILE -n N -s OFFSET` | Arbitrary byte count and start offset |
| `rzq headers FILE` | Header fields |
| `rzq imports FILE` / `exports FILE` | Imported/exported symbols |
| `rzq symbols FILE` | Symbols |
| `rzq libs FILE` | Linked libraries |
| `rzq entry FILE` | Entrypoint |
| `rzq strings FILE [--all]` | Data-section or raw-file strings |
| `rzq all FILE` | `rz-bin -g FILE` |

Run `rzq --help` for the complete reference, aliases, options, and examples.

### Standard input

Hashing, Base64 conversion, and hexdumping support standard input:

```console
$ printf hello | rzq hash md5
$ printf hello | rzq b64 encode
$ cat firmware.bin | rzq hexdump - --bytes 4K
```

Hexdump sizes accept decimal, hexadecimal, `K`, `M`, `G`, or `all`:

```console
$ rzq hd firmware.bin --offset 0x100 --bytes 1K --width 16
$ rzq hd firmware.bin --bytes all
```

Colour is on by default, including when output is redirected. Native Rizin colours are preserved, while otherwise uncoloured text receives an `rzq` fallback colour. Use `--no-color` or the [`NO_COLOR`](https://no-color.org/) environment variable when plain output is needed.

## Install with Nix

The flake explicitly supports `x86_64-linux` and `aarch64-linux` and does **not** use `flake-utils`. Rizin is placed on `rzq`'s runtime `PATH` by the Nix wrapper.

```console
$ nix run github:ezradotexe/rzq -- --help
$ nix profile install github:ezradotexe/rzq
```

From a checkout:

```console
$ nix build
$ ./result/bin/rzq --help
```

A non-flake expression is also included:

```console
$ nix-build
```

## Build without Nix

Install Rizin so that `rz-bin`, `rz-hash`, and `rz-ax` are on `PATH`, then:

```console
$ cargo build --release
$ install -Dm755 target/release/rzq ~/.local/bin/rzq
```

The minimum supported platforms are 64-bit Linux on AMD64 and AArch64. The Rust source has no third-party crate dependencies.

## Development

```console
$ nix develop
$ cargo fmt --check
$ cargo clippy --all-targets -- -D warnings
$ cargo test
```

Command semantics follow the official [Rizin Reference Book](https://book.rizin.re/), particularly its [`rz-bin`](https://book.rizin.re/src/tools/rz-bin/intro.html), [`rz-hash`](https://book.rizin.re/src/tools/rz-hash/rz-hash_tool.html), and [`rz-ax`](https://book.rizin.re/src/tools/rz-ax/intro.html) sections.

## License

MIT

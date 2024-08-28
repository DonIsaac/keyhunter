# KeyHunter
[![CI Check](https://github.com/DonIsaac/keyhunter/actions/workflows/pipeline.yml/badge.svg)](https://github.com/DonIsaac/keyhunter/actions/workflows/pipeline.yml)
[![Crates.io Version](https://img.shields.io/crates/v/keyhunter)](https://crates.io/crates/keyhunter)
[![docs.rs](https://img.shields.io/docsrs/keyhunter)](https://docs.rs/keyhunter/)
[![CodSpeed Badge](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/DonIsaac/keyhunter)

Check for leaked API keys and secrets any website's JavaScript.

<p align="center">
  <a href="https://www.loom.com/share/834dacfb279846548978ceee99909a17?sid=a94db1e2-a4cf-4963-908a-703b8fa87b6f" target="_blank">
    <img src="./assets/keyhunter-yc-demo.gif" alt="KeyHunter running on sites of the last 7 YCombinator startups" />
  </a>
  <br />
  <i>KeyHunter running on sites of the last 7 YCombinator batches</i>
</p>

## Installation
You can install KeyHunter as a Crate from [crates.io](https://crates.io/crates/keyhunter):
```sh
cargo install keyhunter
``` 

You can also use it as a library:
```toml
[dependencies]
# 'build-binary' feature is on by default, which isn't useful for library use
keyhunter = { version = "0.2.0", default-features = false }
```

Library docs are available on [docs.rs](https://docs.rs/keyhunter/).

## Usage
> To reproduce the example above, run `make yc`

Provide KeyHunter with a URL to start scanning from. It will visit all pages
on the same domain that URL links to, find all scripts referenced by those
pages, and check them for leaked API keys and secrets.

```sh
keyhunter https://example.com
```

### Authentication

You can include one or more headers in all requests KeyHunter makes with the
`--header` (or `-H`) flag. This means you can include an `Authorization` header
to scan websites that require authentication.

```sh
keyhunter https://example.com -H "Authorization: Bearer <token>"

# Multiple headers
keyhunter https://example.com -H "Cookie: session-cookie=123" -H "x-another-header: foo"
```

This flag follows the same conventions as `curl`'s `-H` flag. 

> For more information and a list of all available arguments, run `keyhunter
> --help`.

### Output Format

Using the `--format <format>` flag, you can specify how KeyHunter should output
its findings.
- `default`: Pretty-printed, human readable output. This is the default format.
- `json`: Print a JSON object for each finding on a separate line. This format
  is really [JSON lines](https://jsonlines.org/).

## Disclaimer

This tool is for educational purposes only. Only use it on websites and/or web
applications that you own or that are owned by an organization that has given
you their explicit consent. Do not use this tool for malicious purposes. Please
read the [LICENSE](LICENSE.md) for more information.

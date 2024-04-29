# KeyHunter
[![CI Check](https://github.com/DonIsaac/keyhunter/actions/workflows/pipeline.yml/badge.svg)](https://github.com/DonIsaac/keyhunter/actions/workflows/pipeline.yml)
[![Crates.io Version](https://img.shields.io/crates/v/keyhunter)](https://crates.io/crates/keyhunter)
[![docs.rs](https://img.shields.io/docsrs/keyhunter)](https://docs.rs/keyhunter/)
[![CodSpeed Badge](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/DonIsaac/keyhunter)

Check for leaked API keys and secrets on public websites.

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
cargo install keyhunter --all-features
``` 

You can also use it as a library:
```toml
[dependencies]
keyhunter = "0.1.1"
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


For more information, run `keyhunter --help`.

## Disclaimer

This tool is for educational purposes only. Only use it on websites and/or web
applications that you own or that are owned by an organization that has given
you their explicit consent. Do not use this tool for malicious purposes. Please
read the [LICENSE](LICENSE.md) for more information.

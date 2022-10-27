# reytan

reytan extracts media (videos, music, playlists, channels, albums, ...) from some places on the internet.

## how the reytan doin?

- **done status: basic higher-level API done now**, not usable for lists yet
- **stability status: absolutely unstable APIs**
- release status: no

## why reytan?

- reytan will extract metadata even if it cannot extract playback
- extractor hints for what you want to extract (e.g. if playback extraction is set to None, extractor might not try workarounds to extract playback)
- written in Rust - compiles to fast, platform-native binaries, opens possibilities for bindings to other languages

## supported services

- YouTube, incl. JS signatures, agegate workarounds
- Bandcamp (website mp3 playback only)
- Soundcloud

## issues, feature requests

check out the [issues on the codeberg repository](https://codeberg.org/transcast/reytan/issues)

## donate

you can donate to the maintainer on [ko-fi](https://ko-fi.com/selfisekai) or [github sponsors](https://github.com/sponsors/selfisekai)

## copyright

copyright (c) 2022 lauren n. liberda

usage allowed under the conditions of Apache-2.0 license (license text included in the LICENSE file)

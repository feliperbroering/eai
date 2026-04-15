# Changelog

## [0.13.0](https://github.com/feliperbroering/eai/compare/v0.12.0...v0.13.0) (2026-04-15)


### Features

* add clear-cache subcommand, comprehensive README, and improved --help descriptions ([2fb1d0d](https://github.com/feliperbroering/eai/commit/2fb1d0d778e215724d74c6caf0c9e6cecbeb0dc0))

## [0.12.0](https://github.com/feliperbroering/eai/compare/v0.11.0...v0.12.0) (2026-04-15)


### Features

* add caching, shell integration, aliases, script/recipe mode, streaming animation, and more ([f2a64ab](https://github.com/feliperbroering/eai/commit/f2a64ab50048cb4daa86baf0825cb8f0000f027c))

## [0.11.0](https://github.com/feliperbroering/eai/compare/v0.10.0...v0.11.0) (2026-04-15)


### Features

* OS-aware command accuracy — platform tldr, flag correction, enhanced prompts ([8a6e134](https://github.com/feliperbroering/eai/commit/8a6e1343c8b0a37440325125b7377c17992baa14))

## [0.10.0](https://github.com/feliperbroering/eai/compare/v0.9.1...v0.10.0) (2026-04-15)


### Features

* embed tldr-pages for instant tool documentation lookup ([8649947](https://github.com/feliperbroering/eai/commit/8649947abf37faa734a06bb917cc39c10e50ed90))


### Bug Fixes

* disable default-tls for build-dep reqwest to avoid openssl in CI ([27bfec7](https://github.com/feliperbroering/eai/commit/27bfec7c370ae1009badbddbf1e67ba63f946eed))

## [0.9.1](https://github.com/feliperbroering/eai/compare/v0.9.0...v0.9.1) (2026-04-15)


### Bug Fixes

* handle empty/whitespace prompts and command box in non-TTY ([2e3c2f4](https://github.com/feliperbroering/eai/commit/2e3c2f4410bd7424553ff71a26c28a35418d8daa))

## [0.9.0](https://github.com/feliperbroering/eai/compare/v0.8.1...v0.9.0) (2026-04-15)


### Features

* add homebrew tap publishing ([8fa308c](https://github.com/feliperbroering/eai/commit/8fa308cd57c6ccf743ab3a05ae1baecc0fb5b569))

## [0.8.1](https://github.com/feliperbroering/eai/compare/v0.8.0...v0.8.1) (2026-04-14)


### Bug Fixes

* allow winget job to fail without blocking release ([e66a511](https://github.com/feliperbroering/eai/commit/e66a51143657acba4ca797c04676c31e9aa09ccf))

## [0.8.0](https://github.com/feliperbroering/eai/compare/v0.7.0...v0.8.0) (2026-04-14)


### Features

* add winget support and show help on empty prompt ([e52928c](https://github.com/feliperbroering/eai/commit/e52928cdcc3011b7cccd7f41a1d3f4077437f348))

## [0.7.0](https://github.com/feliperbroering/eai/compare/v0.6.0...v0.7.0) (2026-04-13)


### Features

* add Gemini backend, fix edit-then-run input leak, block interactive commands ([555da84](https://github.com/feliperbroering/eai/commit/555da84ccf598c00b42cd7e1b58aba009da057fe))


### Bug Fixes

* add Windows no-op for flush_stdin to fix cross-platform build ([1f455e0](https://github.com/feliperbroering/eai/commit/1f455e0a0c170f09e3e016d8d6470d92f68b8baf))

## [0.6.0](https://github.com/feliperbroering/eai/compare/v0.5.2...v0.6.0) (2026-04-13)


### Features

* auto-update detection with interactive upgrade prompt ([cadaf01](https://github.com/feliperbroering/eai/commit/cadaf01964f3f554ea2f6d95c915c483002bef9f))


### Bug Fixes

* remove useless format! to satisfy clippy ([83411b4](https://github.com/feliperbroering/eai/commit/83411b4b889cd7d74b0d0662970c1a6609cba6ea))

## [0.5.2](https://github.com/feliperbroering/eai/compare/v0.5.1...v0.5.2) (2026-04-12)


### Bug Fixes

* **installer:** prioritize release bin in Windows PATH ([fdfaeb6](https://github.com/feliperbroering/eai/commit/fdfaeb6653a5bf5af41a1aaed3c2b98e46aa1bc9))

## [0.5.1](https://github.com/feliperbroering/eai/compare/v0.5.0...v0.5.1) (2026-04-12)


### Bug Fixes

* **windows:** sanitize command output before console write ([72c3a15](https://github.com/feliperbroering/eai/commit/72c3a1586b82d453c9a267f80047456b4bfb3c4a))

## [0.5.0](https://github.com/feliperbroering/eai/compare/v0.4.0...v0.5.0) (2026-04-12)


### Features

* initial release — natural language to shell commands ([7d8fcb2](https://github.com/feliperbroering/eai/commit/7d8fcb2fa9aea78d315473d13b2ad216da2d8a42))
* **installer:** ship windows binary installer from releases ([cbb6773](https://github.com/feliperbroering/eai/commit/cbb6773c2ff70357281c81e3d15baaddcc780cf0))
* tool discovery, Tavily search, setup UX improvements ([f5e499f](https://github.com/feliperbroering/eai/commit/f5e499f2ae95962f27979e91173b70bbe6a50ac2))


### Bug Fixes

* align install.sh asset names with release artifacts ([6ff16e1](https://github.com/feliperbroering/eai/commit/6ff16e147974923ada707fcbd61b38a7de1200de))
* **ci:** apply rustfmt adjustments for windows changes ([9bda141](https://github.com/feliperbroering/eai/commit/9bda1417d4f5950657b844d410a7f97388ce1cad))
* clippy print_literal warnings in setup.rs ([90c1446](https://github.com/feliperbroering/eai/commit/90c1446e945a34be009cc9371beb428d7409b4fc))
* discovery UX improvements ([3da35fc](https://github.com/feliperbroering/eai/commit/3da35fcf0d289d47f1b20f4efb355dd8db6380bb))
* indent command output and dim stderr in UI ([a011b50](https://github.com/feliperbroering/eai/commit/a011b50a667e3ccdacabf617c16b90a6196ec553))
* **installer:** resolve PowerShell parsing issue on PATH update ([2d46ffb](https://github.com/feliperbroering/eai/commit/2d46ffb468e84486d77ef57078ffdd5e9c1267fc))
* **release:** upload renamed artifacts reliably ([a2991bd](https://github.com/feliperbroering/eai/commit/a2991bdd1acb40279fc9c2afb5b5f3fb35d83d95))
* static musl binaries for Linux + improved tool discovery ([36d3256](https://github.com/feliperbroering/eai/commit/36d3256718cfba571766487924b08de4f104c374))
* **windows:** add native shell support and robust e2e coverage ([b138b42](https://github.com/feliperbroering/eai/commit/b138b429b4c7d2ca5aead69dbcb7eb44fa00ab2a))

## [0.4.0](https://github.com/feliperbroering/eai/compare/v0.3.2...v0.4.0) (2026-04-12)


### Features

* initial release — natural language to shell commands ([7d8fcb2](https://github.com/feliperbroering/eai/commit/7d8fcb2fa9aea78d315473d13b2ad216da2d8a42))
* **installer:** ship windows binary installer from releases ([cbb6773](https://github.com/feliperbroering/eai/commit/cbb6773c2ff70357281c81e3d15baaddcc780cf0))
* tool discovery, Tavily search, setup UX improvements ([f5e499f](https://github.com/feliperbroering/eai/commit/f5e499f2ae95962f27979e91173b70bbe6a50ac2))


### Bug Fixes

* align install.sh asset names with release artifacts ([6ff16e1](https://github.com/feliperbroering/eai/commit/6ff16e147974923ada707fcbd61b38a7de1200de))
* **ci:** apply rustfmt adjustments for windows changes ([9bda141](https://github.com/feliperbroering/eai/commit/9bda1417d4f5950657b844d410a7f97388ce1cad))
* clippy print_literal warnings in setup.rs ([90c1446](https://github.com/feliperbroering/eai/commit/90c1446e945a34be009cc9371beb428d7409b4fc))
* discovery UX improvements ([3da35fc](https://github.com/feliperbroering/eai/commit/3da35fcf0d289d47f1b20f4efb355dd8db6380bb))
* indent command output and dim stderr in UI ([a011b50](https://github.com/feliperbroering/eai/commit/a011b50a667e3ccdacabf617c16b90a6196ec553))
* **installer:** resolve PowerShell parsing issue on PATH update ([2d46ffb](https://github.com/feliperbroering/eai/commit/2d46ffb468e84486d77ef57078ffdd5e9c1267fc))
* **release:** upload renamed artifacts reliably ([a2991bd](https://github.com/feliperbroering/eai/commit/a2991bdd1acb40279fc9c2afb5b5f3fb35d83d95))
* static musl binaries for Linux + improved tool discovery ([36d3256](https://github.com/feliperbroering/eai/commit/36d3256718cfba571766487924b08de4f104c374))
* **windows:** add native shell support and robust e2e coverage ([b138b42](https://github.com/feliperbroering/eai/commit/b138b429b4c7d2ca5aead69dbcb7eb44fa00ab2a))

## [0.3.2](https://github.com/feliperbroering/eai/compare/v0.3.1...v0.3.2) (2026-04-12)


### Bug Fixes

* **installer:** resolve PowerShell parsing issue on PATH update ([2d46ffb](https://github.com/feliperbroering/eai/commit/2d46ffb468e84486d77ef57078ffdd5e9c1267fc))

## [0.3.1](https://github.com/feliperbroering/eai/compare/v0.3.0...v0.3.1) (2026-04-11)


### Bug Fixes

* **release:** upload renamed artifacts reliably ([a2991bd](https://github.com/feliperbroering/eai/commit/a2991bdd1acb40279fc9c2afb5b5f3fb35d83d95))

## [0.3.0](https://github.com/feliperbroering/eai/compare/v0.2.4...v0.3.0) (2026-04-11)


### Features

* **installer:** ship windows binary installer from releases ([cbb6773](https://github.com/feliperbroering/eai/commit/cbb6773c2ff70357281c81e3d15baaddcc780cf0))

## [0.2.4](https://github.com/feliperbroering/eai/compare/v0.2.3...v0.2.4) (2026-04-11)


### Bug Fixes

* **ci:** apply rustfmt adjustments for windows changes ([9bda141](https://github.com/feliperbroering/eai/commit/9bda1417d4f5950657b844d410a7f97388ce1cad))
* **windows:** add native shell support and robust e2e coverage ([b138b42](https://github.com/feliperbroering/eai/commit/b138b429b4c7d2ca5aead69dbcb7eb44fa00ab2a))

## [0.2.3](https://github.com/feliperbroering/eai/compare/v0.2.2...v0.2.3) (2026-04-10)


### Bug Fixes

* indent command output and dim stderr in UI ([a011b50](https://github.com/feliperbroering/eai/commit/a011b50a667e3ccdacabf617c16b90a6196ec553))

## [0.2.2](https://github.com/feliperbroering/eai/compare/v0.2.1...v0.2.2) (2026-04-10)


### Bug Fixes

* static musl binaries for Linux + improved tool discovery ([36d3256](https://github.com/feliperbroering/eai/commit/36d3256718cfba571766487924b08de4f104c374))

## [0.2.1](https://github.com/feliperbroering/eai/compare/v0.2.0...v0.2.1) (2026-04-10)


### Bug Fixes

* discovery UX improvements ([3da35fc](https://github.com/feliperbroering/eai/commit/3da35fcf0d289d47f1b20f4efb355dd8db6380bb))

## [0.2.0](https://github.com/feliperbroering/eai/compare/v0.1.0...v0.2.0) (2026-04-10)


### Features

* tool discovery, Tavily search, setup UX improvements ([f5e499f](https://github.com/feliperbroering/eai/commit/f5e499f2ae95962f27979e91173b70bbe6a50ac2))


### Bug Fixes

* align install.sh asset names with release artifacts ([6ff16e1](https://github.com/feliperbroering/eai/commit/6ff16e147974923ada707fcbd61b38a7de1200de))
* clippy print_literal warnings in setup.rs ([90c1446](https://github.com/feliperbroering/eai/commit/90c1446e945a34be009cc9371beb428d7409b4fc))

## 0.1.0 (2026-04-10)


### Features

* initial release — natural language to shell commands ([7d8fcb2](https://github.com/feliperbroering/eai/commit/7d8fcb2fa9aea78d315473d13b2ad216da2d8a42))

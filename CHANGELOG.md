# Changelog

## [0.1.1](https://github.com/raiderrobert/diecut/compare/diecut-v0.1.0...diecut-v0.1.1) (2026-02-13)


### Features

* add diecut list command and user config ([1408077](https://github.com/raiderrobert/diecut/commit/14080774ece6f22ada966e364e0711282597d443))
* add diecut list command and user config with custom abbreviations ([1bbc1d0](https://github.com/raiderrobert/diecut/commit/1bbc1d0b3a473e74db24b79fa41325e9144406ff))
* add diff3-style three-way conflict output in .rej files ([39e87f8](https://github.com/raiderrobert/diecut/commit/39e87f8a75ced98e9e7ff3f3e815e57764b2ffd9))
* add install script and update install instructions ([d2a99f3](https://github.com/raiderrobert/diecut/commit/d2a99f3979d9c96b55dffcb4b885c7acb03e56fb))
* add PR title conventional commit check ([8da331c](https://github.com/raiderrobert/diecut/commit/8da331cd440394333b1c0e2e38ee416a0e832dfe))
* add release-please configuration and workflow ([fd9c08a](https://github.com/raiderrobert/diecut/commit/fd9c08a922eea1efab2b58792b90a14c9eb4e8ad))
* diff3-style three-way conflict output in .rej files ([4fd4e3b](https://github.com/raiderrobert/diecut/commit/4fd4e3b4f8f756931448487a87cc76fd1e8c82d2))
* distribution hardening ([8e82e6b](https://github.com/raiderrobert/diecut/commit/8e82e6b7946b1656f05b2c8b6a00a1cca201fc95))
* distribution hardening — SHA pinning, hook trust, ready command, example templates ([0f1168a](https://github.com/raiderrobert/diecut/commit/0f1168a98c42b91a2bfb8a0b9be02e82d295c92a))
* git clone via gix for remote template sources ([4eb4125](https://github.com/raiderrobert/diecut/commit/4eb41251f3cbccc5cb8836cedd5383d10399ac0a))
* git clone via gix for remote template sources ([bba6e36](https://github.com/raiderrobert/diecut/commit/bba6e36a5ce67806d5821ce92b750ea15ef79e84))
* git URL detection, Rhai hook system, and template validation ([3cd13e8](https://github.com/raiderrobert/diecut/commit/3cd13e8ca54616d77273858b9e5dd1bbdcddc681))
* implement diecut update command with three-way merge ([3a3b8cb](https://github.com/raiderrobert/diecut/commit/3a3b8cb181cc924f7aad9804e8611d8536838d17))
* implement diecut update command with three-way merge ([0e58eb3](https://github.com/raiderrobert/diecut/commit/0e58eb31b27a65724ad83aaff2a8d63dce4764c8))
* M2/M3 catchup — git URLs, Rhai hooks, check command, repo scaffolding ([107b889](https://github.com/raiderrobert/diecut/commit/107b889c697981fe55a207afcb06683fc2e5fd66))
* support in-place migration with automatic backup ([e5f0de7](https://github.com/raiderrobert/diecut/commit/e5f0de7f0beab88d5a68ea4c608ec1f3c9235bb5))
* support in-place migration with automatic backup ([4b586b8](https://github.com/raiderrobert/diecut/commit/4b586b8542a0df9ce34d7875527f57e3d76e3dbf))
* template adapter system with cookiecutter compatibility ([44e7576](https://github.com/raiderrobert/diecut/commit/44e7576c8977dd6db3723a2a5b99a611edb35c14))
* template caching for git-sourced templates ([93218a1](https://github.com/raiderrobert/diecut/commit/93218a1fe4e3aa116bfb2af81c1414e58563e327))
* template caching for git-sourced templates ([b52fcc8](https://github.com/raiderrobert/diecut/commit/b52fcc884591df1ff3e10a9198b608b61bb6ca65))


### Bug Fixes

* add cargo-workspace plugin to release-please config ([e181218](https://github.com/raiderrobert/diecut/commit/e181218d8a9e27e6e0bd526ac78190b154dfffcd))
* add cargo-workspace plugin to release-please config ([4267588](https://github.com/raiderrobert/diecut/commit/4267588abb4f1f05b48a4d7a753c9431788d0770))
* cache module improvements (timestamps, URL matching, configurability) ([8a029c1](https://github.com/raiderrobert/diecut/commit/8a029c1c85ebafa4f5b3044f56a7894ff70d9fd0))
* configure release-please for Rust workspace per upstream pattern ([77c43c8](https://github.com/raiderrobert/diecut/commit/77c43c88fb16873642aef3cce012a25aeeae572f))
* configure release-please for Rust workspace per upstream pattern ([4b61088](https://github.com/raiderrobert/diecut/commit/4b6108872df07a3bc7d52ae59637ff41a66d64c6))
* enable HTTPS transport for gix git cloning ([dbadba0](https://github.com/raiderrobert/diecut/commit/dbadba0df20c6fedde4063dbb1c83858119d8e96))
* gracefully handle unsupported Jinja2 syntax in foreign templates ([9192ce3](https://github.com/raiderrobert/diecut/commit/9192ce36e11508b86d43ed42d134b96074302d6e))
* include file path in template render error messages ([5d87ddd](https://github.com/raiderrobert/diecut/commit/5d87ddd7f49fc42a9af942bb0fb8a919c22a7e1c))
* include file path in template render error messages ([761651b](https://github.com/raiderrobert/diecut/commit/761651beca76d2c55d1daaf4afeb9790eaac828b))
* proper tempdir lifecycle in clone/cache flow ([1b3ab5c](https://github.com/raiderrobert/diecut/commit/1b3ab5cddd093ec88de8a8a4caa78901069a8a23))
* read only first 8KB for binary file detection ([9fc4ed8](https://github.com/raiderrobert/diecut/commit/9fc4ed8253301c32d981f3ccf7ae6821564e4864))
* read only first 8KB for binary file detection ([f52548a](https://github.com/raiderrobert/diecut/commit/f52548a3214d524bc4bcae6e3da35809dd78842c))
* remove cargo-workspace plugin and restore version.workspace ([1d074c9](https://github.com/raiderrobert/diecut/commit/1d074c9b3c8529f0ab8f7660f01617fac69df97e))
* remove cargo-workspace plugin and restore version.workspace ([5d1c7c6](https://github.com/raiderrobert/diecut/commit/5d1c7c609d1ceb7ccd09f1a5e9cb7993876f8b80))
* removed unneeded plan ([1f4bee9](https://github.com/raiderrobert/diecut/commit/1f4bee9a86ecccf1c9d483e7263749d999a5b5b5))
* resolve cargo fmt formatting issues ([2585f9c](https://github.com/raiderrobert/diecut/commit/2585f9c35756b06430aa8af06f5796f49ae1a5fb))
* resolve cargo fmt formatting issues ([e529115](https://github.com/raiderrobert/diecut/commit/e529115c8e19a308cf1c56225cdff959d62e0369))
* resolve clippy cmp_owned warnings in update tests ([8af793b](https://github.com/raiderrobert/diecut/commit/8af793b66e77e0591ed2bce6c32685e5fc05efcc))
* resolve clippy warnings for CI compatibility ([a7e555e](https://github.com/raiderrobert/diecut/commit/a7e555ebe23ab744675099c7bd3b8207cf0b34e1))
* sandbox Rhai hook engine to prevent filesystem access ([d2092e2](https://github.com/raiderrobert/diecut/commit/d2092e21dd6884eefcf178967e4793f6057ec8ba))
* sandbox Rhai hook engine to prevent filesystem access ([6b2251e](https://github.com/raiderrobert/diecut/commit/6b2251ea38b2dbe7c8ad2f1ddbf0047c49bea8a5))
* use content_inspector for BOM-aware binary detection ([d995178](https://github.com/raiderrobert/diecut/commit/d9951780a642e68edf3602cde586811268dba832))
* use explicit versions in subcrate Cargo.toml for release-please ([52df58f](https://github.com/raiderrobert/diecut/commit/52df58f60750b1df9fee0dc323cf56a69999d2a8))
* use explicit versions in subcrate Cargo.toml for release-please ([e975299](https://github.com/raiderrobert/diecut/commit/e97529959f8ceee0179c015d1ffb0156d88e50ab))
* use OS-level advisory locks (fs4) for cache concurrency ([072072c](https://github.com/raiderrobert/diecut/commit/072072c9728c795865c2b68f4d4d187bbf9f03cc))
* use OS-level advisory locks (fs4) for cache concurrency ([a0e3580](https://github.com/raiderrobert/diecut/commit/a0e3580f6ad13984e88a38579882630938b1358c))
* use rename-swap for in-place migration instead of clear-and-copy ([c061f60](https://github.com/raiderrobert/diecut/commit/c061f609be8df344215a25814a82d3d00c2c7410))


### Miscellaneous

* add .worktrees/ to gitignore ([3722144](https://github.com/raiderrobert/diecut/commit/37221444f4756713963374f7dd233fe8112e22f9))
* add CI workflows, beads issue tracking, and repo scaffolding ([3707e9b](https://github.com/raiderrobert/diecut/commit/3707e9bf1cbf56e4e4242350a520a19541530388))
* add justfile for common dev tasks ([337ac22](https://github.com/raiderrobert/diecut/commit/337ac221de2cd0d9dcb3ca3e1d91cc3853f8ecc9))
* add MIT license file ([7d46744](https://github.com/raiderrobert/diecut/commit/7d467444f701973364cc19a5c54d5f9a2dae8175))
* add release-please configuration ([c752cd6](https://github.com/raiderrobert/diecut/commit/c752cd6996c2474b3614228fc63af9a98cac077d))
* clean up links ([b5f8684](https://github.com/raiderrobert/diecut/commit/b5f86845826a84debcd4d7fe513a6ccc505779aa))
* fix links in docs ([2fb6e1a](https://github.com/raiderrobert/diecut/commit/2fb6e1a68b5341f32d96575827a899936924484f))
* gitignore .research directory ([19da008](https://github.com/raiderrobert/diecut/commit/19da008c05afd24a24fad0946b8f8c58f042059e))
* link to license ([6be9147](https://github.com/raiderrobert/diecut/commit/6be91470c0638ace4a93b85e55fe7dd9b2af1197))
* release main ([18b4195](https://github.com/raiderrobert/diecut/commit/18b41952a591804fafea0924755b7559ffae95f5))
* release main ([ef450ad](https://github.com/raiderrobert/diecut/commit/ef450ad1af75aecc44a29dd8e08eea0c487bec7b))
* remove docs-redesign plan files ([c088220](https://github.com/raiderrobert/diecut/commit/c088220ce91561da6383417bcf1b50efda5c0d26))
* remove unused indicatif dependency ([3486a78](https://github.com/raiderrobert/diecut/commit/3486a78908e1a336adc576335e5f2e372064265b))
* remove unused indicatif dependency ([b19de48](https://github.com/raiderrobert/diecut/commit/b19de48d891200438c8f8504f6ce30348c6aee34))
* remove verbose comments and simplify code ([027bb15](https://github.com/raiderrobert/diecut/commit/027bb158ed5ad48cefff362c622d50b4da4fada3))
* remove verbose comments and simplify code across crates ([e5c5da9](https://github.com/raiderrobert/diecut/commit/e5c5da968f44b0becad28c739469b28f55310dcc))


### Documentation

* add astro-cf-template migration design ([9f0928a](https://github.com/raiderrobert/diecut/commit/9f0928a533d5ea9580e54362f3bbce4fd49cef32))
* add cookiecutter migration guide ([5c3f844](https://github.com/raiderrobert/diecut/commit/5c3f844463a71dc38b6d435e9a36b3a11e2d86d8))
* add creating-templates guide (flagship page) ([8790a2c](https://github.com/raiderrobert/diecut/commit/8790a2c6d723f68e2a94c50392d8540020e3108c))
* add design for replacing gix with system git for cloning ([b02f4a6](https://github.com/raiderrobert/diecut/commit/b02f4a6c28ddcdd0074bb972d5ba659f5fa27a90))
* add diecut.toml config reference ([3eb017d](https://github.com/raiderrobert/diecut/commit/3eb017d466d9a225509e1e9df1c72ba867281d13))
* add documentation redesign design plan ([cbe9db7](https://github.com/raiderrobert/diecut/commit/cbe9db7f72f8e489e53b77f0584ff4e28f394ec5))
* add documentation redesign implementation plan ([61fb1b6](https://github.com/raiderrobert/diecut/commit/61fb1b6601a574b642c23bec73ca80feedc15f57))
* add hooks reference ([2722050](https://github.com/raiderrobert/diecut/commit/2722050a2fe44a8e22769127f4e83eccbab97214))
* add README and project CLAUDE.md ([02d216f](https://github.com/raiderrobert/diecut/commit/02d216f575ce27c90fbf91b39f1cdd84cf4af2cd))
* add README and project CLAUDE.md ([1806857](https://github.com/raiderrobert/diecut/commit/1806857cad16a4375e1e51c972046f5efeb43b81))
* add Starlight documentation site ([f7ab830](https://github.com/raiderrobert/diecut/commit/f7ab83016d8f2f42ad95fd761169435421a60f86))
* add using-templates guide ([0c33cb8](https://github.com/raiderrobert/diecut/commit/0c33cb8e4d5f44146f166db7f6dcaf8e979861e7))
* expand cookiecutter incompatibilities section ([45be9a6](https://github.com/raiderrobert/diecut/commit/45be9a6ffafca07661a12a7b032d724aafbcb52a))
* expand getting-started with real example and audience fork ([4aad3d0](https://github.com/raiderrobert/diecut/commit/4aad3d030e16c86ca5b28bd82dd6a0fb81f910fe))
* redesign documentation site ([1d8c07a](https://github.com/raiderrobert/diecut/commit/1d8c07a54a2e8db692aa19582716d0e98c69cda2))
* rewrite commands reference with consistent format ([9b5ade7](https://github.com/raiderrobert/diecut/commit/9b5ade782d597f17efa9cd0f22ab0b355b84e86d))
* rewrite landing page with origin story and value props ([6a2b9c6](https://github.com/raiderrobert/diecut/commit/6a2b9c644e68a46d52b13097798427154e3edd9e))
* scaffold new page structure with stubs ([dc08df5](https://github.com/raiderrobert/diecut/commit/dc08df51ff021931f01f5aee27cb3087f528d623))
* trim README to point at docs site ([7b897f7](https://github.com/raiderrobert/diecut/commit/7b897f7c20186f34aa7c80e1d115c8c0657c9180))


### Code Refactoring

* extract write_cache_metadata and place_in_cache helpers ([8907726](https://github.com/raiderrobert/diecut/commit/8907726c21a73766e565132c6fe69eaeea493599))
* extract write_cache_metadata and place_in_cache helpers ([1b9756a](https://github.com/raiderrobert/diecut/commit/1b9756a7edd7c31366aa6372dc26deebb4f18101))
* merge diecut-core and diecut-cli into single crate ([5d6e1b3](https://github.com/raiderrobert/diecut/commit/5d6e1b30305278878001fbf8c93bd27048525289))
* merge diecut-core and diecut-cli into single crate ([aeec33a](https://github.com/raiderrobert/diecut/commit/aeec33afa36a48235d1c27b4413f379e9b041c9d))
* use Option&lt;&str&gt; for commit_sha parameter ([2178c3b](https://github.com/raiderrobert/diecut/commit/2178c3b985bc54c4fe4d2314279227f425a8e293))

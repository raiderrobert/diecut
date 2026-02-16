# Changelog

## [0.3.0](https://github.com/raiderrobert/diecut/compare/v0.2.0...v0.3.0) (2026-02-16)


### Features

* add Codeberg (cb:) abbreviation ([ded1ab3](https://github.com/raiderrobert/diecut/commit/ded1ab36a37286c34e680d7cc6d1778f4da6940c))
* add diecut logo to README and docs ([99a860a](https://github.com/raiderrobert/diecut/commit/99a860a972c1ec14e88949dc2110d5d5d7d98ab8))
* add subpath support for multi-template repos ([ff873d0](https://github.com/raiderrobert/diecut/commit/ff873d0189be91dd5c4fd3a12012fc4ba4887b5f))


### Bug Fixes

* docs cleanup — linked cards, README 404, simplify templates ([15b1143](https://github.com/raiderrobert/diecut/commit/15b11435509703823120d69d2ef428e33e1e261f))
* move favicon to public dir so it's served correctly ([d0f9dac](https://github.com/raiderrobert/diecut/commit/d0f9dac0bc0039913b83e05d54ee2c80f810cdea))
* remove dead toml_value_to_tera function ([944a138](https://github.com/raiderrobert/diecut/commit/944a138735592503a6bcac300f668df82329b3de))


### Miscellaneous

* adopt rstest for parameterized tests ([aa255a7](https://github.com/raiderrobert/diecut/commit/aa255a70e7330647100dbc8e76c766b229b4ddb5))
* change bump-patch-for-minor-pre-major to false ([b17768e](https://github.com/raiderrobert/diecut/commit/b17768ececb5da091fa3f6c04aadc90fc02b8011))
* cleaned up description ([7a29bcb](https://github.com/raiderrobert/diecut/commit/7a29bcb9da690aae5c5459e57a77c70e4eb39633))
* remove Bitbucket (bb:) abbreviation ([77a4cb3](https://github.com/raiderrobert/diecut/commit/77a4cb3897ac4c225a3fb76faf401ccb9edd0410))
* remove Sourcehut (sr:) abbreviation ([3573849](https://github.com/raiderrobert/diecut/commit/357384970408cb1c6ef8d2895fec3daf7891e2b6))
* rename 'Starter templates' to 'Example templates' ([a83bb1c](https://github.com/raiderrobert/diecut/commit/a83bb1c3f1c90cab6d7bef4f356e12f0cc3f15fe))
* replace Bitbucket with Codeberg abbreviation ([f5c294f](https://github.com/raiderrobert/diecut/commit/f5c294f7dc287d508de7de8d4e908d0d23597464))


### Documentation

* cross-link diecut and diecut-templates repos ([c9a486d](https://github.com/raiderrobert/diecut/commit/c9a486d3fd90fed0a6fa92e0c118bc0ec02f170a))
* update for refactor (remove cookiecutter, update, check/ready, rhai) ([efa8c18](https://github.com/raiderrobert/diecut/commit/efa8c18f22767adc695608ecf2b89f3d15d39a06))
* update README with subpath examples and starter templates ([8964392](https://github.com/raiderrobert/diecut/commit/896439224b7d51c4b619e9a930beafb68fdb1d35))
* update site for trim-and-refactor changes ([e6bafcd](https://github.com/raiderrobert/diecut/commit/e6bafcd5c481bae005c672cfce9fc7ea360fbcc3))


### Code Refactoring

* remove check/ready commands, examples, and unused deps ([90aef09](https://github.com/raiderrobert/diecut/commit/90aef094d7e109538cc613c8f3ce9d365d7f8890))
* strip non-core features (cookiecutter, update, rhai hooks) ([4bf7182](https://github.com/raiderrobert/diecut/commit/4bf7182cc8dfc2ca674680db2c6a8e1132ab7199))
* strip non-core features and trim to new + list ([3c869d9](https://github.com/raiderrobert/diecut/commit/3c869d99971d3ca678c68e9e715e87d95f15a6ef))
* trim non-core features and add subpath support ([3137e5a](https://github.com/raiderrobert/diecut/commit/3137e5abe7bc7bc4b29bcc010663068a386b8132))

## [0.2.0](https://github.com/raiderrobert/diecut/compare/v0.1.6...v0.2.0) (2026-02-15)


### Features

* add --dry-run flag to new command ([2299430](https://github.com/raiderrobert/diecut/commit/229943054fd9f96a53c83b56022f619035b3965d)), closes [#62](https://github.com/raiderrobert/diecut/issues/62)
* add --dry-run flag to update command ([5df566b](https://github.com/raiderrobert/diecut/commit/5df566bf4407fab7792f30365e5bc02d02ddcdae)), closes [#60](https://github.com/raiderrobert/diecut/issues/60)
* add --verbose flag to dry-run output ([7d68cdb](https://github.com/raiderrobert/diecut/commit/7d68cdb43bb9128e07a048bdd831bf3f9afc8774)), closes [#63](https://github.com/raiderrobert/diecut/issues/63)


### Code Refactoring

* split generate() into plan and execute phases ([711b4c1](https://github.com/raiderrobert/diecut/commit/711b4c1bdc21a30bd7eb599c97f15c909fe7617d)), closes [#61](https://github.com/raiderrobert/diecut/issues/61)

## [0.1.6](https://github.com/raiderrobert/diecut/compare/v0.1.5...v0.1.6) (2026-02-14)


### Bug Fixes

* use SSH URL for gh: abbreviation when user has SSH configured ([8828b7d](https://github.com/raiderrobert/diecut/commit/8828b7dfd18fb4217b0f456c6a3cbc2a992b536c))

## [0.1.5](https://github.com/raiderrobert/diecut/compare/v0.1.4...v0.1.5) (2026-02-14)


### Bug Fixes

* suppress interactive git credential prompts ([9b0ecbd](https://github.com/raiderrobert/diecut/commit/9b0ecbd0c3ec107cd76345732572e4a000b2765a))


### Documentation

* add CONTRIBUTING.md and fix stale paths in README ([c8c714f](https://github.com/raiderrobert/diecut/commit/c8c714f0216bbd0e069d4cc3a405b3dbc9abc946))

## [0.1.4](https://github.com/raiderrobert/diecut/compare/v0.1.3...v0.1.4) (2026-02-14)


### Bug Fixes

* add --repo flag to gh workflow dispatch ([717d133](https://github.com/raiderrobert/diecut/commit/717d13352d05b3c178ecf0ada5a97c4dd01c7e93))
* add actions:write permission for workflow dispatch ([a311d17](https://github.com/raiderrobert/diecut/commit/a311d171dbbe27de405ea413713d92e82ad40b0b))
* release binaries not building for v0.1.1+ ([2e482bf](https://github.com/raiderrobert/diecut/commit/2e482bf83e82ad2579527ad6814898c31fb8381f))

## [0.1.3](https://github.com/raiderrobert/diecut/compare/v0.1.2...v0.1.3) (2026-02-14)


### Bug Fixes

* release binaries not building for v0.1.1+ ([97ebd42](https://github.com/raiderrobert/diecut/commit/97ebd42de4cdd5899948393b5e132c45a0799c8b))

## [0.1.2](https://github.com/raiderrobert/diecut/compare/v0.1.1...v0.1.2) (2026-02-14)


### Bug Fixes

* sync Cargo.lock version with Cargo.toml ([e120605](https://github.com/raiderrobert/diecut/commit/e120605f6914e3cc608e953b83efa36ce8e5289d))

## [0.1.1](https://github.com/raiderrobert/diecut/compare/v0.1.0...v0.1.1) (2026-02-14)


### Features

* add diff3-style three-way conflict output in .rej files ([39e87f8](https://github.com/raiderrobert/diecut/commit/39e87f8a75ced98e9e7ff3f3e815e57764b2ffd9))
* add install script and update install instructions ([d2a99f3](https://github.com/raiderrobert/diecut/commit/d2a99f3979d9c96b55dffcb4b885c7acb03e56fb))
* add PR title conventional commit check ([8da331c](https://github.com/raiderrobert/diecut/commit/8da331cd440394333b1c0e2e38ee416a0e832dfe))
* add release-please configuration and workflow ([fd9c08a](https://github.com/raiderrobert/diecut/commit/fd9c08a922eea1efab2b58792b90a14c9eb4e8ad))
* replace gix with system git for template cloning ([a8eb1c9](https://github.com/raiderrobert/diecut/commit/a8eb1c93af62bdb125adb432865b7e763dc99f37))
* support in-place migration with automatic backup ([e5f0de7](https://github.com/raiderrobert/diecut/commit/e5f0de7f0beab88d5a68ea4c608ec1f3c9235bb5))


### Bug Fixes

* add cargo-workspace plugin to release-please config ([e181218](https://github.com/raiderrobert/diecut/commit/e181218d8a9e27e6e0bd526ac78190b154dfffcd))
* configure release-please for Rust workspace per upstream pattern ([77c43c8](https://github.com/raiderrobert/diecut/commit/77c43c88fb16873642aef3cce012a25aeeae572f))
* diverged tag history ([1fdc5fa](https://github.com/raiderrobert/diecut/commit/1fdc5fa0ea028418f71d78a14e258b4e14b7afd5))
* include file path in template render error messages ([5d87ddd](https://github.com/raiderrobert/diecut/commit/5d87ddd7f49fc42a9af942bb0fb8a919c22a7e1c))
* read only first 8KB for binary file detection ([9fc4ed8](https://github.com/raiderrobert/diecut/commit/9fc4ed8253301c32d981f3ccf7ae6821564e4864))
* remove cargo-workspace plugin and restore version.workspace ([1d074c9](https://github.com/raiderrobert/diecut/commit/1d074c9b3c8529f0ab8f7660f01617fac69df97e))
* remove component prefix from release-please tags ([286ac2b](https://github.com/raiderrobert/diecut/commit/286ac2bc197e66beb9dd681bfe52cda1aa219b1d))
* removed unneeded plan ([1f4bee9](https://github.com/raiderrobert/diecut/commit/1f4bee9a86ecccf1c9d483e7263749d999a5b5b5))
* sandbox Rhai hook engine to prevent filesystem access ([d2092e2](https://github.com/raiderrobert/diecut/commit/d2092e21dd6884eefcf178967e4793f6057ec8ba))
* update stale crate paths after single-crate merge ([2d796dc](https://github.com/raiderrobert/diecut/commit/2d796dcc43206173f54794dcf741f7bf73bee293))
* use content_inspector for BOM-aware binary detection ([d995178](https://github.com/raiderrobert/diecut/commit/d9951780a642e68edf3602cde586811268dba832))
* use explicit versions in subcrate Cargo.toml for release-please ([52df58f](https://github.com/raiderrobert/diecut/commit/52df58f60750b1df9fee0dc323cf56a69999d2a8))
* use OS-level advisory locks (fs4) for cache concurrency ([072072c](https://github.com/raiderrobert/diecut/commit/072072c9728c795865c2b68f4d4d187bbf9f03cc))
* use rename-swap for in-place migration instead of clear-and-copy ([c061f60](https://github.com/raiderrobert/diecut/commit/c061f609be8df344215a25814a82d3d00c2c7410))


### Miscellaneous

* add .worktrees/ to gitignore ([3722144](https://github.com/raiderrobert/diecut/commit/37221444f4756713963374f7dd233fe8112e22f9))
* add MIT license file ([7d46744](https://github.com/raiderrobert/diecut/commit/7d467444f701973364cc19a5c54d5f9a2dae8175))
* add release-please configuration ([c752cd6](https://github.com/raiderrobert/diecut/commit/c752cd6996c2474b3614228fc63af9a98cac077d))
* clean up links ([b5f8684](https://github.com/raiderrobert/diecut/commit/b5f86845826a84debcd4d7fe513a6ccc505779aa))
* fix links in docs ([2fb6e1a](https://github.com/raiderrobert/diecut/commit/2fb6e1a68b5341f32d96575827a899936924484f))
* link to license ([6be9147](https://github.com/raiderrobert/diecut/commit/6be91470c0638ace4a93b85e55fe7dd9b2af1197))
* **main:** release diecut 0.1.1 ([7f09800](https://github.com/raiderrobert/diecut/commit/7f09800ee29dcc0b4cceb4370dfe0da2dda356b4))
* release main ([18b4195](https://github.com/raiderrobert/diecut/commit/18b41952a591804fafea0924755b7559ffae95f5))
* remove docs-redesign plan files ([c088220](https://github.com/raiderrobert/diecut/commit/c088220ce91561da6383417bcf1b50efda5c0d26))
* remove unused indicatif dependency ([3486a78](https://github.com/raiderrobert/diecut/commit/3486a78908e1a336adc576335e5f2e372064265b))
* remove verbose comments and simplify code ([027bb15](https://github.com/raiderrobert/diecut/commit/027bb158ed5ad48cefff362c622d50b4da4fada3))


### Documentation

* add astro-cf-template migration design ([9f0928a](https://github.com/raiderrobert/diecut/commit/9f0928a533d5ea9580e54362f3bbce4fd49cef32))
* add cookiecutter migration guide ([5c3f844](https://github.com/raiderrobert/diecut/commit/5c3f844463a71dc38b6d435e9a36b3a11e2d86d8))
* add creating-templates guide (flagship page) ([8790a2c](https://github.com/raiderrobert/diecut/commit/8790a2c6d723f68e2a94c50392d8540020e3108c))
* add design for replacing gix with system git for cloning ([b02f4a6](https://github.com/raiderrobert/diecut/commit/b02f4a6c28ddcdd0074bb972d5ba659f5fa27a90))
* add diecut.toml config reference ([3eb017d](https://github.com/raiderrobert/diecut/commit/3eb017d466d9a225509e1e9df1c72ba867281d13))
* add documentation redesign design plan ([cbe9db7](https://github.com/raiderrobert/diecut/commit/cbe9db7f72f8e489e53b77f0584ff4e28f394ec5))
* add documentation redesign implementation plan ([61fb1b6](https://github.com/raiderrobert/diecut/commit/61fb1b6601a574b642c23bec73ca80feedc15f57))
* add hooks reference ([2722050](https://github.com/raiderrobert/diecut/commit/2722050a2fe44a8e22769127f4e83eccbab97214))
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
* merge diecut-core and diecut-cli into single crate ([5d6e1b3](https://github.com/raiderrobert/diecut/commit/5d6e1b30305278878001fbf8c93bd27048525289))
* use Option&lt;&str&gt; for commit_sha parameter ([2178c3b](https://github.com/raiderrobert/diecut/commit/2178c3b985bc54c4fe4d2314279227f425a8e293))

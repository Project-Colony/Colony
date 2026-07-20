# Changelog

## [0.8.0](https://github.com/Project-Colony/Colony/compare/v0.7.1...v0.8.0) (2026-07-20)


### Features

* Colony's own app icon in the blessed assets/icons/ layout ([#49](https://github.com/Project-Colony/Colony/issues/49)) ([ab30ded](https://github.com/Project-Colony/Colony/commit/ab30dedcf915d43b75f1ab5b2f6ada827c181485))
* download size and speed in the progress toast; keyboard grid navigation ([#47](https://github.com/Project-Colony/Colony/issues/47)) ([f2e7d8f](https://github.com/Project-Colony/Colony/commit/f2e7d8f154a98a3066d7dc82c3d86a2b961b279b))
* live language switching and desktop entries for installed apps ([#46](https://github.com/Project-Colony/Colony/issues/46)) ([2931f36](https://github.com/Project-Colony/Colony/commit/2931f36dd3985fe96d23ec7d327e6740967bc723))
* local-app cards with explicit controls, local favorites, persisted window size ([#37](https://github.com/Project-Colony/Colony/issues/37)) ([d6b15c9](https://github.com/Project-Colony/Colony/commit/d6b15c90a3e2def1a0dbd71991968a3df3314b20))
* one-click Update All and per-release 'What's new' notes ([#35](https://github.com/Project-Colony/Colony/issues/35)) ([f6f6fa0](https://github.com/Project-Colony/Colony/commit/f6f6fa04fbe815f4724eb48cb6d9412a1e93ce98))
* store hygiene - orphan cache pruning, description search, installed version ([#36](https://github.com/Project-Colony/Colony/issues/36)) ([3de8cd5](https://github.com/Project-Colony/Colony/commit/3de8cd5ca7d242a365ba01c54ab8afb8af4b13a6))
* verify app release signatures and expressive filePattern globs ([#42](https://github.com/Project-Colony/Colony/issues/42)) ([31f60cd](https://github.com/Project-Colony/Colony/commit/31f60cd041248c60057d5839db8223705b81b4fa))


### Bug Fixes

* eight small verified defects from the audit's long tail ([#39](https://github.com/Project-Colony/Colony/issues/39)) ([dd7f6b8](https://github.com/Project-Colony/Colony/commit/dd7f6b8b3c8080cf7a9328ed262d96d73db5a62a))
* make installs atomic with their metadata; safer asset matching and launch paths ([#32](https://github.com/Project-Colony/Colony/issues/32)) ([d4159b1](https://github.com/Project-Colony/Colony/commit/d4159b1c4cd14c69fc93fb9261cb7283c41b4bcb))
* state-integrity pass on the update loop (detail by name, honest checks, badge lifecycle) ([#30](https://github.com/Project-Colony/Colony/issues/30)) ([e5870da](https://github.com/Project-Colony/Colony/commit/e5870da41723cbeaf668c63ab40b7a35d3462fc6))
* typed 404 classification and package-manager guidance for self-update ([#33](https://github.com/Project-Colony/Colony/issues/33)) ([315bc4d](https://github.com/Project-Colony/Colony/commit/315bc4d9b4b089ce077a9b9abe2d978f858c11a0))


### Performance Improvements

* cache per-repo install status instead of per-frame disk I/O ([#48](https://github.com/Project-Colony/Colony/issues/48)) ([d6583a5](https://github.com/Project-Colony/Colony/commit/d6583a5bd12fe91b40fd29fc60848708cec127e3))

## [0.7.1](https://github.com/Project-Colony/Colony/compare/v0.7.0...v0.7.1) (2026-07-20)


### Bug Fixes

* browse and install from the catalog without GitHub sign-in ([#28](https://github.com/Project-Colony/Colony/issues/28)) ([8d9f691](https://github.com/Project-Colony/Colony/commit/8d9f691b62d789375360c41fe952a978f93575e6))
* **ci:** sign release assets in CI and chain the AUR bump after the build ([#27](https://github.com/Project-Colony/Colony/issues/27)) ([fa803e2](https://github.com/Project-Colony/Colony/commit/fa803e2de1293b21ecb43faa0f7d313623fa4bdd))

## [0.7.0](https://github.com/Project-Colony/Colony/compare/v0.6.0...v0.7.0) (2026-07-16)


### Features

* **themes:** Stellar Blade character theme family (5 palettes) ([9b102ee](https://github.com/Project-Colony/Colony/commit/9b102ee0026a753e79a81cbd1146428837033f87))

## [0.6.0](https://github.com/Project-Colony/Colony/compare/v0.5.0...v0.6.0) (2026-07-13)


### Features

* per-app icons in the grid (decentralized, PNG) ([#23](https://github.com/Project-Colony/Colony/issues/23)) ([4b45ed8](https://github.com/Project-Colony/Colony/commit/4b45ed8807a8891ae04e165b0918aeb67bb207a4))

## [0.5.0](https://github.com/Project-Colony/Colony/compare/v0.4.2...v0.5.0) (2026-07-08)


### Features

* signed self-update, Hive app grid, and audit hardening ([#20](https://github.com/Project-Colony/Colony/issues/20)) ([849828e](https://github.com/Project-Colony/Colony/commit/849828ed2ff643114e98d45c418f36159866e373))

## [0.4.2](https://github.com/Project-Colony/Colony/compare/v0.4.1...v0.4.2) (2026-06-02)


### Bug Fixes

* **config:** resolve config from user/exe dirs and embed shipped categories ([#18](https://github.com/Project-Colony/Colony/issues/18)) ([a203d5e](https://github.com/Project-Colony/Colony/commit/a203d5e4abbe47cc7ee25c53fd6006c3af52c1b5))

## [0.4.1](https://github.com/Project-Colony/Colony/compare/v0.4.0...v0.4.1) (2026-06-02)


### Bug Fixes

* **deps:** bump openssl, rustls-webpki, tar to clear 15 Dependabot alerts ([#16](https://github.com/Project-Colony/Colony/issues/16)) ([1d9c789](https://github.com/Project-Colony/Colony/commit/1d9c78933ece111bdcf203050c8c4285e1812556))

## [0.4.0](https://github.com/Project-Colony/Colony/compare/v0.3.0...v0.4.0) (2026-04-20)


### Features

* **tutorial:** spotlight real UI zones on first launch ([#14](https://github.com/Project-Colony/Colony/issues/14)) ([90bbc5f](https://github.com/Project-Colony/Colony/commit/90bbc5f1b488dc77270b53364508e25cc5b356f5))

## [0.3.0](https://github.com/Project-Colony/Colony/compare/v0.2.1...v0.3.0) (2026-04-19)


### Features

* **ui:** shields.io badges as pills + native table rendering ([#12](https://github.com/Project-Colony/Colony/issues/12)) ([a90ac70](https://github.com/Project-Colony/Colony/commit/a90ac70eec7c60134c28fc9cd3f65c189947a92e))

## [0.2.1](https://github.com/Project-Colony/Colony/compare/v0.2.0...v0.2.1) (2026-04-19)


### Bug Fixes

* **ui:** embed FontAwesome 6 Free for properly centered icons ([#9](https://github.com/Project-Colony/Colony/issues/9)) ([8583e2f](https://github.com/Project-Colony/Colony/commit/8583e2f7715369182cf474e88a3d563442facd43))

## [0.2.0](https://github.com/Project-Colony/Colony/compare/v0.1.4...v0.2.0) (2026-04-18)


### Features

* **onboarding:** carousel 3 étapes pour le premier lancement ([08c6019](https://github.com/Project-Colony/Colony/commit/08c60191388255c06797168b1017ac4f85bc8bf9))


### Bug Fixes

* **ui:** dismiss GitHub/Settings overlays when a sidebar section is clicked ([6021bdf](https://github.com/Project-Colony/Colony/commit/6021bdfde3bb1a50da4aed2fb3256d47700cac5a))

## [0.1.4](https://github.com/Project-Colony/Colony/compare/v0.1.3...v0.1.4) (2026-03-08)


### Bug Fixes

* hide console window on Windows ([5935427](https://github.com/Project-Colony/Colony/commit/5935427fdfe59df1950b23d3fbbd009c01cefa7c))

## [0.1.3](https://github.com/Project-Colony/Colony/compare/v0.1.2...v0.1.3) (2026-03-08)


### Bug Fixes

* handle raw binaries in extract_binary_from_archive ([fb6c526](https://github.com/Project-Colony/Colony/commit/fb6c526e493fe1d430389423580146bc55faa88e))

## [0.1.2](https://github.com/Project-Colony/Colony/compare/v0.1.1...v0.1.2) (2026-03-08)


### Bug Fixes

* security hardening and i18n cleanup ([50b5e65](https://github.com/Project-Colony/Colony/commit/50b5e65a0b0c4d22eea74678e75697039cc88467))

## [0.1.1](https://github.com/Project-Colony/Colony/compare/v0.1.0...v0.1.1) (2026-03-02)


### Bug Fixes

* resolve clippy warnings (unnecessary unwrap, type complexity, redundant closure) ([17382b5](https://github.com/Project-Colony/Colony/commit/17382b5d7f40240d0ba1217af15f2ed2332b5b29))

## 0.1.0 (2026-03-02)


### Features

* Colony launcher — browse, install, update, and launch Colony apps ([20cc20d](https://github.com/Project-Colony/Colony/commit/20cc20d6e60605d4f10bb80b69c1fcd3619d7978))

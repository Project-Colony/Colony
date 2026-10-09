# Release signing

Colony checks an **ed25519 signature** before it runs anything it downloads.
The trusted public keys are embedded in [`src/signing.rs`](../src/signing.rs)
(`RELEASE_PUBLIC_KEYS`) and verified with the pure-Rust `ed25519-dalek` crate,
so the shipped binary needs no OpenSSL.

- Signature format: the **raw 64-byte ed25519 signature** over the asset bytes,
  exactly what `openssl pkeyutl -sign -rawin` emits (base64 text is also
  accepted). Published as `<asset>.sig` next to each release asset.
- **Launcher self-updates** (`colony-<platform>[.exe]`): mandatory and
  fail-closed. If the signature or the signed sidecar (below) is missing,
  malformed or does not verify, the update is refused and the running binary
  is left untouched.
- **App installs and updates**: any app release that publishes `<asset>.sig`
  MUST verify against the same keys, or the install is refused. An app without
  a `.sig` installs as a legacy unsigned app, unless its `colony.json` declares
  `"signed": true`. Once an install of an app has verified a signature, that
  app is pinned: a later release that stops publishing `.sig` is refused,
  whatever the manifest now says. The same applies to the `.meta` sidecar
  ([`src/download.rs`](../src/download.rs), `download_release_asset`). The
  optional `sha256` field in `colony.json` is checked on top of that.

The organisation signs every program with one key, so the keys embedded here
are the trust root for the launcher and for every signed app at once.

### Why a signature alone is not enough

A signature over raw bytes proves only *these bytes came from the release key* -
not **which** artefact or **which** version they are. Anything able to control
what the release host serves could therefore replay an older, genuinely signed
build as an "update" (a downgrade), or serve the macOS asset where the Linux one
was requested. So every asset also gets a **signed metadata sidecar**:

```
<asset>.meta        version=v1.2.3
                    asset=colony-linux
                    sha256=<hex of the asset bytes>
<asset>.meta.sig    ed25519 signature over the .meta bytes
```

The launcher verifies the sidecar's signature, then requires that it names the
asset it asked for, that its digest matches the bytes actually downloaded, that
its version equals the tag the update check resolved, and that this version is
**strictly newer** than the running build. That last check is the anti-rollback.
Both the signature and the sidecar are re-verified at install time, not only at
download time, so the staging file cannot be swapped in between. Apps get the
same bindings with one difference: their version must be **no older** than the
installed one, since an app pinned to a fixed tag must stay reinstallable.

## Every release MUST ship signatures and sidecars

Because verification is fail-closed, a release published without the
`colony-<platform>.sig`, `.meta` and `.meta.sig` assets will make self-update
fail for users on that channel. The release workflow's `sign-and-publish` job
checks all three exist and verify for all four platforms and fails the release
otherwise.

Every other program's release depends on Colony's too: the organisation's
release template downloads Colony's latest `colony-linux`, verifies its `.sig`
and `.meta` against Colony's public key, and runs its `validate-manifest` on
the program's `colony.json`. A Colony release whose signatures do not verify,
or whose `validate-manifest` regresses, stops every program in the organisation
from releasing.

## How releases are signed

Signing happens in CI and nowhere else. Since the v0.7.0 incident (a release
shipped unsigned because signing was a manual step, bricking self-update for
every existing install), there is no hand-signing procedure and no signing
script in this repository.

The release workflow is `.github/workflows/release.yml`. Merging the release
PR that release-please opens creates the tag and a release, which the workflow
immediately holds as a draft. Its build legs only build, smoke-test and upload
each binary as a workflow artifact; they never see a key. The
`sign-and-publish` job then calls the organisation's shared workflow,
`.github/workflows/sign-and-publish.yml` in
[Project-Colony-Resources](https://github.com/Project-Colony/Project-Colony-Resources),
pinned by commit, which in that same run:

1. checks the release is still a draft;
2. sends `colony-windows.exe` to SignPath for Authenticode, once SignPath is
   turned on for this repository (`signpath-project-slug`), and waits for a
   person to approve the request;
3. writes `.sig`, `.meta` and `.meta.sig` over the final bytes with the
   `COLONY_SIGNING_KEY_PEM` organisation secret, in a job that checks out
   nothing and builds nothing;
4. uploads everything to the draft, downloads it again and verifies every
   signature and digest against what users will download;
5. publishes the release.

The order is the point: Authenticode rewrites the `.exe`, so a `.sig` or
`.meta` computed before it would describe bytes that no longer exist, and the
launcher would refuse the update. The job **fails the release** if the secret is
missing or any file is missing, empty or does not verify, so a release the
launcher cannot verify can no longer ship silently. Only after that does the
`aur` job bump `colony-bin`.

A published release carries 16 assets (four binaries, each with `.sig`,
`.meta` and `.meta.sig`):

```sh
gh release view v1.2.3 --json assets --jq '.assets|length'   # must be 16
```

### If a release goes wrong

`gh release view <tag> --json isDraft,assets --jq '{draft:.isDraft, n:(.assets|length)}'`
tells you which state you are in.

**Still a draft, incomplete.** Nobody is affected: `/releases/latest` still
points at the previous version and no client has been offered anything. Fix the
cause and use **"Re-run failed jobs"**. The build artifacts are kept for one
day; after that, dispatch the workflow again, started from the tag itself:

```sh
gh workflow run release.yml --ref <tag> -f tag=<tag>
```

The dispatch rebuilds the tagged commit, signs it and publishes it. It must
start from the tag, not from `main`: with SignPath on, the shared workflow
refuses a run whose commit is not the tag's.

**Never "Re-run all jobs".** release-please re-runs against a `main` whose
release already exists, emits an empty `release_created`, and every downstream
job skips - while the run reports green. That looks like a successful recovery
and is the opposite of one.

**Published but unsigned or partial.** Clients are being offered an update that
cannot be applied. Take it out of `latest` by turning it back into a draft,
confirm the fallback, then run the same dispatch:

```sh
gh release edit <tag> --draft=true
gh api repos/Project-Colony/Colony/releases/latest --jq .tag_name   # the previous version
gh workflow run release.yml --ref <tag> -f tag=<tag>
```

The workflow refuses to touch a release that is still published, because
replacing live binaries would leave signatures describing bytes users no longer
receive. Drafting it first is what makes the rebuild possible.

## Key custody

- The private key exists in one place: the `COLONY_SIGNING_KEY_PEM`
  organisation secret (the PEM contents). Only the signing job of the shared
  workflow receives it. There is no copy on any machine and none in any
  repository.
- GitHub never returns a secret's value, so nobody, maintainers included, can
  read the key back. If the secret is deleted or overwritten, the key is gone
  and the only way forward is a rotation (below). If it leaks, rotate
  immediately.
- Every program in the organisation is signed with this same key, so a
  rotation is an organisation-wide event, not a Colony-only one.

## Generating / rotating the key

```sh
# 1. New keypair, on a trusted machine. The private half goes into the org
#    secret and is then deleted from disk.
openssl genpkey -algorithm ed25519 -out colony-release.pem
openssl pkey -in colony-release.pem -pubout -out colony-release.pub.pem

# 2. Extract the raw 32-byte public key (ed25519 SPKI = 12-byte header + 32-byte key)
openssl pkey -pubin -in colony-release.pub.pem -outform DER | tail -c 32 | xxd -i
```

`src/signing.rs` embeds a **list** of accepted keys (`RELEASE_PUBLIC_KEYS`), and
a signature is accepted if any listed key validates it. That is what makes a
rotation possible at all: with a single key, the one `<asset>.sig` a release
carries is either old-key (refused by every updated client) or new-key (refused
by every client in the field), and verification is fail-closed, so the refusal
is permanent either way.

Rotate over three releases:

| Release | `RELEASE_PUBLIC_KEYS` contains | Signed with | Who can update |
|---|---|---|---|
| N | `[new, old]` | **old** | everyone in the field; afterwards they trust both |
| N+1 | `[new, old]` | **new** | everyone on N or later |
| N+2 | `[new]` | **new** | everyone on N or later; `old` is now revoked |

Step by step:

1. **Ship N carrying both keys, signed with the old one.** Add the new key to
   `RELEASE_PUBLIC_KEYS`, update the length assertion in the test below, and
   release as usual: the org secret still holds `old`. N's whole job is to
   widen the trusted set on machines that only trust `old`. Signing it with
   `new` is the mistake that strands the install base.
2. **Wait.** Leave N in the field long enough for installs older than N to be
   a rounding error, and check the release download counts.
3. **Switch the org secret to `new`.** From then on the shared workflow signs
   every program with `new`, not only Colony. Every Colony install still on
   N-1 or older trusts only `old`, so it refuses **every signed app** it tries
   to install or update, not only its own self-update. That is why step 2
   comes first.
4. **Ship N+1**, signed with `new`. At the same time, replace Colony's public
   key PEM everywhere it is copied:
   - in Project-Colony-Resources, `templates/sign-and-publish-caller.yml` (the
     `Validate colony.json` step verifies Colony's latest `colony-linux`
     against it) and the `## Code signing policy` section of
     `templates/program/README.md`;
   - in every program, its copy of that release workflow and the public key in
     its README's `## Code signing policy`;
   - in this file, under "Verifying a signature by hand".

   Until a program's workflow carries the new PEM, its releases stop at the
   manifest check once N+1 is Colony's latest release.
5. **Ship N+2** with `old` removed from `RELEASE_PUBLIC_KEYS`. Do not skip
   to it: anyone still on N-1 or earlier when `old` is dropped can no longer
   self-update and must reinstall by hand.

For an EMERGENCY rotation after a key leak, the same sequence applies but the
overlap is a liability rather than a courtesy: the attacker holding `old` can
sign anything the field will accept until N+2 ships. Publish N and N+1 back to
back, keep the window to hours, and say so publicly - a compromised key is not a
quiet fix. If the old key is lost rather than leaked, N cannot be signed with
it: every install older than the first release embedding the new key has to
reinstall by hand.

The test `any_trusted_key_verifies_and_an_untrusted_one_does_not` in
`src/signing.rs` asserts both halves: any listed key validates, an unlisted one
never does. It also asserts the list is length 1, so starting a rotation
requires deliberately updating that assertion.

## Verifying a signature by hand

With OpenSSL 3 and the public key:

```sh
cat > colony-release.pub.pem <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=colony-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub.pem -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub.pem -rawin -in "$a.meta" -sigfile "$a.meta.sig"
# -> "Signature Verified Successfully", twice
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

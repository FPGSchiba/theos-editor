# Release pipeline

Status: accepted with the handoff; built in P05 (CE-08R) and extended in P18 (CE-49). Binding.
Versioning and tagging rules are in `CONTRIBUTING.md` §5.

## 1. Goals

- Jann decides when a release happens; nothing else does.
- One product version and one tag per release.
- The changelog is written from the Conventional Commits on `main`, not by hand.
- Every artifact is built by CI from the tagged commit with `--locked`, for Windows and Linux,
  with checksums and build provenance.

## 2. Flow

```text
push to main ──► release-plz updates the release PR "chore(release): vX.Y.Z"
                 (version in [workspace.package], CHANGELOG.md section)
Jann merges the release PR
             ──► release-plz creates the annotated tag vX.Y.Z on main
tag vX.Y.Z   ──► release.yml builds on windows-latest and ubuntu-latest
                 ──► GitHub Release vX.Y.Z with artifacts, SHA256SUMS, provenance,
                     notes = the CHANGELOG section (pre-release for -rc.N)
```

1. **Release PR.** On every push to `main`, release-plz opens or updates one pull request with the
   next version (from the commit types since the last tag) and the new `CHANGELOG.md` section. The
   PR touches protected paths, so the gate keeps it waiting; Claude never merges it. Jann can edit
   the proposed version (for example to reach a milestone or cut `-rc.1`) before merging.
2. **Tag.** Merging the release PR makes release-plz create the tag `vX.Y.Z`. Only the
   `theos-editor` package creates a tag; all crates are in one version group.
3. **Build and publish.** The tag starts `release.yml`, which builds, checksums, attests and
   creates the GitHub Release. Nothing is published to crates.io.
4. **Consumers.** The GitHub Action in tower-of-theos that runs `theos check` pins a released
   version; moving it to a new release is a normal pull request there.

## 3. Artifacts

| Artifact | From | Built since |
|---|---|---|
| `theos-<version>-x86_64-pc-windows-msvc.zip` (`theos.exe`, `theos-mcp.exe`) | `cargo build --release --locked` | P05 |
| `theos-<version>-x86_64-unknown-linux-gnu.tar.gz` (`theos`, `theos-mcp`) | same | P05 |
| `Theos Editor_<version>_x64-setup.exe` | Tauri bundler (NSIS) | P18 |
| `SHA256SUMS` | all files above | P05 |
| Build provenance attestation | `actions/attest-build-provenance` | P05 |

`theos --version`, `theos-mcp --version` and the editor's About page show the product version, the
commit, the data model `schema_version` and the generator protocol version.

## 4. Configuration sketch (written in P05)

`release-plz.toml`:

```toml
[workspace]
publish = false                 # never publish to crates.io
git_release_enable = false      # release.yml creates the GitHub Release
changelog_config = "cliff.toml"

[[package]]
name = "theos-editor"
version_group = "theos"
git_tag_enable = true
git_tag_name = "v{{ version }}"

# every other crate: version_group = "theos", git_tag_enable = false
```

`cliff.toml` groups commits into Breaking changes, Features, Fixes, Performance, Documentation and
Reverts, drops `refactor`, `test`, `build`, `ci` and `chore`, and links each entry to its pull
request. The exact keys are fixed in P05 against the release-plz version in use.

## 5. Branches and hotfixes

Before 1.0 there are no release branches: every release is cut from `main`, and fixes go to
`main` first. After 1.0, a `release/vX.Y` branch may be created for patch releases of an older
minor version; that decision is Jann's.

## 6. When something goes wrong

- A failed `release.yml` run is re-run on the same tag; the tag never moves.
- A broken release is followed by a fixed patch release; its GitHub Release notes say which
  version replaces it. Tags and releases are never deleted.

## 7. Not in v1

Code signing for Windows (the installer shows a SmartScreen warning until a certificate is
added), the Tauri auto-updater (needs signing keys), macOS builds, crates.io publishing.

# Release maintainer setup

For release maintainers only. To install the client, see [Client installation](Distribution).

1. Create the public `inbjo/homebrew-tap` repository with `main` as its default branch.
2. Create a fine-grained PAT with Contents read/write access only to that tap and
   save it as the `HOMEBREW_TAP_TOKEN` Actions secret in this repository.
3. Create a dedicated APT signing key. Save its ASCII-armored private key as
   `APT_GPG_PRIVATE_KEY` and, when applicable, its passphrase as
   `APT_GPG_PASSPHRASE`. Never commit the private key.

   ```bash
   gpg --quick-generate-key "MirrorProxy APT Repository <noreply@github.com>" ed25519 sign 3y
   gpg --list-secret-keys --keyid-format=long
   gpg --armor --export-secret-keys YOUR_KEY_FINGERPRINT >mirrorproxy-apt-private.asc
   ```

   After storing the complete exported file in the secret, back it up offline
   and remove it from the working directory. Losing the key prevents existing
   clients from authenticating future repository updates.
4. The Release workflow publishes the signed repository to the dedicated `apt`
   branch at `https://raw.githubusercontent.com/inbjo/MirrorProxy/apt`.
5. After the first WinGet manifest is accepted into `microsoft/winget-pkgs`, add
   a `WINGET_TOKEN` capable of creating the update PR and set the repository
   variable `WINGET_AUTO_SUBMIT` to `true`.

   For the first release, download `mirrorproxy-winget-manifests.zip` from the
   GitHub Release, run `winget validate <manifest-directory>`, and submit the
   `manifests/i/Inbjo/MirrorProxy/<version>` directory to
   `microsoft/winget-pkgs`. Do not enable automatic submission until it merges.

On a stable `v*` tag, the release workflow builds the client packages and
manifests, signs and deploys the APT repository, and updates the Homebrew tap.
WinGet submission runs only after it is explicitly enabled.

[简体中文](Release-Maintenance-zh-CN)

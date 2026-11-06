# Code signing on Windows

Orrery's Windows installers are not signed yet. The only free way for an open-source project is the [SignPath Foundation](https://signpath.org/). Azure Artifact Signing is not open to private persons in Germany and costs about 10 dollars a month, a Certum open-source certificate costs at least 69 euros per term. Both conflict with Orrery's rule of no running costs.

Signing does not remove the SmartScreen warning right away. Reputation grows with every release that is signed with the same identity.

## Application checklist

These steps need the repository owner, they cannot be done by a build:

1. Read the conditions at <https://signpath.org/terms>. Orrery fits the license rule (GPL-3.0-only, no proprietary parts, no commercial dual license) and is published in the form that will be signed
2. Enable multi-factor authentication on GitHub, and later on SignPath
3. Make sure `site/code-signing.html` is published at <https://nowaaak.github.io/orrery/code-signing.html>. It names what is signed, the roles and the privacy page
4. Apply through <https://signpath.org/> with the repository URL, the release workflow and the code signing policy page
5. Other projects report that SignPath first issues a test certificate and the production certificate is requested separately once the pipeline works
6. Create the signing policies `test-signing` and `release-signing` in SignPath, restricted to the `main` branch and to GitHub-hosted runners, and add the repository secrets `SIGNPATH_API_TOKEN` and the variable `SIGNPATH_ORGANIZATION_ID`
7. Change the status box in `site/code-signing.html` to "Free code signing provided by SignPath.io, certificate by SignPath Foundation"

SignPath checks the project's reputation by hand. Orrery is young, so an application can be declined. Nothing else changes in that case.

## Release workflow

The release workflow currently runs `tauri-action`, which builds the executable, bundles NSIS and MSI and signs the updater files in one step. SignPath signs artifacts that a GitHub Actions job uploaded, so the Windows job has to be split:

1. Build the frontend and the executable with `npm run tauri build -- --no-bundle`
2. Upload `orrery.exe` with `actions/upload-artifact` and sign it through `signpath/github-action-submit-signing-request` with `wait-for-completion: true`
3. Put the signed executable back into `target/release` and run `npm run tauri bundle -- --bundles nsis,msi`
4. Upload the setup program and the MSI and sign them in a second request. The MSI is a container, so its artifact configuration signs the inner executable as well. The NSIS setup is treated as a plain executable
5. Create the updater signatures only now, with `npm run tauri signer sign` on the signed setup program, because Authenticode changes the file and an earlier `.sig` would no longer match
6. Upload the signed files, `latest.json` and the new `.sig` files to the draft release

Until SignPath is set up the workflow stays as it is. The steps above should first run with the test certificate on a separate branch.

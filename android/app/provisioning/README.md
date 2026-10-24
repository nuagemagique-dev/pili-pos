# Provisioning public key (development only)

`dev-minisign.pub` is the minisign **public** key that debug builds of the Pili POS app verify provisioning
bundles with (PD108). It is a throwaway key for development and the emulator. It signs nothing that reaches
a customer: release builds never read this file.

- The secret half lives only in `$HOME/.cache/pili-android/dev-minisign/minisign.key` (mode 600), outside
  every git work tree. It must never be committed, copied into the repository or sent anywhere.
- Release builds take their key from the Gradle property `-PpiliProvisioningPublicKey=<key text>` and refuse
  every bundle when it is empty (`prov_err_key_missing`).
- Only prehashed minisign signatures (the default of `minisign -S`) are accepted.

## Make a development bundle

```sh
scripts/android/make_dev_bundle.sh --seed            # writes $HOME/.cache/pili-android/dev-bundle
scripts/android/make_dev_bundle.sh --fdm-url http://10.0.2.2:19380/graphql --printer 10.0.2.2:19390 --out /tmp/bundle
```

The script creates the key pair when it is missing, refuses an output or key directory inside a git work
tree, and fails when the key pair does not match this `dev-minisign.pub`. On the emulator, push the bundle
folder to the debug inbox (`adb push <bundle>/. /sdcard/Android/data/dev.nuagemagique.pilipos.debug/files/provisioning-inbox/`)
or choose it with the folder picker.

## Regenerate the development key

Only needed when the secret half is lost (a new machine): the old public key then verifies nothing.

```sh
rm -rf "$HOME/.cache/pili-android/dev-minisign"
scripts/android/make_dev_bundle.sh --write-pub       # new pair, copies the public half here
git add android/app/provisioning/dev-minisign.pub    # commit the PUBLIC key only
```

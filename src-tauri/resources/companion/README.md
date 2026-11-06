# Companion jars

`npm run companion` builds the Orrery Companion mod in `companion/` and copies one jar per supported Minecraft version into this folder, together with `companion.json`, which lists the SHA-512 of every jar. The launcher only loads a jar whose hash matches the manifest.

The jars and the manifest are build output and are not committed.

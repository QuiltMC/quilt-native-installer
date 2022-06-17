> :warning: NOTE: This project is still in its early development. There's guaranteed bugs and missing functionality.

# Quilt Installer
This is a native installer for [quilt-loader](https://github.com/QuiltMC/quilt-loader).

## Note: Building linux executables
Because of the horrors that is glibc, I recommend building with musl instead.
The easiest way to achieve this is by building inside an alpine docker container:
```
docker run --rm -v "$PWD":/usr/src -w /usr/src rust:1.60-alpine sh -c "apk add --update --no-cache musl-dev openssl-dev && cargo build --release"
```

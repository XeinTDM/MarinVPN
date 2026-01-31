# Reproducible Builds for MarinVPN

MarinVPN is committed to transparency and security. In line with the [Reproducible Builds](https://reproducible-builds.org/) project, we provide a mechanism for anyone to verify that our official binaries match the public source code bit-for-bit.

## Why this matters
- **Trust but Verify:** You don't have to trust that the developers haven't added a backdoor; you can prove it yourself.
- **Security:** It prevents supply chain attacks where a malicious actor might compromise the build server.

## How to Verify (Server)

To reproduce the server binary on any system with Docker installed:

1. **Clone the repository at a specific tag:**
   ```bash
   git checkout v1.0.0
   ```

2. **Run the reproducible build container:**
   ```bash
   docker build -t marinvpn-repro -f Dockerfile.reproducible .
   ```

3. **Extract the hash:**
   ```bash
   docker run --rm marinvpn-repro
   ```

4. **Compare the output hash** with the hash provided in our official release notes. If they match, the binary is verified.

## Deterministic Compilation Flags
Our build environment uses several flags to ensure consistency across different machines:
- `--remap-path-prefix`: Strips the build-time directory paths from the binary.
- `-C codegen-units=1`: Ensures the compiler doesn't introduce non-determinism during parallel optimization.
- `RUST_VERSION`: Pinned to a specific version in `rust-toolchain.toml`.

## How to Verify (Client - Windows)

1. **Run the reproducible build container for Windows:**
   ```bash
   docker build -t marinvpn-win-repro -f Dockerfile.windows.reproducible .
   ```

2. **Extract the hash:**
   ```bash
   docker run --rm marinvpn-win-repro
   ```

## How to Verify (Client - Linux)

1. **Run the reproducible build container for the client:**
   ```bash
   docker build -t marinvpn-client-repro -f Dockerfile.marinvpn.reproducible .
   ```

2. **Extract the hash:**
   ```bash
   docker run --rm marinvpn-client-repro
   ```

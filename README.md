# Registro de equipos

A small app that adds the computer it runs on to the company's IT inventory ([INVENTARIO-IT](https://github.com/ZOOPA-Smart-Agency/INVENTARIO-IT)), or updates it if its serial number is already there. Someone from IT opens it, types the inventory's address and their email and password, checks the details it read, and sends them.

It reads the serial number, maker and model, processor, memory, drives, graphics, screen size, battery health, operating system, disk encryption (FileVault, BitLocker, LUKS), Active Directory membership and network cards. For a computer that is new to the inventory, it also asks for the asset tag, device type and location (filled in with sensible choices). For one that is already there, only its hardware and software details change: status, location, assignment, purchase data and notes stay as they are.

It needs inventory version 1.2.0 or later, and an account with the technician or administrator role.

It does nothing in the background and never updates itself: it runs while it is open and stops when it is closed. The password is only used to sign in, and is never saved.

| System | Versions | Download |
|---|---|---|
| macOS | 10.15 Catalina and later, Intel and Apple silicon (one universal app) | `.dmg` |
| Windows | 10 and 11, x64 and ARM | `-setup.exe` (or `.msi` for x64) |
| Linux | Ubuntu 22.04, Debian 12, Fedora 38 and later, and similar; x64 and ARM | `.deb`, `.rpm` or `.AppImage` |

Windows and Linux also have a command-line version (`inventario-registro-cli_….zip`), with no window, for servers and remote sessions. On macOS, the app itself works as the command line (see below).

## Unsigned builds

The builds are not signed yet, so each system warns the first time:

- **macOS**: right-click the app, choose **Open**, then **Open** again. On macOS 15 and later: open it once, then go to **System Settings > Privacy & Security** and click **Open Anyway**.
- **Windows**: in the blue "Windows protected your PC" window, click **More info**, then **Run anyway**.
- **Linux**: no warning. Make an AppImage executable first: `chmod +x Registro*.AppImage`.

## What needs administrator rights

Everything works as a normal user, except:

- **Linux**: the serial number and memory module details. Without them, the app asks for the serial number (it is usually on a label). To read everything, open it with `sudo`. With the AppImage, that is `sudo ./Registro*.AppImage`.
- **Windows**: battery health on some laptops.

## The server's own certificate

An inventory installed with its own certificate (`TLS_MODE=internal`, for in-house addresses and IP addresses) is not trusted by computers out of the box. The first time, the app shows the SHA-256 fingerprint of the certificate authority behind it, and asks whether to trust it. Compare it with the one the inventory's installer printed. To see that fingerprint again, run this in the server's install folder:

```bash
docker compose exec -T caddy cat /data/caddy/pki/authorities/local/root.crt | openssl x509 -noout -fingerprint -sha256
```

The choice is remembered, and it applies only to that address. Nothing private, the password included, is sent before the certificate is trusted. An inventory with a public certificate, or one from the company's own certificate authority installed on the computer, needs none of this.

## Command line

The command-line version takes the same steps as the window. The app itself also works this way when given options (for example `"/Applications/Registro de equipos.app/Contents/MacOS/inventario-registro" --help`).

```bash
inventario-registro-cli --server inventario.empresa.es --email it@empresa.es
```

It asks for the password, unless one of these is given:

- `--password-env VARIABLE` reads it from an environment variable.
- `--password-stdin` reads it from the first line of standard input.

| Option | Purpose |
|---|---|
| `--tag`, `--type`, `--location` | Asset tag, device type and location for a new computer. Type and location accept a name in any of the inventory's languages, or an id. |
| `--serial` | Serial number, when the computer does not report one |
| `--trust SHA256` | Trust the server's own certificate (or its certificate authority) with this fingerprint, without asking |
| `--dry-run` | Only show what would be sent; needs no server |
| `--json` | Print the result as JSON |

| Exit code | Meaning |
|---|---|
| 0 | Done |
| 2 | Wrong options |
| 3 | Certificate not trusted |
| 4 | Sign-in refused |
| 5 | Other problem |

## Settings

The last address and email, and the certificates someone chose to trust, are saved in `prefs.json` in a folder named `inventario-registro` in the user's settings folder:

- macOS: `~/Library/Application Support`
- Windows: `%APPDATA%`
- Linux: `~/.config`

Delete that file to forget them.

## Development

The code is a Rust workspace:

- `core`: reads the computer's details, talks to the server, and holds the command-line mode.
- `cli`: the command-line version.
- `app`: the [Tauri](https://tauri.app) window. `app/src` holds its page; `app/src-tauri` holds its Rust side.

You need Rust, and Node.js for the Tauri command-line tool. On Linux you also need the libraries listed in [.github/workflows/build.yml](.github/workflows/build.yml).

```bash
cargo test -p inventario-core
```

```bash
cargo run -p inventario-registro-cli -- --dry-run
```

```bash
cd app && npm ci && npx tauri dev
```

For testing against a local inventory, the address may be plain `http://localhost:8000`. Everywhere else it must be HTTPS. The server side is described in INVENTARIO-IT's `docs/agent-api.md`.

## Releases

Change `version` in `Cargo.toml`, then push a tag such as `v0.2.0`. GitHub Actions builds every system and attaches the files to a **draft** release. Check it and publish it by hand. Pushes to `main` only run the tests.

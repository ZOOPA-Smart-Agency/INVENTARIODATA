# Registro de equipos

A small app that adds the computer it runs on to the company's IT inventory ([INVENTARIO-IT](https://github.com/ZOOPA-Smart-Agency/INVENTARIO-IT)), or updates it if its serial number is already there. Someone from IT opens it, types the inventory's address, their email (or, from inventory 1.7.0, their username) and password, checks the details it read, and sends them.

It reads the serial number, maker and model, processor, memory, drives (whether each can be replaced, its connection generation such as SATA III or NVMe PCIe 4.0 where the system reports it, and its form factor where the model settles it), graphics, screen size, battery health, operating system, disk encryption (FileVault, BitLocker, LUKS), Active Directory membership and network cards (with the Ethernet ports' maximum speed).

It also reads each drive's own health data (SMART), only reading it and never testing or writing to the drive: whether it reports problems, and on Windows and Linux how worn an SSD is and its hours of use; and the battery's charge cycles. macOS only says whether a drive passes its own checks. On Windows this uses the administrator rights the app runs with; on Linux it needs `sudo` and the `smartmontools` package (`smartctl`), and is left out otherwise. On Linux, a drive that takes more than 15 seconds to answer is skipped.

Sizes are given as they are sold: a drive that holds 1,024 GB is the "1 TB" model, and Apple's "512 GB" SSD reports 500 GB. Only the computer's own network ports count: USB adapters and docks move from one computer to another, so they are left out. Where the system does not say something and it cannot be worked out with certainty (the form factor of a SATA SSD in a PC, an Ethernet port whose name and driver give no speed), it is left blank rather than guessed. On Macs it also notices OpenCore Legacy Patcher, which lets older Macs run a macOS Apple no longer supports on them, and reads its version (saved from inventory 1.11.0). For a computer that is new to the inventory, it also asks for the asset tag, device type and location (filled in with sensible choices). For one that is already there, only its hardware and software details change: status, location, assignment, purchase data and notes stay as they are.

It needs inventory version 1.2.0 or later (1.2.1 also saves the Wi-Fi generation and Bluetooth version), and an account with the technician or administrator role. From inventory 1.3.0, when its device types have tag codes, the suggested asset tag follows the chosen type (for example `ZLAP001` for a laptop, `ZMB001` for a MacBook), and a computer whose model names a type (a "MacBook Air" and a "MacBook" type) gets that type. From 1.4.0, each device type can also list words to match in **Settings > Device types** (for example "ThinkPad, Latitude"): a computer whose maker or model contains one gets that type. From 1.10.0, when the inventory uses computer names as asset tags (**Settings > General**), the app checks this computer's name: a name that is a valid, free tag becomes its tag; otherwise it warns, and offers to rename the computer (it asks for administrator rights; on Windows the new name applies after a restart) or to open the system's settings for it. From 1.5.0, a type can also take every laptop, desktop or all-in-one that no word catches (from 1.6.0, also every computer of any shape), for all makers, only Apple, or all but Apple: for example, all laptops except Apple ones go to "Portátil" while MacBooks get their own type. The type can still be changed before sending.

It does nothing in the background: it runs while it is open and stops when it is closed. On opening it checks for a newer version (see [Updates](#updates)) and offers it; it never installs one unasked. The password is only used to sign in, and is never saved. Its version is shown at the top right of the window.

| System | Versions | Download |
|---|---|---|
| macOS | 10.15 Catalina and later, Intel and Apple silicon (one universal app) | `.dmg` |
| Windows | 10 and 11, x64 and ARM | `-setup.exe` (or `.msi` for x64), or `-portable.exe` |
| Linux | Ubuntu 22.04, Debian 12, Fedora 38 and later, and similar; x64 and ARM | `.deb`, `.rpm` or `.AppImage` |

On Windows, the `-portable.exe` is the app itself, with nothing to install: copy it to a USB stick or a shared folder and open it on each computer. It uses Microsoft Edge WebView2, which Windows 11 and up-to-date Windows 10 already have; on a Windows 10 computer without it, use the installer, which adds it. Like the installed app, it keeps its settings in `%APPDATA%\inventario-registro`, not next to the `.exe`.

Windows and Linux also have a command-line version (`inventario-registro-cli_….zip`), with no window, for servers and remote sessions. On macOS, the app itself works as the command line (see below).

## Unsigned builds

The builds are not signed yet, so each system warns the first time:

- **macOS**: recent versions of macOS (15 and later) often refuse unsigned apps that were downloaded, even after **Open Anyway** in **System Settings > Privacy & Security**: the app only bounces in the Dock. The reliable way is to install it from Terminal into your own Applications folder, which also clears the marks macOS puts on downloads. With the disk image open, run:

  ```bash
  mkdir -p ~/Applications && ditto "/Volumes/Registro de equipos/Registro de equipos.app" ~/Applications/"Registro de equipos.app" && xattr -cr ~/Applications/"Registro de equipos.app" && codesign --force --deep --sign - ~/Applications/"Registro de equipos.app"
  ```

  Then open it from **Applications** in your home folder (in the Finder, **Go > Home**, or Launchpad). If you had copied it to the main **Applications** folder before, move that copy to the Bin: macOS remembers it refused it there. Signing the app with an Apple Developer ID would remove all of this.
- **Windows**: in the blue "Windows protected your PC" window, click **More info**, then **Run anyway**.
- **Linux**: no warning. Make an AppImage executable first: `chmod +x Registro*.AppImage`.

## Updates

From 0.13.0, the app checks GitHub for the latest **published** release each time it opens (drafts are never offered). When there is a newer version it shows **Update now** and **Later**; Update downloads it, installs it and opens the new version. Without internet access, or if GitHub does not answer, it carries on as usual.

- **macOS** (the app), **Windows** (`-setup.exe` or `.msi`) and **Linux** (`.AppImage`, `.deb`, `.rpm`) update in place. The Mac asks for an administrator password if the app's folder needs one; the Linux packages always do.
- The **portable Windows `.exe`** replaces its own file: the new version keeps the same name and folder, so shortcuts keep working. If the folder cannot be written to (a locked USB stick, a read-only share), it shows the download page instead.
- A copy that cannot install the update shows a link to the download page.

Every update file is signed when the release is built, and the app installs it only if the signature matches the key built into it, so nobody else can push an update to it, even though the repository is public. Copies older than 0.13.0 have no updater: install 0.13.0 by hand once.

## What needs administrator rights

On **Windows**, the app always starts as administrator: Windows asks for permission each time it opens (or, on an account without administrator rights, for an administrator's name and password). That way it can read everything, including battery health where the laptop reports it. The app first asks Windows' battery classes, then its battery report (`powercfg /batteryreport`); if neither has the figures, the laptop does not report them, and the app says so. The command-line version (`inventario-registro-cli`) does not ask; run it from a terminal opened as administrator to read everything.

On macOS everything works as a normal user. On **Linux**, the serial number and memory module details need administrator rights. Without them, the app asks for the serial number (it is usually on a label). To read everything, open it with `sudo`. With the AppImage, that is `sudo ./Registro*.AppImage`.

## The server's own certificate

An inventory installed with its own certificate (`TLS_MODE=internal`, for in-house addresses and IP addresses) is not trusted by computers out of the box. The first time, the app shows the SHA-256 fingerprint of the certificate authority behind it, and asks whether to trust it. Compare it with the one the inventory's installer printed. To see that fingerprint again, run this in the server's install folder:

```bash
docker compose exec -T caddy cat /data/caddy/pki/authorities/local/root.crt | openssl x509 -noout -fingerprint -sha256
```

The choice is remembered, and it applies only to that address. Nothing private, the password included, is sent before the certificate is trusted. An inventory with a public certificate, or one from the company's own certificate authority installed on the computer, needs none of this.

## Command line

The command-line version takes the same steps as the window. On macOS and Linux, the app itself also works this way when given options (for example `"/Applications/Registro de equipos.app/Contents/MacOS/inventario-registro" --help`). On Windows, use `inventario-registro-cli` instead: the app asks to run as administrator, which opens it in a separate window.

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
| `--rename` | When the inventory uses computer names as asset tags, rename this computer to its tag (needs administrator rights) |

| Exit code | Meaning |
|---|---|
| 0 | Done |
| 2 | Wrong options |
| 3 | Certificate not trusted |
| 4 | Sign-in refused |
| 5 | Other problem |

## Settings

The address starts filled in with `inventario.zoopa.local` and can be changed; a build can offer another one by setting `INVENTARIO_DEFAULT_SERVER` when compiling. The last address and email or username, and the certificates someone chose to trust, are saved in `prefs.json` in a folder named `inventario-registro` in the user's settings folder:

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

Change `version` in `Cargo.toml`, then push a tag such as `v0.2.0` (the same number: the release build refuses a tag that does not match). GitHub Actions builds every system, signs the update files and attaches everything to a **draft** release, with `latest.json`, the file the app reads to find updates. Check it and publish it by hand: publishing is what makes the apps offer it.

Signing needs the repository secret `TAURI_SIGNING_PRIVATE_KEY` (**Settings > Secrets and variables > Actions**): the private key whose public half is `plugins > updater > pubkey` in `app/src-tauri/tauri.conf.json`. Keep a copy of it somewhere safe: without it, the apps already installed cannot be sent another update, and every one would need the new key installed by hand. Builds without it (pull requests) are fine but unsigned; a release fails.

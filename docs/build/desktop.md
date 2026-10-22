# Septuvo VPN desktop app (Linux, N4)

**Development preview.** The app creates no tunnel and has no cryptography before the G1 security review. It is an unprivileged viewer and remote control for the `septuvod` service, which stays lab-only (N3-01).

- **Code:** `desktop/`, a separate Cargo workspace (N4-D4), PolyForm Noncommercial 1.0.0 (N4-D5).
- **Language:** all Septuvo-authored code is Rust (D-005).
- **Toolkit:** egui/eframe 0.36.2 with the OpenGL (glow) renderer and AccessKit screen-reader support (N4-D1).
- **Dependencies:** reviewed in [dependency-review-desktop.md](../reviews/dependency-review-desktop.md), pinned in `desktop/Cargo.lock`, and checked in CI by the native-code gate.

## What N4-01 delivers

The **Simple main window** of the approved design ([DEFAULT-UI.md](../ui/desktop/DEFAULT-UI.md), `Default_VPN_UI.png`; D-037), connected to the real service.

| Part | Where | Notes |
| --- | --- | --- |
| Main window | `desktop/src/main_window.rs`, `icons.rs`, `assets.rs`, `theme.rs` | Header lockup, connection selector, ring and shield, status heading and lines, status row, main button, divider, three footer tiles, Settings cog, in the approved order. Brand colours and the Inter typeface |
| View model | `desktop/src/state.rs` | Plain Rust, built from one service answer: every DEFAULT-UI §7.1 state the service can report today. **"Protected" only when all four checks pass** (D-020); otherwise never. Untrusted text is plain, without control characters, length-bounded |
| Live status | `desktop/src/service.rs` | A background thread asks the service every 1.5 s and on every change. A lost service shows **Service Unavailable** on the next refresh, never the last known state |
| Window and X | `desktop/src/app.rs`, `close.rs`, `prefs.rs` | 780 × 760 at first, minimum 520 × 580, size and position remembered, moved back on screen if needed. **X (Exit and Disconnect, D-041):** disconnects first, then exits; if disconnecting fails, the window stays with the error |
| Secondary windows | `desktop/src/shells.rs` | Connections, VPN Configuration, Help & Diagnostics and Settings open at their specified sizes and say plainly that they are not available yet (later N4 tasks) |
| Refuses root | `desktop/src/lib.rs` (`check_user`) | Exits with status 2 and an explanation. The service does the privileged work |
| Design preview | `desktop/examples/preview.rs` | Every state from a fake service (**sample data**, said in the window title), with a screenshot option. Development only: never part of `septuvo-vpn` |
| Native-code gate | `crates/septuvo-native-gate`, `desktop/native-allowlist.txt` | See below |

**Not yet, by design (later N4 tasks):** saved connections and the selector's list (N4-03; until then it reads **No Connections** and Connect is disabled with the reason), Send to System Tray (N4-06), the contents of the secondary windows, the Advanced interface. **The hero artwork** (globe and glow) waits for the owner's decision N4-D9; a plain ring and shield stand in for it.

**Window states (D-041):**

| Situation when X is pressed | What happens |
| --- | --- |
| Nothing active | The app exits |
| Kill switch on, or a session active | "Disconnecting…", the service is asked to disconnect; on success the app exits |
| Disconnect refused or failed | The window stays open: "Could not disconnect safely." with the reason. A second X exits the app only; the service keeps its state |
| Service not running | The window stays open with the same message (the app cannot tell what is in force). A second X exits |

## Screenshots (for the owner's visual sign-off)

Rendered with Mesa's software OpenGL. **Sample data** from a fake service; `protected` is a state today's service can never report (no tunnel before G1).

![Approved reference beside the N4-01 build](desktop-screens/compare-reference.png)

**Every state, 780 × 760:**

![All states at 780 x 760](desktop-screens/states-780x760.png)

**Every state, 520 × 580** (the minimum; the footer tiles stack and the window scrolls):

![All states at 520 x 580](desktop-screens/states-520x580.png)

## What N4-02 adds

The **Advanced main window** ([ADVANCED-UI.md](../ui/desktop/ADVANCED-UI.md), `Advanced_VPN_UI.png`) and **Interface Mode** switching.

| Part | Where | Notes |
| --- | --- | --- |
| Advanced window | `desktop/src/advanced_window.rs` | Header with the lockup and the **Home**, **Connections**, **Settings**, **Diagnostics** tabs (selected tab underlined); the Home panel (headline, text, status row, main button, connection card, IP row, hero placeholder); the three tiles. 980 × 640 at first, minimum 760 × 560; below 900 px the tiles stack and the window scrolls |
| Home view model | `desktop/src/home.rs` | Built from the same service answer as the Simple window, so both modes always agree. **You’re Connected** only when all four checks pass |
| Tiles (ADVANCED-UI §11) | `home.rs` | **Kill Switch** and **DNS Protection**: status only, never buttons. **Local Network Access**: the service's value; operable from N4-04 |
| Interface Mode | `desktop/src/shells.rs` (Settings → Interface), `app.rs` | `Simple` / `Advanced`, saved at once; the window takes the mode's size (each mode remembers its own). **Sends nothing to the service**: the connection, a pending Disconnect and notices carry over |
| Tab pages | `advanced_window.rs`, `shells.rs` | Connections, Settings and Diagnostics show the same content as the Simple windows |

**Truthful values (ADVANCED-UI §6, §11):**

| Tile or card | Value |
| --- | --- |
| Kill Switch | **On** when the service reports its rules installed; **Off** when nothing is active; **Failed** when the rules are missing (`error-kill-switch-missing`); **Unknown** otherwise, and whenever the service is lost |
| DNS Protection | **On** only when all four checks pass; **Off** when nothing is active; **Failed** with the kill switch; **Unknown** otherwise. The service does not yet report DNS enforcement on its own, so while blocking it is **Unknown** |
| Local Network Access | **Allowed** / **Not Allowed** from the service's `local_network` field; **Unknown** while the service does not report it (until N4-04) |
| Connection card | The connection's name and server address with the neutral icon; no country, city or flag. **No Connections** until N4-03 |
| Your New IP Address | **Not Connected**, or **Not Available** when connected: the address can only come from your own server over the verified tunnel (after G1). No copy button until there is an address |

**Screenshots**, sample data from a fake service (`protected` and its Local Network Access value cannot happen today):

![Approved Advanced reference beside the N4-02 build](desktop-screens/compare-advanced.png)

**Every state at 980 × 640, and the Settings tab:**

![Advanced states at 980 x 640](desktop-screens/advanced-states-980x640.png)

**Every state at 760 × 560** (the minimum; the tiles stack and the window scrolls):

![Advanced states at 760 x 560](desktop-screens/advanced-states-760x560.png)

## Build and run

From `desktop/`:

```sh
cargo build --locked --release
target/release/septuvo-vpn            # as your normal user, never as root
target/release/septuvo-vpn --runtime-dir /run/septuvo   # the default
```

To control the service your user must be in its control group (`septuvo`); without it the app shows the status and explains that disconnecting is not permitted.

**Preview with sample data:**

```sh
cargo run --locked --example preview -- blocking
cargo run --locked --example preview -- lost --size 520x580 --screenshot lost.png
cargo run --locked --example preview -- blocking --advanced            # Advanced, 980 x 640
cargo run --locked --example preview -- blocking --advanced --settings # the Settings tab
```

States: `empty`, `blocking`, `captive`, `failed`, `lost`, `disconnecting`, `protected`.

**Requirements on the machine:** a Wayland or X11 desktop with OpenGL (Mesa provides software OpenGL without a GPU) and `libxkbcommon`. These are operating-system libraries, loaded at run time.

**Preferences:** `$XDG_CONFIG_HOME/septuvo/desktop.conf` (normally `~/.config/septuvo/desktop.conf`): interface mode, close-button behaviour, and window size and position for each mode. Plain text, written atomically with mode 0600, at most 16 KiB read; a damaged file falls back to the defaults.

## Tests

```sh
cargo test --locked          # from desktop/
```

- **View model** (`src/state.rs`): every state, all 81 combinations of the four checks (only all-pass is Protected), the lost service, the Disconnecting override, untrusted text.
- **Close logic** (`src/close.rs`) and **preferences** (`src/prefs.rs`): each X case; round trip, damaged file, file mode.
- **Main window through the accessibility tree** (`tests/main_window.rs`, `egui_kittest`): what a screen reader would announce at 780 × 760, 520 × 580 and 1400 × 900:
  - the lockup's name, the selector, the tiles and Settings;
  - each state's heading and status, and never "Protected" or "Connected" for today's lab states;
  - disabled controls with their spoken reasons;
  - keyboard focus order (DEFAULT-UI §9.1);
  - the status following the service without a restart;
  - Disconnect, a failed disconnect, the tiles opening their windows;
  - every X case.
- **Advanced Home view model** (`src/home.rs`): every Home state and tile value; Kill Switch and DNS never On unless the service says so (DNS On only for all 16 check combinations' all-pass case); unreported values are Unknown; spoken tile names.
- **Advanced window and Interface Mode** (`tests/advanced_window.rs`): the approved structure at 980 × 640, 760 × 560 and 1400 × 900; Kill Switch and DNS are not buttons; Local Network Access is not operable yet; every Home state; tabs and the connection card; Disconnect and the X; keyboard order; and **switching Simple → Advanced → Simple with the kill switch blocking sends the fake service nothing but status requests**, and the state is unchanged.
- **Theme** (`src/theme.rs`): text colours meet WCAG AA contrast (4.5:1) on their backgrounds.
- **The X against the real service** (`tests/lab_close.rs`, ignored by default): run by `tests/lab/namespaces/desktop_close.sh` as root on a disposable machine. It starts `septuvod` in a lab namespace and connects; then, as a control-group member, the app's X must disconnect the service and exit; as a user outside the group, disconnect is refused, the window stays with the error, and a second X exits without changing the service.

```sh
# from the repository root, as root, on a disposable lab machine
cargo build --locked -p septuvo-daemon -p septuvo-cli --bins
sudo tests/lab/namespaces/desktop_close.sh target/debug/septuvod target/debug/septuvo \
    "$(cd desktop && cargo test --locked --no-run --test lab_close --message-format=json | python3 -c 'import sys,json; [print(m["executable"]) for m in map(json.loads, sys.stdin) if m.get("executable") and m["target"]["name"] == "lab_close"]')"
```

## The native-code gate (D-005, D-015)

Septuvo-authored code is Rust. External components may be in any language, but each is reviewed before it is adopted. The gate makes that mechanical for `desktop/`.

```sh
# from the repository root
cargo run --locked -p septuvo-native-gate -- deps --manifest-path desktop/Cargo.toml --allowlist desktop/native-allowlist.txt
cargo run --locked -p septuvo-native-gate -- artifacts --allowlist desktop/native-allowlist.txt \
    --target-dir desktop/target desktop/target/release/septuvo-vpn
```

**`deps` inspects every crate cargo builds for Linux.** That includes normal, build and dev dependencies, with the features actually enabled (`cargo metadata` and `cargo tree`, both `--locked`). It fails on:
- a native build tool (`cc`, `cmake`, `pkg-config`, `bindgen`…) not shown to be inert for the enabled features;
- a build script that runs a compiler;
- a `links` key;
- a library linked or loaded at run time;
- a bundled binary;
- a stale allowlist entry.

Each of these passes only if `desktop/native-allowlist.txt` lists it with a reason. **GTK, libadwaita, Qt, Skia, web-view and SDL crates always fail**: they are not the chosen toolkit (N4-D1; GTK is permitted under D-005 but was not chosen), so adopting one needs a new owner decision.

**`artifacts` checks after a real build.** The build directory must hold no native objects compiled by build scripts, and the binary may need only listed libraries (its ELF `DT_NEEDED` entries, read in Rust).

**Tests** (`crates/septuvo-native-gate/tests/gate.rs`) build throw-away workspaces:
- a pure-Rust one passes;
- one with a crate that compiles C in its build script fails on its `links` key, its build tool, the compiler and archiver it runs, and the library it links;
- after a real build of the C fixture, `artifacts` finds the compiled object.

Changing the allowlist is a dependency decision (D-015): it needs the owner's approval in a pull request.

## CI

`.github/workflows/desktop.yml`, Ubuntu 24.04 with the pinned toolchain, runs in order:
1. the native-code gate (`deps`);
2. `cargo fmt --check` and `clippy -D warnings`;
3. tests;
4. a release build and the native-code gate (`artifacts`);
5. the root-refusal check (`sudo septuvo-vpn` must exit 2);
6. a headless start-up under Xvfb with Mesa software OpenGL (still running after 10 s);
7. the desktop close lab against `septuvod` in a throw-away namespace;
8. screenshots of every state, Simple at 780 × 760 and 520 × 580 and Advanced at 980 × 640 and 760 × 560 (and its Settings tab), with both reference images, uploaded as the `desktop-main-window-screenshots` artifact.

Xvfb, Mesa, the X11/Wayland/keyboard libraries, nftables and iproute2 are Ubuntu packages installed on the runner only, to run the tests.

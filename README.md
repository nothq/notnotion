# notnotion

**Notion, without the browser.** 1.36 GB of RAM down to 258 MB. Same workspace, same pages, same pixels, no Electron.

notnotion is a native Notion client written in Rust on [GPUI](https://www.gpui.rs), the GPU-accelerated UI framework that powers the Zed editor. It signs in with the Notion session you already have and looks exactly like the app you use every day, except it is a single native binary drawing straight to the GPU.

## Download

| Platform | Get it |
| --- | --- |
| macOS, Apple Silicon | [notnotion-macos-arm64.dmg](https://github.com/nothq/notnotion/releases/latest/download/notnotion-macos-arm64.dmg) |
| macOS, Intel | [notnotion-macos-x86_64.dmg](https://github.com/nothq/notnotion/releases/latest/download/notnotion-macos-x86_64.dmg) |
| Linux, x86_64 | [notnotion-linux-x86_64.tar.gz](https://github.com/nothq/notnotion/releases/latest/download/notnotion-linux-x86_64.tar.gz) |
| Windows, x86_64 | [notnotion-windows-x86_64.zip](https://github.com/nothq/notnotion/releases/latest/download/notnotion-windows-x86_64.zip) |

Each download is the whole app: one native binary. No installer, no runtime, nothing else to fetch.

- **macOS**: open the .dmg and drag notnotion into Applications. The first time you open it, go to System Settings › Privacy & Security and click **Open Anyway**.
- **Linux**: `tar -xzf notnotion-linux-x86_64.tar.gz` and run `./notnotion-linux-x86_64/notnotion`.
- **Windows**: unzip and run `notnotion.exe`.

Every [release](https://github.com/nothq/notnotion/releases) is built from source by GitHub Actions.

## Why

Notion Desktop is Electron: a whole copy of Chromium plus Node.js, running a web app across a swarm of helper processes. All of that to show you pages of text.

notnotion throws the browser away. No DOM, no JavaScript, no garbage collector. Every pixel is Rust rendered straight to the GPU through Metal, Vulkan or DirectX, so it opens fast, scrolls at your display's refresh rate and barely registers in Activity Monitor. And it is pixel perfect: the sidebar, page editor, databases, menus and search match Notion, so there is nothing to relearn.

| Same page, 1320×860 window | Memory footprint |
| --- | --- |
| Notion Desktop (6 processes) | ~1.36 GB |
| **notnotion** (1 process) | **~258 MB** |

Measured with macOS's `footprint` tool on a MacBook Pro, 5 October 2026, with both apps showing the same page with an inline board database.

## Come build it

This is early, and that is the fun part. Here is what is open:

- **Halve the memory again.** notnotion still keeps each loaded page's Notion response as compact JSON so it can apply edits. Parsing it straight into typed records would let it drop that text too.
- **Notion AI.** The AI panel is drawn pixel for pixel but not wired to a model yet.
- **Every place we are a pixel off from Notion.** Put the two side by side and file what you see.

If you have ever watched Notion eat your laptop's memory, or wanted to ship real code on GPUI, pick one and open a PR.

## What's there

- Your workspace sidebar: favorites, teamspaces, private pages, recents, meetings and the inbox
- The block editor: text, headings, lists, to-dos, toggles, callouts, quotes, tables, columns, images, equations and synced blocks
- Slash commands, `@` mentions for dates, people and pages, markdown shortcuts, drag and drop, undo and page history
- Code blocks with syntax highlighting, and Mermaid diagrams
- Databases with board, table and calendar views, filters, sorts, grouping and properties
- Comments, sharing and permissions, favorites and page icons
- Quick Find search with live previews
- Light and dark mode, following the system

## How sign-in works

notnotion reuses the Notion session you already have. The first time it opens, notnotion quits Notion Desktop, relaunches it with a local debugging port, reads your session, and stores it in `~/.notnotion/auth.json`, readable only by you. Nothing is sent anywhere except to Notion.

Notion Desktop covers macOS and Windows. Notion ships no Linux app, so there, or anywhere Notion Desktop is missing, notnotion opens Notion's sign-in page in Chrome, Edge, Brave or Chromium, in a separate profile of its own, and picks up the session as soon as you sign in.

## Build and run

You need Rust 1.95 or newer. The first build compiles GPUI and takes a few minutes.

On macOS, with the Xcode command line tools:

```bash
xcode-select --install
git clone https://github.com/nothq/notnotion
cd notnotion
cargo run --release
```

On Linux, install GPUI's system libraries first:

```bash
sudo apt install pkg-config clang libasound2-dev libfontconfig-dev libvulkan-dev \
  libwayland-dev libx11-xcb-dev libxkbcommon-x11-dev
cargo run --release
```

On Windows, with the Visual Studio C++ build tools, `cargo run --release`.

`script/package` turns a release build into the downloads above, and `.github/workflows/release.yml` runs the whole thing for every platform when a `v*` tag is pushed.

## Layout

| Crate | What it does |
| --- | --- |
| `crates/notnotion` | The app: opens the window and hosts the Notion surface |
| `crates/notion` | The Notion client: API, session and caches (`live`), data types (`model`) and the GPUI interface (`ui`) |
| `crates/gpui-components` | Text input, tooltips and other shared widgets |
| `crates/app/model` | Theme, appearance and the surface interface |
| `crates/zed-syntax` | Tree-sitter syntax highlighting for code blocks |
| `crates/remote-image` | Safe image fetching for covers, icons and image blocks |
| `crates/local_cache`, `crates/secret_store` | Encrypted on-disk cache and the credential file |

## Disclaimer

notnotion is an independent project. It is not affiliated with or endorsed by Notion Labs.

## License

[AGPL-3.0](LICENSE). The highlight queries in `crates/zed-syntax/queries` come from the [Zed](https://github.com/zed-industries/zed) editor under GPL-3.0-or-later.

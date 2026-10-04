# notnotion

**Notion, without the browser.** 1.33 GB of RAM down to 333 MB. Same workspace, same pages, same pixels, no Electron.

notnotion is a native Notion client written in Rust on [GPUI](https://www.gpui.rs), the GPU-accelerated UI framework that powers the Zed editor. It signs in with the Notion session you already have and looks exactly like the app you use every day, except it is a single native binary drawing straight to the GPU.

## Why

Notion Desktop is Electron: a whole copy of Chromium plus Node.js, running a web app across a swarm of helper processes. All of that to show you pages of text.

notnotion throws the browser away. No DOM, no JavaScript, no garbage collector. Every pixel is Rust rendered straight to the GPU through Metal, so it opens fast, scrolls at your display's refresh rate and barely registers in Activity Monitor. And it is pixel perfect: the sidebar, page editor, databases, menus and search match Notion, so there is nothing to relearn.

| Same page, 1320×860 window | Memory footprint |
| --- | --- |
| Notion Desktop (6 processes) | ~1.33 GB |
| **notnotion** (1 process) | **~333 MB** |

Measured with macOS's `footprint` tool on a MacBook Pro, 4 October 2026, with both apps showing the same page with an inline board database.

## Come build it

This is early, and that is the fun part. Here is what is open:

- **Halve the memory again.** Most of notnotion's footprint is Notion's raw JSON record maps, kept after parsing. Parsing them straight into typed records would cut it to a fraction.
- **Notion AI.** The AI panel is drawn pixel for pixel but not wired to a model yet.
- **Linux and Windows.** Sign-in reuses the Notion Desktop session through macOS today.
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

notnotion reuses the session of the Notion Desktop app you already have. The first time it opens, notnotion quits Notion Desktop, relaunches it with a local debugging port, reads your session, and stores it in `~/.notnotion/auth.json`, readable only by you. Nothing is sent anywhere except to Notion.

So you need Notion Desktop installed and signed in, and for now that means macOS.

## Build and run

You need macOS, the Xcode command line tools and Rust 1.95 or newer.

```bash
xcode-select --install
git clone https://github.com/nothq/notnotion
cd notnotion
cargo run --release
```

The first build compiles GPUI and takes a few minutes.

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

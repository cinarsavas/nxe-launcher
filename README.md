# NXE Launcher

A fullscreen, console-style dashboard laid out like the 2008 "NXE" dashboard. It launches games on eleven consoles, using your own emulators or built-in cores, plus native PC ports and recompilations.

**[Try the live demo in your browser](https://cinarsavas.github.io/nxe-launcher/)** · **[Download for Windows](https://github.com/cinarsavas/nxe-launcher/releases/latest)**

> Fan project. Not affiliated with or endorsed by Microsoft, Sony or Nintendo. No games, BIOS files, keys or official artwork are included.


| Channel | Emulator | Games it finds |
|---|---|---|
| xbox 360 | Xenia (canary / edge) | `.iso`, `.zar`, `default.xex`, GOD / Arcade packages (real name + icon read from the package) |
| playstation 3 | RPCS3 | Disc folders (`PS3_GAME`), installed games in `dev_hdd0\game`, `.iso`. Real title + ICON0 come from PARAM.SFO; updates and DLC are skipped |
| playstation 2 | PCSX2 (Qt) | `.iso`, `.chd`, `.cso`, `.zso`, `.gz`, `.cue` (`.bin` only if it has no `.cue`) |
| switch | Eden, Citron, Sudachi, yuzu, Ryujinx / Ryubing | `.nsp`, `.xci`, `.nsz`, `.xcz` |
| GameCube | Dolphin, **or built-in core** | `.iso`, `.rvz`, `.gcz`, `.gcm`, `.ciso`, `.wbfs` (real title + exact GameTDB cover read from the disc ID) |
| Nintendo 64 | RMG, simple64, Project64, **or built-in core** | `.z64`, `.n64`, `.v64`, `.zip` |
| Nintendo DS | melonDS, **or built-in core** | `.nds` |
| Nintendo 3DS | Azahar / Lime3DS / Citra, **or built-in core** | `.3ds`, `.cci`, `.cxi`, `.3dsx` (must be decrypted) |
| PSP | PPSSPP, **or built-in core** | `.iso`, `.cso`, `.chd`, `.pbp` |
| PlayStation | DuckStation, **or built-in core** (SwanStation, DuckStation-based) | `.cue`, `.chd`, `.pbp`, `.m3u`, `.iso` (needs a PS1 BIOS) |
| Xbox | xemu, **or built-in core** (experimental) | `.iso` (XISO) (needs MCPX/BIOS/HDD files) |

Xbox 360 (Xenia) and PS3 (RPCS3) also have experimental built-in cores. Switch has none.

PS2 can use a built-in core too, but it needs your own PS2 BIOS in `runtime\system\pcsx2\bios`.

## Built-in cores (no emulator needed)
If a console has no emulator installed, the launcher uses a libretro core instead:
- GameCube: Dolphin core
- N64: Mupen64Plus-Next
- DS: melonDS DS
- 3DS: Azahar core
- PS2: LRPS2

Cores are **optional and never downloaded unless you ask**. If you start a game for a console that has no emulator, a small dialog offers to download that console's built-in core or to choose an emulator. You can also get a core under Settings > [console] > Built-in Core. Either way, the launcher downloads a menu-less RetroArch player (about 58 MB, only once for all consoles) plus the core. If you started from a game, it then launches that game by itself.

The player runs fullscreen with no menus or pop-ups, and quits as soon as the game closes, so it feels exactly like a standalone emulator. Guide, or holding Back + Start, still returns you to the dashboard. L3 + R3 opens the core's own menu if you ever need it.

**Settings > [console] > Play With** switches between three modes:
- Auto: your emulator; the core only if you've downloaded it and have no emulator
- Emulator
- Built-in Core

"Built-in Core" in the same menu downloads the core ahead of time. Everything is stored in `runtime\` next to the launcher.

## First launch
**Quick setup** opens automatically. It walks you through four steps:
1. Your name and theme colour
2. Your gamerpic
3. Emulators (auto-detected ones show a green dot)
4. Cover art

Games are added later, in Settings → Consoles.

Press A on **Continue** to move on. You can run setup again from Settings → General.

## Settings
Press **Start** (Tab on the keyboard) anywhere. The categories are:
- **General:** rescan your games, run Quick Setup again, open the settings file, exit
- **Consoles:** pick a console to open its own page:
  - **Emulator:** choose the program (.exe) or just the folder it's in
  - **Play With:** your emulator or the built-in core
  - **Built-in Core:** optional download
  - **Graphics:** resolution, aspect ratio, VSync, start fullscreen. These are written straight into the emulator's own settings file for PCSX2, Dolphin, DuckStation, PPSSPP, Xenia and xemu, and the original file is backed up as `*.nxe-backup`.
  - **Controls & More Settings:** opens the emulator itself
  - **Game folders** and **Launch Options**
- **Profile:** gamertag (typed right in the row) and gamerpic. Pick one of the 18 built-in gamerpics (Controller, Superstar, Lightning, Heart, Diamond, Night Owl, Ringworld, Encore, Royalty, Paw Print, Cube, Bloom, Liftoff, Boo, Ember, Summit, Tide, Halo Ring), one from your `theme\gamerpics` folder, your RetroAchievements profile picture (pick the trophy tile; it sends you to sign in if you aren't yet), or your own picture moved/zoomed to fit.
- **Display:** theme, animation frame rate (Match Display, or a 240/144/120/60/30 FPS cap), fullscreen, sounds, Show Version Number (off by default)
- **Cover Art**, **Achievements** (username, password, Sign In), **About**

Older layout notes:
- **General:** name, fullscreen, sounds, minimize while playing, rescan
- **Xbox 360 / PlayStation 3 / PlayStation 2 / Switch:** emulator exe, launch arguments, whether to use the emulator's game list, game folders (A removes one)
- **Cover art:** automatic downloads, look for missing covers again, clear the downloaded covers

The settings are still saved to `nxe-launcher.toml`, but you never need to edit that file.

## Motion
All movement runs on frame-rate-independent spring physics, so it's equally smooth at 60, 144, 180 or 240 Hz. The dashboard renders at your monitor's refresh rate. Animated elements:
- channels and tiles glide into place
- the focused tile "pops"
- the selected game zooms in as it launches
- tiles slide back in when you return from a game
- the green menu bar glides between rows
- panels rise in, and notifications slide up
- opening a page (Settings, the gamerpic picker, achievements…) zooms the dashboard towards you and fades it while the page grows in, PS4-style; moving deeper into a page or back out does a smaller zoom in the matching direction

## Look
The layout follows the NXE dashboard:
- channel names stacked top-left; Up/Down cycles through them and wraps around
- one row of tiles shrinking into the distance, sitting on a curved silver floor
- your profile top-right
- button hints bottom-left
- Settings, Quick Setup, the cover picker and the achievement list all use a slate panel with a green selection bar and a "Current Setting" column

Pick a colour under Settings > General > Theme: Green, Blue, Orange, Purple or Black.

On Windows the launcher uses Segoe UI automatically. To use a different font, drop a `font.ttf` into `theme\`.

## RetroAchievements
RetroAchievements covers **PS2, GameCube, N64 and DS** here. PS3, Xbox 360, Switch and 3DS have no RA sets. Your emulator does the actual unlocking: PCSX2, Dolphin and others each have their own RetroAchievements sign-in.

How to set it up:
1. Sign in once inside PCSX2 under Settings > Achievements. PCSX2 is what actually unlocks achievements while you play.
2. In the launcher, open Settings > Achievements, enter your username, then choose **Sign In** and type your password. This is the same sign-in emulators use. The password is never saved; only the session token is kept. Built-in cores get signed in automatically too.

What it adds:
- your RA points and profile picture in the top-right
- a progress bar under each PS2 game that has a set (matched by title)
- **X** on a PS2 game opens its achievement list, with badges, descriptions, points and unlock dates

## Cover art
Missing covers are downloaded automatically in the background:
- **PS2:** by disc serial, which is read from SYSTEM.CNF inside the ISO. Uses the same cover set as PCSX2.
- **PS2 / PS3 / Xbox 360:** libretro-thumbnails box art, matched by name
- **Switch:** the official icon, looked up by the `[TitleID]` in the file name
- PS3 games keep their own ICON0 icon. Xbox GOD packages use their built-in icon if nothing better is found.

If a cover is wrong, highlight the game and press **Y** (C on the keyboard). A row of options opens. Pick one with A, or choose "Automatic" to undo your pick. Your picks are saved in `theme\covers\<console>\`, and automatic downloads are cached in `theme\cache\`.

## Setup details
Put the launcher in a folder and put each emulator in its own subfolder next to it, for example:

```
NXE-Launcher\
  nxe-launcher.exe
  xenia\xenia_canary.exe
  rpcs3\rpcs3.exe
  pcsx2\pcsx2-qt.exe
  eden\eden.exe
```

The launcher finds them automatically. If you keep an emulator somewhere else, use its "emulator" tile at the end of that console's channel to choose the exe.

**You often don't need to add any folders.** The launcher imports what each emulator already knows about:
- RPCS3: everything installed in `dev_hdd0\game`, plus every disc game listed in `games.yml`
- PCSX2: the game list folders from `PCSX2.ini`, plus PCSX2's `covers` folder
- Eden / Citron / yuzu: the game directories in `qt-config.ini`
- Ryujinx: the `game_dirs` in `Config.json`
- Xenia: has no game list of its own, so add a folder with the "Add game folder" tile

## Going from the dashboard to a game and back
- Pressing A plays a short fade, then starts the emulator straight into the game in fullscreen, with the emulator's own menu hidden where it supports that. The launcher then brings the emulator's window to the front.
- To come back, quit the game the normal way, or press **Guide**, or hold **Back + Start** for 1.5 s. The launcher first asks the emulator to close like clicking X, which gives it time to save. If the emulator hasn't exited after 3 s, it forces it closed.
- When the emulator exits, the dashboard fades back in on the tile you launched from, and the game moves to the front of **home** under "Continue playing".

Default launch arguments:
- Xenia: `--fullscreen=true`
- PCSX2: `-fullscreen -nogui -batch --`
- RPCS3: `--no-gui --fullscreen`
- yuzu family: `-f -g`
- Ryujinx: `--fullscreen`

You can override them per console with `args` in the settings file.

## Controls
| Controller | Keyboard | Action |
|---|---|---|
| D-pad / left stick | Arrow keys | Up brings the channel listed above into place, Down goes back; Left/Right = tiles |
| A / Start | Enter | Select / play |
| B | Esc | Back to the first tile, then to home |
| X | X | Achievements (PS2) |
| Y | C | Change cover |
| Start | Tab | Settings |
| Guide | Home | Jump to home |
| Guide, or hold Back + Start while playing | | Return to the dashboard |
| | F11 | Toggle fullscreen |

## Theme folder (`theme\`)
- `background.png` / `.jpg` and `font.ttf` replace the background and the UI font.
- Cover art goes in `covers\<console>\` (console = `xbox360`, `ps3`, `ps2` or `switch`) or in `covers\`. Name each file `<ID>.png`, `<Game Name>.png` or `<file name>.png`. IDs come from the game: Xbox title ID, PS3 TITLE_ID, or a `[serial]` / `[TitleID]` in the file name.
- `gamerpics\*.png` / `.jpg`: your own gamerpics. They show up in the picker, named after the file (`Master_Chief.png` → "Master Chief"). Non-square pictures are centre-cropped.
- `sounds\move.wav`, `select.wav`, `back.wav`, `channel.wav` and `launch.wav` are optional UI sounds.

## Settings file (`nxe-launcher.toml`)
```toml
gamertag = "Player"
fullscreen = true
minimize_while_playing = false

[systems.ps3]
emulator = ""              # empty = auto-detect
args = []                  # empty = defaults above
game_dirs = []
import_from_emulator = true
```
The same keys exist under `[systems.xbox360]`, `[systems.ps2]` and `[systems.switch]`. Old v0.1 settings are migrated automatically.

## PC Ports
Native PC versions of console games (recompilations such as Zelda 64: Recompiled or Starship, and ports such as Ship of Harkinian) get their own **PC Ports** channel. They run on their own, no emulator needed.

- **Settings → Consoles → PC Ports → Add Port Folder:** pick a port's folder, or a folder with several ports in it (one sub-folder each). The game's .exe is picked for you; uninstallers, updaters and redistributables are skipped.
- **Add a Single Program:** pick one .exe directly.
- **GitHub Launcher (optional):** if you install ports with GitHub Launcher (SirDiabo, formerly N64RecompLauncher), point to `GithubLauncher.exe` or its folder. Everything it installed shows up with the same names, and the exe it remembers is used. Updates still happen in GitHub Launcher.
- Known ports get the original game's box art (e.g. Zelda 64: Recompiled → Majora's Mask). For anything else, press Y to pick a cover.

RecompLauncher (recomplauncher.com) has no desktop app yet; once it ships, its library can be imported the same way.

## Gamerpic folders
The gamerpic list loads every picture from `theme\gamerpics` (created empty, put your pictures there) and from any folder you add: **Add a Folder** in the gamerpic list, or Settings → Profile → Add a Gamerpic Folder. Pictures are centre-cropped to a square and get the same glossy 360-style finish as the built-in ones. Big folders scroll.

## RetroAchievements
Sign in once in Settings → Achievements (your normal username and password). Then:
- **Your points** show under your name, every game with achievements gets a progress bar, and **X** opens the game's achievement list (also from the Guide menu in a game).
- **Achievement unlocked pop-ups:** achievements are unlocked by the emulator while you play. When you come back to the dashboard, NXE Launcher asks RetroAchievements what's new since the game started and shows a 360-style "Achievement unlocked" pop-up for each one.
- **Sign In My Emulators Too** (Settings → Achievements) signs PCSX2, Dolphin and PPSSPP in with the same account, by writing the settings each of them writes after its own login. DuckStation encrypts its sign-in with a key tied to your PC, so sign in there once yourself. Built-in cores are signed in automatically.
- Optionally your RetroAchievements profile picture as gamerpic.

## Controllers
Xbox, PlayStation 3/4/5, Switch Pro, Joy-Cons and most other gamepads work. The button icons on screen follow the controller you last pressed a button on: Xbox letters, PlayStation shapes or Nintendo letters (Settings → Controller lets you fix one style).

- **Nintendo controllers** confirm with A (the right button) like on a Switch; switch to "bottom button confirms" in Settings → Controller if you prefer.
- **Joy-Cons:** one on its own is held sideways; connect both and they work together as one upright controller.
- **PS3 controllers on Windows** need the free DsHidMini driver (SDF or DS4Windows mode). PS4 and PS5 controllers work as they are.

## In a game: the Guide menu
Press Guide / PS / Home (or hold Back + Start for a second). A small menu opens over the game: **Resume Game**, **Achievements** (when the game has a RetroAchievements set) and **Quit Game** (asks once more before closing, so you don't lose progress by accident).

## Game details, play time, pins
Highlight a game and press **Y** (keyboard: C):
- **Time Played** and **Last Played.** NXE Launcher measures the time of every game it starts. It also reads the play time the emulators keep themselves, so time played before you used NXE Launcher counts too; whichever is higher is shown, with where it came from:

| Emulator | File it reads | Matched by |
|---|---|---|
| PCSX2 | `inis\playtime.dat` | disc serial (read from the .iso / .bin) |
| DuckStation | `playtime.dat` in its data folder | disc serial |
| RPCS3 | `GuiConfigs\persistent_settings.dat` | title ID |
| Dolphin | `Config\TimePlayed.ini` | 6-character game ID |
| PPSSPP | `[PlayTime]` in `ppsspp.ini` | disc ID (from the .iso) |
| Eden / Citron / Sudachi / yuzu | `play_time\*.bin` | title ID |
| Ryujinx | `games\<title id>\gui\metadata.json` | title ID |

  Compressed images (CHD, CSO) don't expose the serial, so for those only NXE Launcher's own time is shown. Xenia, xemu, melonDS, Azahar and RMG don't record play time.
- **Pin to Home** puts the game at the front of the Home channel.
- **Hide This Game** removes it from the dashboard (Settings → General → Hidden Games shows them again).
- **Pictures:** the title screen and a gameplay screenshot of the game (from libretro-thumbnails, the same project the covers come from); ports installed through Get Ports also show the screenshots from their project page. Left / Right scrolls through them.
- **Change Cover**, **Achievements**.

## Get Ports
PC Ports channel → **Get More Ports** (or Settings → Consoles → PC Ports → Get Ports) lists well-known ports — Zelda 64: Recompiled, Ship of Harkinian, 2 Ship 2 Harkinian, Starship, Spaghetti Kart, Banjo: Recompiled, Goemon 64: Recompiled, SM64 Co-op DX, Unleashed Recompiled, Marathon Recompiled, Dusk. Press A to download the latest Windows release from the project's GitHub page into the launcher's `ports` folder; when a new version comes out the row says **Update** and A updates it (the port's own settings and saves are kept). You always bring your own game files; each entry says which. Each port shows pictures of the game (Left / Right to scroll).

## Your profile file
Your name, gamerpic, theme, play time, pinned and hidden games are kept in **Documents\NXE Launcher\profile.json** — outside the launcher's folder, so they survive deleting or moving it. When you set the launcher up again, Quick Setup says **Welcome Back! Restore My Profile**. Settings → General can also save a copy anywhere (USB stick, cloud folder) and load one. Your RetroAchievements password/token is never stored in it.

## Discord status
Settings → General → Discord Status shows "Playing <game> · <console>" on your Discord profile. One-time setup: at discord.com/developers/applications click **New Application**, name it *NXE Launcher*, and copy its **Application ID** into Settings → General → Discord Application ID. The Discord desktop app must be running.

## Background music
Put MP3, OGG, FLAC or WAV files in `theme\music` and they play shuffled on the dashboard, like a 360 custom soundtrack. They fade out when a game starts and come back afterwards. Settings → Display: on/off, volume, next song.

## Updates
NXE Launcher checks GitHub for a new version when it starts (Settings → General → Check for Updates). Nothing installs on its own: **Update to vX** downloads it, swaps the exe and restarts. Settings, profile and games stay.

## License
MIT, see `LICENSE`. Trademarks belong to their owners.

## Building
`cargo build --release` on Windows, or cross-compile from Linux with `cargo build --release --target x86_64-pc-windows-gnu`.

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Ratatui](https://img.shields.io/badge/ratatui-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://ratatui.rs)
[![Docker](https://img.shields.io/badge/docker-%23000000.svg?style=for-the-badge&logo=docker&logoColor=white)](https://docker.com)
[![Make](https://img.shields.io/badge/Make-%23000000.svg?style=for-the-badge&logo=gnu&logoColor=white)](https://www.gnu.org/software/make/)
[![PostgreSQL](https://img.shields.io/badge/postgresql-%23000000.svg?style=for-the-badge&logo=postgresql&logoColor=white)](https://www.postgresql.org)

[![Neura Hustle Tracker](https://img.shields.io/badge/Neura%20Hustle%20Tracker-7f56da)](https://meetneura.ai) [![Powered by Neura AI](https://img.shields.io/badge/Powered%20by-Neura%20AI-7f56da)](https://meetneura.ai)

# Neura Hustle Tracker BETA

**Track what apps you use and how long you spend on them.**

This app runs in your terminal and shows you exactly where your time goes during work sessions. Built with Ratatui.

![Demo](src/screenshots/hustle-tracker-demo.GIF)

## What Does This Do?

- **Tracks your app usage** - Automatically monitors which programs you're using
- **Shows pretty charts** - See your time broken down by app and category
- **Saves your data** - Everything stored locally in your own PostgreSQL database
- **Works everywhere** - Linux, macOS, and Windows

## Quick Start (Easiest Way)

### Linux

Copy and paste this into your terminal:

```bash
sudo apt update && sudo apt install -y make docker.io curl git openssl && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && source ~/.cargo/env && git clone https://github.com/adolfousier/neura-hustle-tracker.git && cd neura-hustle-tracker && make run
```

That's it! The app will start tracking automatically.

### macOS

1. Install [Docker Desktop](https://docs.docker.com/desktop/install/mac-install/) first
2. Then paste this into Terminal:

```bash
brew install make git rustup-init && rustup-init -y && source ~/.cargo/env && git clone https://github.com/adolfousier/neura-hustle-tracker.git && cd neura-hustle-tracker && make daemon-start
```

3. View your stats anytime: `make view`

### Windows

1. Install [Docker Desktop](https://www.docker.com/products/docker-desktop/)
2. Open PowerShell as Administrator
3. Run this:

```powershell
powershell -Command "iwr -useb https://raw.githubusercontent.com/adolfousier/neura-hustle-tracker/main/src/scripts/windows_build/windows-install.ps1 | iex"
```

4. View your stats anytime: `hustle-view`

## Already Have Rust and Docker?

If you already have the prerequisites installed:

```bash
git clone https://github.com/adolfousier/neura-hustle-tracker
cd neura-hustle-tracker
make run
```

Done! The app handles everything else automatically.

## How to Use It

Once the app is running:

- **Tab** - Switch between Daily, Weekly, and Monthly views
- **h** - See your complete session history
- **r** - Rename apps to organize them better
- **Shift+C** - See all available commands
- **q** - Quit

The app tracks automatically. Just switch between your programs normally and it records everything.

## Two Ways to Run (Important!)

### Linux Users → Use "Unified Mode"

Run `make run` and you're done. Everything works in one window.

### macOS/Windows Users → Use "Daemon Mode"

You need two steps because of how these systems work:

1. **Start tracking in background**: `make daemon-start`
2. **Open the dashboard**: `make view`

Why? On macOS/Windows, if the tracking runs in the dashboard window, it can't see when you switch to other apps. Running it in the background fixes this.

**Commands for daemon mode:**

- `make daemon-start` - Start tracking
- `make view` - Open dashboard
- `make daemon-stop` - Stop tracking
- `make daemon-status` - Check if running

## What You Need

- **Computer**: Windows 10+, macOS 10.15+, or Linux with a desktop
- **Space**: About 500MB for Docker and dependencies
- **Permissions**:
  - macOS needs Screen Recording permission
  - Linux needs a desktop environment (GNOME, KDE, etc.)
  - Windows works out of the box

## Special Notes

**Wayland users (Linux)**: Install the [Window Calls extension](https://extensions.gnome.org/extension/4724/window-calls/) for GNOME to track windows properly.

**First time running**: The app creates secure database credentials automatically. You don't need to configure anything.

## Start on Boot (Optional)

Want the app to start automatically when you log in?

**Linux:**

```bash
mkdir -p ~/.config/autostart/
cp src/scripts/startup/neura-tracker.desktop ~/.config/autostart/
```

Edit the file and change `/path/to/neura-hustle-tracker` to your actual path.

**macOS:**

```bash
mkdir -p ~/Library/LaunchAgents/
cp src/scripts/startup/neura-tracker.plist ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/neura-tracker.plist
```

**Windows:**

```cmd
copy src\scripts\startup\neura-tracker.bat "%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\"
```

## Uninstall (Remove Everything)

Want to remove Neura Hustle Tracker completely? It will delete the app, all tracked data, and the database volume.

**Linux/macOS:**

```bash
make uninstall
```

Or use the dedicated script:

```bash
./src/scripts/uninstall.sh
```

**Windows:**

From PowerShell in the app directory:

```powershell
make uninstall
```

Or download and run the uninstall script:

```powershell
powershell -ExecutionPolicy Bypass -File src/scripts/windows_build/windows-uninstall.ps1
```

You'll be asked twice to confirm:
1. **First prompt**: Confirm you want to proceed
2. **Second prompt**: Type `yes` to confirm deletion (this prevents accidental removal)

The uninstall will:
- Stop the PostgreSQL database
- Remove the database volume (deletes all your tracked data)
- Delete the installation directory

## Comparison with Other Apps

| Feature | Neura Hustle Tracker | ActivityWatch | RescueTime |
|---------|---------------------|---------------|------------|
| Your data stays with you | ✅ | ✅ | ❌ |
| Open source | ✅ | ✅ | ❌ |
| Works offline | ✅ | ✅ | ❌ |
| Terminal interface | ✅ | ❌ | ❌ |
| Fast & lightweight | ✅ | ❌ | ❌ |

## Need Help?

- **App not starting?** Make sure Docker Desktop is running
- **Can't see windows?** Check permissions in System Settings
- **Database errors?** Try `make clean` then `make run`
- **Want to remove the app?** Use `make uninstall` to safely delete everything

## Contributing

Found a bug or want to add something? Check [CONTRIBUTING.md](CONTRIBUTING.md).

## License

See [LICENSE](LICENSE) file for details.

## Star History Chart

[![Star History Chart](https://api.star-history.com/svg?repos=adolfousier/neura-hustle-tracker&type=date&legend=top-left)](https://www.star-history.com/#adolfousier/neura-hustle-tracker&type=date&legend=top-left)

## ✨ Stay Tuned

⭐ **Star this repository to keep up with exciting updates and new releases, including powerful new features and productivity tracking capabilities!** ⭐

**Built with ❤️ by the Neura community** | [Website](https://meetneura.ai) | [Issues](https://github.com/adolfousier/neura-hustle-tracker/issues)

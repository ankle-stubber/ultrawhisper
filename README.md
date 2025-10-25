# UltraWhisper

> An experimental fork of Handy exploring voice-to-workflow automation

**UltraWhisper** is an independent project building on [Handy](https://github.com/cjpais/Handy)'s excellent speech-to-text foundation. We're exploring ideas around workflow automation, file outputs, and making transcription more powerful.

## 🧪 Status: Experimental

This is an active experiment in extending Handy with new capabilities. Things might break, APIs might change, and we're figuring it out as we go. That's part of the fun!

## 🎯 What This Is

- A fork of [Handy](https://github.com/cjpais/Handy) by [CJ Pais](https://github.com/cjpais)
- An exploration of workflow-based transcription
- A playground for voice automation ideas
- 100% open source and privacy-focused

## ✨ What's Working

All the great Handy features work out of the box:
- Multiple Whisper models for transcription
- Global keyboard shortcuts
- Voice Activity Detection
- Recording history
- Cross-platform support

## 🚧 What We're Adding

Some ideas we're experimenting with:
- **File outputs** - Save transcriptions directly to files/folders
- **Multiple workflows** - Different shortcuts for different outputs
- **Folder watching** - Batch process audio files
- **Integrations** - Direct connections to note-taking apps

## 🚀 Quick Start

```bash
# Clone and enter the repo
git clone https://github.com/ankle-stubber/ultrawhisper.git
cd ultrawhisper

# Install dependencies
bun install

# Download required model
mkdir -p src-tauri/resources/models
curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx

# Run in development
bun run tauri dev

# Build for production
bun run tauri build
```

## 🤝 Contributing

This is an experimental project and we're learning as we go. Feel free to:
- Open issues with ideas or bugs
- Submit PRs with improvements
- Fork and make your own version
- Share what you build!

## 📄 License

MIT License - Same as Handy. See [LICENSE](LICENSE) for details.

## 🙏 Credits

Huge thanks to [CJ Pais](https://github.com/cjpais) for creating Handy. This wouldn't exist without that solid foundation. See [ATTRIBUTION.md](ATTRIBUTION.md) for details.

---

**Note**: This is an independent project, not affiliated with the original Handy development.
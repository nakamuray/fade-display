# fade-display

Fade out display to black and after that execute specified command.

Intended to be used with display power off command to add fade out effect for wayland compositors such as niri.

Example:
```bash
fade-display niri msg action power-off-monitors
```

Requirements:

- gtk4
- gtk4-layer-shell

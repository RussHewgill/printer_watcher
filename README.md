
# Printer Watcher

Printer Watcher is a dashboard for monitoring the status of multiple printers.

## Features

- Notifications on print error
- Occasional bugs

## Installation

1. Download the app from the [releases](github.com/RussHewgill/printer_watcher/releases/latest) page. (printer_watcher for MacOS, printer_watcher.exe for windows)
2. Put the app in a folder somewhere
3. Install [GStreamer](https://gstreamer.freedesktop.org/documentation/installing/index.html?gi-language=c) and add it to the system PATH

## Instructions

1. Create a file in the same directory as the program, named `config.toml`
2. Paste the following template into it:
```toml
[[bambu]]
id = "some_random_name"
serial = "XXXXXXXXXXXXXXX"
name = "Name"
host = "XXX.XXX.XXX.XXX"
access_code = "12341234"
[[bambu]]
id = "some_random_name2"
serial = "XXXXXXXXXXXXXXX"
name = "Name2"
host = "XXX.XXX.XXX.XXX"
access_code = "12341234"
```
3. For each P1S, go to the 3rd menu, then select "WLAN"
  - Copy the `IP` and `Access Code` to the `host` and `access_code` fields
  - Go to Bambu Studio/Orca Slicer, and copy the serial from the `device` tab in the `update` menu

## Credits

(Some) Icons from [Icons8](https://icons8.com)

## If this is helpful to you, consider buying me a coffee:

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/I3I1W8O4I)



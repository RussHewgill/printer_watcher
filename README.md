
# Printer Watcher

Printer Watcher is a dashboard for monitoring the status of multiple printers.

## Features

- Notifications on print error
- 

## Instructions

OUTDATED

1. Create a file in the same directory as the program, named `config.yaml`
2. Paste the following template into it:
```yaml
printers:
- name: printer1
  host: XXX.XXX.XXX.XXX
  access_code: 12341234
  serial: XXXXXXXXXXXXXXX
- name: printer2
  host: XXX.XXX.XXX.XXX
  access_code: 56785678
  serial: XXXXXXXXXXXXXXX
```
3. For each P1S, go to the 3rd menu, then select "WLAN"
  - Copy the `IP` and `Access Code` to the `host` and `access_code` fields
  - Go to Bambu Studio/Orca Slicer, and copy the serial from the `device` tab in the `update` menu

## Credits

Icons from [Icons8](https://icons8.com)

## If this is helpful to you, consider buying me a coffee:

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/I3I1W8O4I)



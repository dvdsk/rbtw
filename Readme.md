Restarts the computer to windows from Linux. The next boot will be to the default OS again. The first run will ask for sudo, after that it can run without sudo. Copy into your path at `.local/bin` to restart to windows with 4 keystrokes. 

### Installing
- Download the latest binary from https://github.com/dvdsk/rbtw/releases and place it somewhere in your path. For example `.local/bin` or for a system-wide install `/usr/bin/chmod`.
- Using cargo and crates.io use: `cargo install +nightly rbtw`

The program will crash/halts if there is no windows bootloader or if there are multiple.

### Alternative
you can use the shell script:
```bash
#!/usr/bin/env bash
bootnext=$(efibootmgr | grep Windows | cut -c 5-8)
sudo efibootmgr --bootnext $bootnext
reboot 
```
This will however, need a sudo password on each run. You can set the SetUID bit for efibootmgr and remove the sudo call to get around this.

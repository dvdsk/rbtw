A single comment to reboot and start a specific OS without further user
interaction (bootloader nor sudo). The next boot after will be to the default OS
again. The first run will require you to configure the target OS, the setting is
stored inside the executable. A second run will ask for sudo, from then on
calling rbtw will instantly reboot tot the configured OS.

# Example usecase
Set up a number of commands to restart to different OS's. I have 4 OS's
currently installed: a general purpose linux, a linux for gaming, a windows
install for gaming and finally an OS for work. 

First I ensure I have 4 copies of rbtw with names that make sense to me (you
might want to chose these differently).
```bash
cp rbtw .local/bin/rbta
cp rbtw .local/bin/rbtg
cp rbtw .local/bin/rbtz
```

Then I configure each of these:
```bash
rbta --set-target abydos
rbtg --set-target gaming
rbtz --set-target zed
rbtw --set-target windows
```

Now I can restart to my gaming linux by calling `rbtg` and get a cup of tea
while its restarting :)

### Installing
You can either download the binary, it should work on any Linux system. Or
install from the source on *crates.io*, recommended if you have `cargo`
installed

- [recommended] Download the latest binary from https://github.com/dvdsk/rbtw/releases make it executable (`chmod +x rbtw`) and place it somewhere in your path. For example `.local/bin` or for a system-wide install `/usr/bin/rbtw`.
- Using `cargo` and *crates.io* use: `cargo install rbtw`.

### Alternative
For rebooting to windows you can use the shell script:
```bash
#!/usr/bin/env bash
bootnext=$(efibootmgr | grep Windows | cut -c 5-8)
sudo efibootmgr --bootnext $bootnext
reboot 
```
This will however, need a sudo password on each run. You can set the SetUID bit for efibootmgr and remove the sudo call to get around this.

For rebooting to a specific linux use a combination of `bootctl list` and
`bootctl set-oneshot`. See its help for more.


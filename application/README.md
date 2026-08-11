# Application build

The final standalone Windows executable is copied into this folder after a successful release build using the versioned name `paper-master-v<version>.exe`.

Generated `.exe` and `.lnk` files are intentionally excluded from Git. They can always be reproduced with `pnpm tauri build`.

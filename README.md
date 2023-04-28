# gb-emu
Game Boy emulator written in Rust.

## References
- Abstruct
  - https://gbdev.io/pandocs/
- CPU
  - http://marc.rawer.de/Gameboy/Docs/GBCPUman.pdf
  - https://gekkio.fi/files/gb-docs/gbctr.pdf
  - https://www.pastraiser.com/cpu/gameboy/gameboy_opcodes.html
- PPU
  - http://pixelbits.16-b.it/GBEDG/ppu/
- MBC
  - https://gbdev.gg8.se/wiki/articles/MBC1
  - http://pixelbits.16-b.it/GBEDG/mbcs/

## Status

- [ ] CPU
    - [ ] Instructions
    - [ ] Instruction timing
    - [ ] Interrupt handling
- [ ] PPU
    - [ ] Background
    - [ ] Window
    - [ ] Sprite
    - [ ] V-blank interrupt
    - [ ] LCDC STAT interrupt
    - [ ] Sprite and background priority
    - [ ] OAM bug
- [ ] Joypad
    - [ ] Joypad input
    - [ ] Joypad interrupt
- [ ] Catridge
    - [ ] Catridge loading
    - [ ] Data
    - [ ] MBC1
    - [ ] MBC3
    - [ ] MBC5
    - [ ] External RAM persistence
- [ ] Timer
    - [ ] Timer registers
    - [ ] Timer overflow interrupt
- [ ] APU

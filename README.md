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
      - [x] LD r, r'
      - [x] LD r, n
      - [x] LD r, (HL)
      - [x] LD (HL), r
      - [x] LD (HL), n
      - [x] LD A, (BC)
      - [x] LD A, (DE)
      - [x] LD (BC), A
      - [x] LD (DE), A
      - [x] LD A, (nn)
      - [x] LD (nn), A
      - [x] LDH A, (C)
      - [x] LDH (C), A
      - [x] LDH A, (n)
      - [x] LDH (n), A
      - [x] LD A, (HL-)
      - [x] LD (HL-), A
      - [x] LD (HL+), A
      - [x] LD rr, nn
      - [x] LD (nn), SP
      - [x] LD SP, HL
      - [x] PUSH rr
      - [x] POP rr
      - [x] ADD r
      - [x] ADD (HL)
      - [x] ADD n
      - [x] ADC r
      - [x] ADC (HL)
      - [x] ADC n
      - [x] SUB r
      - [x] SUB (HL)
      - [x] SUB n
      - [x] SBC r
      - [x] SBC (HL)
      - [x] SBC n
      - [x] CP r
      - [x] CP (HL)
      - [x] CP n
      - [x] INC r
      - [x] INC (HL)
      - [x] DEC r
      - [x] DEC (HL)
      - [x] RLCA
      - [x] RLA
      - [x] RRCA
      - [x] RRA
      - [x] RLC r
      - [x] RLC (HL)
      - [x] RL r
      - [x] RL (HL)
      - [x] RRC r
      - [x] RRC (HL)
      - [x] RR r
      - [x] RR (HL)
      - [x] AND r
      - [x] AND (HL)
      - [x] AND n
      - [x] OR r
      - [x] OR (HL)
      - [x] OR n
      - [x] XOR r
      - [x] XOR (HL)
      - [x] XOR n
      - [x] CCF
      - [x] SCF
      - [x] DAA
      - [x] CPL
      - [x] JP nn
      - [x] JP HL
      - [x] JP cc, nn
      - [x] JR e
      - [x] JR cc, e
      - [x] CALL nn
      - [x] CALL cc, nn
      - [x] RET
      - [x] RET cc
      - [x] RETI
      - [ ] RST n
      - [ ] HALT
      - [ ] STOP
      - [ ] DI
      - [ ] EI
      - [x] NOP
    - [x] Interrupt handling
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

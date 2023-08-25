# gb-emu
Game Boy emulator written in Rust.

## References
- Abstruct
  - https://gbdev.io/pandocs/
  - https://github.com/pokemium/gb-docs-ja (for japanese)
- CPU
  - http://marc.rawer.de/Gameboy/Docs/GBCPUman.pdf
  - https://gekkio.fi/files/gb-docs/gbctr.pdf
  - https://www.pastraiser.com/cpu/gameboy/gameboy_opcodes.html
- PPU
  - https://github.com/Hacktix/GBEDG/blob/master/ppu/index.md
- Timer
  - https://github.com/Hacktix/GBEDG/blob/master/timers/index.md
- OAM DMA transfer
  - https://github.com/Hacktix/GBEDG/blob/master/dma/index.md
- MBC
  - https://gbdev.gg8.se/wiki/articles/MBC1
  - https://github.com/Hacktix/GBEDG/blob/master/mbcs/index.md

## Test suite

### [Blargg's tests](https://gbdev.gg8.se/files/roms/blargg-gb-tests/)

| Test         | gb-emu |
| ------------ | :----: |
| cpu instrs   | :+1:   |
| dmg sound 2  | :x:    |
| instr timing | :+1:   |
| mem timing   | :+1:   |
| mem timing 2 | :+1:   |
| oam bug 2    | :x:    |
| cgb sound 2  | :x:    |

## Status

- [x] CPU
    - [x] Instructions
      - [x] 8bit operators
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
        - [x] SLA r
        - [x] SLA (HL)
        - [x] SRA r
        - [x] SRA (HL)
        - [x] SRL r
        - [x] SRL (HL)
        - [x] SWAP r
        - [x] SWAP (HL)
        - [x] BIT r
        - [x] BIT (HL)
        - [x] SET r
        - [x] SET (HL)
        - [x] RES r
        - [x] RES (HL)
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
        - [x] RST n
        - [x] HALT
        - [x] STOP
        - [x] DI
        - [x] EI
        - [x] NOP
      - [x] 16bit operators
        - [x] LD rr, nn
        - [x] LD (nn), SP
        - [x] LD SP, HL
        - [x] LD HL, SP+e
        - [x] PUSH rr
        - [x] POP rr
        - [x] ADD HL, rr
        - [x] ADD SP, e
        - [x] INC rr
        - [x] DEC rr
    - [x] Interrupt handling
- [x] PPU
    - [x] Background
    - [x] Window
    - [x] Sprite
    - [x] V-blank interrupt
    - [x] LCDC STAT interrupt
    - [x] Sprite and background priority
- [x] Joypad
    - [x] Joypad input
    - [x] Joypad interrupt
- [ ] Catridge
    - [x] Catridge loading
    - [x] Data
    - [x] MBC1
    - [ ] MBC3
    - [ ] MBC5
    - [ ] External RAM persistence
- [x] Timer
    - [x] Timer registers
    - [x] Timer overflow interrupt
- [ ] APU

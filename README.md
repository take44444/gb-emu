# gb-emu
Game Boy emulator written in Rust.

![boot_rom](./boot_rom.png)
![instr_timing](./cpu_instrs.png)
![instr_timing](./instr_timing.png)
![mem_timing](./mem_timing.png)
![mario](./mario.png)
![pokemon1](./pokemon1.png)
![pokemon2](./pokemon2.png)

## References
- Abstruct
  - https://gbdev.io/pandocs/
  - https://github.com/pokemium/gb-docs-ja (for japanese)
- MBC
  - https://gbdev.io/pandocs/MBCs.html
  - https://github.com/Hacktix/GBEDG/blob/master/mbcs/index.md
- CPU
  - http://marc.rawer.de/Gameboy/Docs/GBCPUman.pdf
  - https://github.com/AntonioND/giibiiadvance/blob/master/docs/TCAGBD.pdf
  - https://gekkio.fi/files/gb-docs/gbctr.pdf
  - https://izik1.github.io/gbops/index.html
- PPU
  - https://gbdev.io/pandocs/Rendering.html
  - https://github.com/Hacktix/GBEDG/blob/master/ppu/index.md
- APU
  - https://gbdev.io/pandocs/Audio.html
  - https://nightshade256.github.io/2021/03/27/gb-sound-emulation.html
- Timer
  - https://github.com/Hacktix/GBEDG/blob/master/timers/index.md
- Joypad
  - https://gbdev.io/pandocs/Joypad_Input.html
- OAM DMA transfer
  - https://gbdev.io/pandocs/OAM_DMA_Transfer.html
  - https://github.com/Hacktix/GBEDG/blob/master/dma/index.md

## Test suite

### [Blargg's tests](https://gbdev.gg8.se/files/roms/blargg-gb-tests/)

| Test         | gb-emu |
| ------------ | :----: |
| cpu instrs   | :+1:   |
| dmg sound    | :x:    |
| instr timing | :+1:   |
| mem timing   | :+1:   |
| mem timing 2 | :+1:   |
| oam bug      | :x:    |
| cgb sound    | :x:    |

## Status

- [ ] Catridge
    - [x] Catridge loading
    - [x] Data
    - [x] MBC1
    - [ ] MBC3
    - [ ] MBC5
    - [ ] External RAM persistence
- [x] CPU
    - [x] Instructions
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
- [x] APU
  - [x] Channel1
    - [x] Envelope
    - [x] Sweep
  - [x] Channel2
    - [x] Envelope
  - [x] Channel3
  - [x] Channel4
    - [x] Envelope
- [x] Timer
    - [x] Timer registers
    - [x] Timer overflow interrupt
- [x] Joypad
    - [x] Joypad input
    - [x] Joypad interrupt
- [x] OAM DMA transfer
- [x] Saving game data

import init, { GameBoyHandle, AudioHandle } from "../wasm/gbemu_web.js"

const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d");

const rom_input = document.getElementById("rom_input");
const on = document.getElementById("on");
const off = document.getElementById("off");

ctx.fillStyle = "black";
ctx.fillRect(0.0, 0.0, canvas.width, canvas.height);

async function main() {
  await init();

  let rom_file = null;
  let gameboy = null;
  let running = false;

  rom_input.oninput = (_) => {
    rom_file = rom_input.files[0];
  };

  on.onclick = (_) => {
    if (rom_file === null || gameboy !== null) {
      return;
    }

    let reader = new FileReader();

    reader.readAsArrayBuffer(rom_file);
    reader.onloadend = (_) => {
      let rom = new Uint8Array(reader.result);
      running = true;

      let audio = AudioHandle.new();
      let intervalID = null;

      gameboy = GameBoyHandle.new(rom, new Uint8Array(), (buffer) => {
        audio.append(buffer);
      });

      function main_loop() {
        if (!running) {
          gameboy = null;
          audio = null;

          clearInterval(intervalID);
          return;
        }

        if (audio.length() < 15) {
          let framebuffer = gameboy.emulate_frame();
          let image_data = new ImageData(framebuffer, 160, 144);

          createImageBitmap(image_data, {
            resizeQuality: "pixelated",
            resizeWidth: 640,
            resizeHeight: 576,
          }).then((bitmap) => {
            ctx.drawImage(bitmap, 0.0, 0.0);
          });
        }
      }
      intervalID = setInterval(main_loop, 16);
    };
  };

  off.onclick = (_) => {
    running = false;
    ctx.fillRect(0.0, 0.0, canvas.width, canvas.height);
  };

  document.onkeydown = (event) => {
    if (gameboy !== null) {
      gameboy.key_down(event.code);
    }
  }

  document.onkeyup = (event) => {
    if (gameboy !== null) {
      gameboy.key_up(event.code);
    }
  }
}

main()

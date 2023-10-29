import { io } from "https://cdn.socket.io/4.4.1/socket.io.esm.min.js";
import init, { GameBoyHandle, AudioHandle } from "../wasm/gbemu_web.js"

const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d");

const rom_input = document.getElementById("rom_input");
const sav_input = document.getElementById("sav_input");
const on = document.getElementById("on");
const off = document.getElementById("off");

const connect = document.getElementById("connect");
const disconnect = document.getElementById("disconnect");
const connection = document.getElementById('connection')

ctx.fillStyle = "black";
ctx.fillRect(0.0, 0.0, canvas.width, canvas.height);

async function main() {
  await init();

  const socket = io('http://localhost:3000');
  socket.on('connect', () => {
    document.getElementById('me').textContent = 'ID: ' + socket.id;
  });
  socket.on('leave', () => {
    connection.textContent = 'Disconnected';
    console.log('leaved');
  });
  socket.on('join', (data) => {
    connection.textContent = 'Connected to ' + data;
    console.log('joined ' + data);
  });
  connect.onsubmit = (e) => {
    e.preventDefault();
    socket.emit('join', connect.data.value);
  };
  disconnect.onclick = (e) => {
    e.preventDefault();
    socket.emit('leave');
  };
  socket.connect();

  let rom_file = null;
  let sav_file = null;
  let gameboy = null;
  let running = false;

  rom_input.oninput = (_) => {
    rom_file = rom_input.files[0];
  };
  sav_input.oninput = (_) => {
    sav_file = sav_input.files[0];
  };

  on.onclick = (_) => {
    if (rom_file === null || gameboy !== null) {
      return;
    }
    let rom = null;
    let sav = new Uint8Array();
    let lock_step = false;
    let run = () => {
      running = true;

      gameboy = GameBoyHandle.new(rom, sav);
      let audio = AudioHandle.new();
      let intervalID = null;

      let apu_callback = (buffer) => {
        audio.append(buffer);
      };
      let serial_callback = (val) => {
        socket.emit('master', val);
        lock_step = true;
      };

      gameboy.set_callback(apu_callback, serial_callback);
      socket.on('master', (data) => {
        if (!gameboy.serial_is_master()) {
          socket.emit('slave', gameboy.serial_data());
          gameboy.serial_receive(data);
        }
      });
      socket.on('slave', (data) => {
        if (gameboy.serial_is_master()) {
          lock_step = false;
          intervalID = setInterval(main_loop, 16);
          gameboy.serial_receive(data);
        }
      });

      function main_loop() {
        if (!running) {
          gameboy = null;
          audio = null;

          clearInterval(intervalID);
          return;
        }

        if (audio.length() < 15) {
          while (true) {
            if (lock_step) {
              clearInterval(intervalID);
              return;
            }
            if (gameboy.emulate_cycle()) {
              break;
            }
          }
          let framebuffer = gameboy.frame_buffer();
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

    let reader1 = new FileReader();
    reader1.readAsArrayBuffer(rom_file);
    reader1.onloadend = (_) => {
      rom = new Uint8Array(reader1.result);
      if (sav_file === null) {
        run();
      } else {
        let reader2 = new FileReader();
        reader2.readAsArrayBuffer(sav_file);
        reader2.onloadend = (_) => {
          sav = new Uint8Array(reader2.result);
          run();
        };
      }
    };
  };

  off.onclick = (_) => {
    running = false;
    ctx.fillRect(0.0, 0.0, canvas.width, canvas.height);
    socket.on('master', (_) => {});
    socket.on('slave', (_) => {});
  };

  document.onkeydown = (e) => {
    if (gameboy !== null) {
      gameboy.key_down(e.code);
    }
  }

  document.onkeyup = (e) => {
    if (gameboy !== null) {
      gameboy.key_up(e.code);
    }
  }
}

main()

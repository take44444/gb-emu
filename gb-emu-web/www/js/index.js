import { io } from "https://cdn.socket.io/4.4.1/socket.io.esm.min.js";
import init, { GameBoyHandle, AudioHandle } from "../wasm/gbemu_web.js"

const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d");

const rom_input = document.getElementById("rom_input");
const on = document.getElementById("on");
const off = document.getElementById("off");

const connect = document.getElementById("connect");
const disconnect = document.getElementById("disconnect");
const connection = document.getElementById('connection')

ctx.fillStyle = "black";
ctx.fillRect(0.0, 0.0, canvas.width, canvas.height);

async function main() {
  await init();

  let connected = false;
  const socket = io('http://localhost:8000');
  socket.on('connect', () => {
    document.getElementById('me').textContent = 'ID: ' + socket.id;
    connected = true;
  });
  socket.on('leave', () => {
    connection.textContent = 'Disconnected';
    console.log('leaved');
  });
  socket.on('join', (data) => {
    connection.textContent = 'Connected to ' + data;
    console.log('joined' + data);
  });
  connect.onsubmit = (e) => {
    e.preventDefault();
    socket.emit('join', connect.data.value);
  };
  disconnect.onclick = (_) => {
    e.preventDefault();
    socket.emit('leave');
  };
  socket.connect();

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

      gameboy = GameBoyHandle.new(rom, new Uint8Array(),
        (buffer) => {
          audio.append(buffer);
        },
        (val) => {
          if (connected) {
            socket.emit('master', val);
          } else {
            gameboy.serial_receive(0xFF);
          }
        }
      );
      socket.on('master', (data) => {
        if (!gameboy.serial_is_master()) {
          socket.emit('slave', gameboy.serial_data());
          gameboy.serial_receive(data);
        }
      });
      socket.on('slave', (data) => {
        if (gameboy.serial_is_master()) {
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

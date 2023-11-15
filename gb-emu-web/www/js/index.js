import { io } from "https://cdn.socket.io/4.4.1/socket.io.esm.min.js";
import init, { GameBoyHandle, AudioHandle } from "../wasm/gbemu_web.js"

const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d");

const rom_input = document.getElementById("rom_input");
const sav_input = document.getElementById("sav_input");
const on = document.getElementById("on");
const off = document.getElementById("off");
const save = document.getElementById("save");

const connect = document.getElementById("connect");
const disconnect = document.getElementById("disconnect");
const connection = document.getElementById('connection')

ctx.fillStyle = "black";
ctx.fillRect(0.0, 0.0, canvas.width, canvas.height);

async function main() {
  await init();

  const socket = io();
  socket.on('connect', () => {
    document.getElementById('me').textContent = 'ID: ' + socket.id;
  });
  socket.on('leave', () => {
    connection.textContent = 'Disconnected';
  });
  socket.on('join', (data) => {
    connection.textContent = 'Connected to ' + data;
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
    if (rom_file === null || gameboy !== null) return;
    let rom = null;
    let sav = new Uint8Array();
    let lockstep = false;
    let received_data = null;
    let run = () => {
      running = true;

      gameboy = GameBoyHandle.new(rom, sav);
      let audio = AudioHandle.new();
      let intervalID = null;

      let apu_callback = (buffer) => {
        audio.append(buffer);
      };
      let send_callback = (val) => {
        socket.emit('master', val);
        lockstep = true;
      };

      gameboy.set_callback(apu_callback, send_callback);
      socket.on('master', (data) => {
        if (!gameboy.serial_is_master()) {
          received_data = data;
        }
      });
      socket.on('slave', (data) => {
        if (gameboy.serial_is_master()) {
          lockstep = false;
          intervalID = setInterval(main_loop, 15);
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
            if (lockstep) {
              // if (gameboy.serial_is_master()) {
              clearInterval(intervalID);
              return;
              // }
            }
            if (received_data !== null) {
              // const rollbacked = gameboy.rollback(received_data.rollback);
              // console.log('rollbacked: ' + rollbacked);
              socket.emit('slave', gameboy.serial_data());
              gameboy.serial_receive(received_data);
              for (let i = 0; i < 100000; i++) {}
              received_data = null;
            }
            if (gameboy.emulate_cycle()) {
              let framebuffer = gameboy.frame_buffer();
              let image_data = new ImageData(framebuffer, 160, 144);
              createImageBitmap(image_data, {
                resizeQuality: "pixelated",
                resizeWidth: 640,
                resizeHeight: 576,
              }).then((bitmap) => {
                ctx.drawImage(bitmap, 0.0, 0.0);
              });
              return;
            }
          }
        }
      }
      intervalID = setInterval(main_loop, 15);
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

  save.onclick = (_) => {
    if (!running || gameboy === null) return;
    const sav_data = gameboy.save();
    if (sav_data.length === 0) return;
    var a = document.createElement("a");
    a.style = "display: none";
    document.body.appendChild(a);

    var blob = new Blob([sav_data.buffer], {type: "octet/stream"}),
    url = window.URL.createObjectURL(blob);

    a.href = url;
    a.download = gameboy.title() + ".SAV";
    a.click();
    window.URL.revokeObjectURL(url);
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

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
const connection = document.getElementById('connection');

ctx.fillStyle = "black";
ctx.fillRect(0.0, 0.0, canvas.width, canvas.height);

async function main() {
  await init();

  let rom = null;
  let sav = new Uint8Array();
  let gameboy = null;
  let running = false;
  let other = null;

  rom_input.oninput = (_) => {
    let reader1 = new FileReader();
    reader1.readAsArrayBuffer(rom_input.files[0]);
    reader1.onloadend = (_) => {
      rom = new Uint8Array(reader1.result);
    };
  };
  sav_input.oninput = (_) => {
    let reader2 = new FileReader();
    reader2.readAsArrayBuffer(sav_input.files[0]);
    reader2.onloadend = (_) => {
      sav = new Uint8Array(reader2.result);
    }
  }

  const socket = io();
  socket.on('connect', () => {
    document.getElementById('me').textContent = 'ID: ' + socket.id;
  });
  socket.on('leave', () => {
    other = null;
    if (gameboy !== null) gameboy.disconnect();
    connection.textContent = 'Disconnected';
  });
  socket.on('join', (id) => {
    if (other !== null) return;
    other = id;
    if (gameboy !== null) socket.emit('sync', gameboy.to_json());
  });
  socket.on('sync', (data) => {
    if (gameboy !== null) gameboy.connect(data);
    connection.textContent = 'Connected to ' + other;
  });
  socket.on('keydown', (code) => {
    if (gameboy !== null) {
      gameboy.key_down2(code);
    }
  });
  socket.on('keyup', (code) => {
    if (gameboy !== null) {
      gameboy.key_up2(code);
    }
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

  on.onclick = (_) => {
    if (rom === null || gameboy !== null) return;
    running = true;

    gameboy = GameBoyHandle.new(rom, sav);
    let audio = AudioHandle.new();
    let intervalID = null;

    gameboy.set_apu_callback((buffer) => {
      audio.append(buffer);
    });

    function main_loop() {
      if (!running) {
        gameboy = null;
        clearInterval(intervalID);
        return;
      }
      if (audio.length() < 15) {
        while (!gameboy.emulate_cycle()) {}
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
    intervalID = setInterval(main_loop, 15);
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
  };

  document.onkeydown = (e) => {
    if (gameboy !== null) {
      gameboy.key_down(e.code);
      socket.emit('keydown', e.code);
    }
  }

  document.onkeyup = (e) => {
    if (gameboy !== null) {
      gameboy.key_up(e.code);
      socket.emit('keyup', e.code);
    }
  }
}

main()

import { io } from "https://cdn.socket.io/4.4.1/socket.io.esm.min.js";
import init, { GameBoyHandle, AudioHandle } from "../wasm/gbemu_web.js"

const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d");

const rom_button = document.getElementById("rom_button");
const sav_button = document.getElementById("sav_button");
const rom_input = document.getElementById("rom_input");
const sav_input = document.getElementById("sav_input");
const power = document.getElementById("power");
const save = document.getElementById("save");

const server = document.getElementById("server");
const connect = document.getElementById("connect");
const disconnect = document.getElementById("disconnect");

ctx.fillStyle = "black";
ctx.fillRect(0.0, 0.0, canvas.width, canvas.height);

function resize_canvas() {
  let width, height;
  if ((document.documentElement.clientWidth-250)*144 > (document.documentElement.clientHeight-230)*166) {
    height = document.documentElement.clientHeight - 230;
    width = height * 160/144;
  } else {
    width = document.documentElement.clientWidth - 250;
    height = width * 144/160;
  }
  canvas.setAttribute('width', width);
  canvas.setAttribute('height', height);
  ctx.fillRect(0.0, 0.0, canvas.width, canvas.height);
}

async function main() {
  await init();

  resize_canvas();

  let socket = null;

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
    rom_button.classList.add('specified');
  };
  sav_input.oninput = (_) => {
    let reader2 = new FileReader();
    reader2.readAsArrayBuffer(sav_input.files[0]);
    reader2.onloadend = (_) => {
      sav = new Uint8Array(reader2.result);
    }
    sav_button.classList.add('specified');
  }
  connect.onsubmit = (e) => e.preventDefault();
  disconnect.onclick = (e) => e.preventDefault();

  window.onresize = resize_canvas;

  server.onclick = (_) => {
    if (socket === null) {
      socket = io();
      socket.on('connect', () => {
        document.getElementById('me').textContent = socket.id;
        server.classList.add('connected');
      });
      socket.on('disconnect', () => {
        document.getElementById('me').textContent = socket.id;
        server.classList.remove('connected');
      });
      socket.on('leave', () => {
        other = null;
        if (gameboy !== null) gameboy.disconnect();
        document.getElementById('connection').textContent = '-';
        document.getElementById("player").classList.remove('connected');
      });
      socket.on('join', (id) => {
        if (other !== null) return;
        other = id;
        if (gameboy !== null) socket.emit('sync', gameboy.to_json());
      });
      socket.on('sync', (data) => {
        if (gameboy !== null) gameboy.connect(data);
        document.getElementById('connection').textContent = other;
        document.getElementById("player").classList.add('connected');
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
    }
    if (socket.connected) socket.disconnect();
    else socket.connect();
  };

  document.getElementById("connection-modal").onclick = (_) => {
    document.getElementById("connection-modal").style.visibility = 'hidden';
    document.getElementById("connection-modal-window").style.visibility = 'hidden';
    document.getElementById("connection-modal-window").classList.remove('on');
  };

  document.getElementById("player").onclick = (_) => {
    document.getElementById("connection-modal").style.visibility = 'visible';
    document.getElementById("connection-modal-window").style.visibility = 'visible';
    document.getElementById("connection-modal-window").classList.add('on');
  };

  power.onclick = (_) => {
    if (running) {
      power.classList.remove('on');
      running = false;
      ctx.fillRect(0.0, 0.0, canvas.width, canvas.height);
      return;
    }
    if (rom === null || gameboy !== null) return;
    power.classList.add('on');
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
          resizeWidth: canvas.width,
          resizeHeight: canvas.height,
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

  document.onkeydown = (e) => {
    if (gameboy !== null) {
      gameboy.key_down(e.code);
      if (socket.connected && other !== null) socket.emit('keydown', e.code);
    }
  };

  document.onkeyup = (e) => {
    if (gameboy !== null) {
      gameboy.key_up(e.code);
      if (socket.connected && other !== null) socket.emit('keyup', e.code);
    }
  };
}

main()

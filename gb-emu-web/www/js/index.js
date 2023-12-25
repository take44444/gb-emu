import { io } from "https://cdn.socket.io/4.4.1/socket.io.esm.min.js";
import init, { GameBoyHandle, AudioHandle } from "../wasm/gbemu_web.js"

const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d");

let socket = null;

let rom = null;
let sav = new Uint8Array();
let gameboy = null;
let audio = null;
let other = null;
let mode = null;
let initialized = false;

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

function initialize_socket() {
  socket = io();
  socket.on('connect', () => {
    document.getElementById('me').textContent = socket.id;
    document.getElementById("server").classList.add('connected');
  });
  socket.on('disconnect', () => {
    document.getElementById('me').textContent = '-';
    document.getElementById("server").classList.remove('connected');
  });
  socket.on('leave', () => {
    other = null;
    mode = null;
    initialized = false;
    if (gameboy !== null) gameboy.disconnect();
    document.getElementById('connection').textContent = '-';
    document.getElementById("player").classList.remove('connected');
  });
  socket.on('slave', (id) => {
    if (other !== null) return;
    other = id;
    if (gameboy !== null) socket.emit('init', gameboy.to_json());
    mode = 'slave';
  });
  socket.on('master', (id) => {
    if (other !== null) return;
    other = id;
    if (gameboy !== null) socket.emit('init', gameboy.to_json());
    mode = 'master';
  });
  socket.on('init', (data) => {
    if (gameboy !== null && !initialized) gameboy.connect(data);
    initialized = true;
    document.getElementById('connection').textContent = other;
    document.getElementById("player").classList.add('connected');
  });
  socket.on('sync', (data) => {
    if (gameboy === null || mode !== 'slave' || !initialized) return;
    document.getElementById("sync").classList.add('synchronizing');
    let json = JSON.parse(data);
    if (gameboy !== null) {
      gameboy.sync(JSON.stringify(json.slave), JSON.stringify(json.master));
      gameboy.set_apu_callback((buffer) => audio.append(buffer));
    }
    setTimeout((_) => document.getElementById("sync").classList.remove('synchronizing'), 700);
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
  document.getElementById("connect").onsubmit = (e) => {
    e.preventDefault();
    if (socket === null || !socket.connected || other !== null) return;
    socket.emit('join', document.getElementById("connect").data.value);
  };
  document.getElementById("disconnect").onclick = (e) => {
    e.preventDefault();
    if (socket === null || !socket.connected) return;
    socket.emit('leave');
  };
}

function initialize_dom() {
  document.getElementById("rom_input").oninput = (_) => {
    let reader1 = new FileReader();
    reader1.readAsArrayBuffer(document.getElementById("rom_input").files[0]);
    reader1.onloadend = (_) => {
      rom = new Uint8Array(reader1.result);
    };
    document.getElementById("rom_button").classList.add('specified');
  };
  document.getElementById("sav_input").oninput = (_) => {
    let reader2 = new FileReader();
    reader2.readAsArrayBuffer(document.getElementById("sav_input").files[0]);
    reader2.onloadend = (_) => {
      sav = new Uint8Array(reader2.result);
    }
    document.getElementById("sav_button").classList.add('specified');
  };
  document.getElementById("connect").onsubmit = (e) => e.preventDefault();
  document.getElementById("disconnect").onclick = (e) => e.preventDefault();

  window.onresize = resize_canvas;

  document.getElementById("server").onclick = (_) => {
    if (socket === null) initialize_socket();
    if (socket.connected) socket.disconnect();
    else socket.connect();
  };

  document.getElementById("connection-modal").onclick = (_) => {
    document.getElementById("connection-modal").style.visibility = 'hidden';
    document.getElementById("connection-modal-window").style.visibility = 'hidden';
    document.getElementById("connection-modal-window").classList.remove('on');
  };

  document.getElementById("player").onclick = (_) => {
    if (socket === null || !socket.connected) return;
    document.getElementById("connection-modal").style.visibility = 'visible';
    document.getElementById("connection-modal-window").style.visibility = 'visible';
    document.getElementById("connection-modal-window").classList.add('on');
  };

  document.getElementById("power").onclick = (_) => {
    if (gameboy !== null) {
      document.getElementById("power").classList.remove('on');
      gameboy = null;
      ctx.fillRect(0.0, 0.0, canvas.width, canvas.height);
      return;
    }
    if (rom === null) return;
    document.getElementById("power").classList.add('on');

    gameboy = GameBoyHandle.new(rom, sav);
    audio = AudioHandle.new();
    gameboy.set_apu_callback((buffer) => audio.append(buffer));

    let sync_loop_id = null;
    let main_loop_id = null;
    function main_loop() {
      if (gameboy === null) {
        clearInterval(main_loop_id);
        return;
      }
      if (audio.length() < 15) {
        while (!gameboy.emulate_cycle()) {}
        let image_data = new ImageData(gameboy.frame_buffer(), 160, 144);
        createImageBitmap(image_data, {
          resizeQuality: "pixelated",
          resizeWidth: canvas.width,
          resizeHeight: canvas.height,
        }).then((bitmap) => {
          ctx.drawImage(bitmap, 0.0, 0.0);
        });
      }
    }
    function sync_loop() {
      if (gameboy === null) {
        clearInterval(sync_loop_id);
        return;
      }
      if (socket !== null && socket.connected && mode === 'master') {
        document.getElementById("sync").classList.add('synchronizing');
        socket.emit('sync', `{"master":${gameboy.to_json()},"slave":${gameboy.to_json2()}}`);
        setTimeout((_) => document.getElementById("sync").classList.remove('synchronizing'), 700);
      }
    }
    main_loop_id = setInterval(main_loop, 15);
    sync_loop_id = setInterval(sync_loop, 2000);
  };

  document.getElementById("save").onclick = (_) => {
    if (gameboy === null) return;
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

async function main() {
  await init();

  resize_canvas();

  initialize_dom();
}

main();

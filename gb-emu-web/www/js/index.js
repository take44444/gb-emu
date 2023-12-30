import { io } from "https://cdn.socket.io/4.4.1/socket.io.esm.min.js";
import init, { GameBoyHandle, AudioHandle } from "../wasm/gbemu_web.js"
import { canvas, ctx, resize_canvas, Dom } from "./dom.js"

const SYNC_INTERVAL = 100000;

function assert(bool) {
  if (!bool) throw new Error("assertion error!");
}

class GameBoyManager {
  constructor() {
    this.gameboy = null;
    this.audio = null;
    this.rom = null;
    this.sav = null;

    this.last_sync_cycle = 0;
    this.synchronized_gameboy = null;
    this.input_history = [{cycle: 0, history: []}, {cycle: 0, history: []}];
    this.cycle = 0;
  }

  init(rom, sav) {
    this.gameboy = GameBoyHandle.new(rom, sav);
    this.audio = AudioHandle.new();
    this.gameboy.set_apu_callback((buffer) => this.audio.append(buffer));
    this.rom = rom;
    this.sav = sav;

    this.last_sync_cycle = 0;
    this.synchronized_gameboy = null;
    this.input_history = [{cycle: 0, history: []}, {cycle: 0, history: []}];
    this.cycle = 0;
  }

  power_on() {
    this.init(this.rom, this.sav);
  }

  power_off() {
    this.gameboy = null;

    this.last_sync_cycle = 0;
    this.synchronized_gameboy = null;
    this.input_history = [{cycle: 0, history: []}, {cycle: 0, history: []}];
    this.cycle = 0;
  }

  disconnect() {
    this.gameboy.disconnect();

    this.last_sync_cycle = 0;
    this.synchronized_gameboy = null;
    this.input_history = [{cycle: 0, history: []}, {cycle: 0, history: []}];
    this.cycle = 0;
  }

  is_on() {
    return this.gameboy !== null;
  }

  emulate_cycle() {
    assert(this.gameboy !== null);
    if (this.synchronized_gameboy !== null) this.cycle++;
    return this.gameboy.emulate_cycle();
  }

  key_down(code) {
    assert(this.gameboy !== null);
    if (this.synchronized_gameboy !== null) {
      if (this.input_history[0].cycle === this.last_sync_cycle + SYNC_INTERVAL) return;
      if (this.input_history[0].history.length > 0 && this.input_history[0].history.slice(-1)[0].cycle === this.cycle) return;
      if (!this.gameboy.key_down(code)) return;
      this.input_history[0].history.push({cycle: this.cycle, down: true, code: code});
    } else this.gameboy.key_down(code);
  }

  key_up(code) {
    assert(this.gameboy !== null);
    if (this.synchronized_gameboy !== null) {
      if (this.input_history[0].cycle === this.last_sync_cycle + SYNC_INTERVAL) return;
      if (this.input_history[0].history.length > 0 && this.input_history[0].history.slice(-1)[0].cycle === this.cycle) return;
      if (!this.gameboy.key_up(code)) return;
      this.input_history[0].history.push({cycle: this.cycle, down: false, code: code});
    } else this.gameboy.key_up(code);
  }

  sync_init(other_gameboy_json) {
    assert(this.gameboy !== null);
    assert(this.synchronized_gameboy === null);
    this.gameboy.connect(other_gameboy_json);
    this.last_sync_cycle = 0;
    this.synchronized_gameboy = this.gameboy._clone();
    this.input_history = [{cycle: 0, history: []}, {cycle: 0, history: []}];
    this.cycle = 0;
  }

  sync() {
    const target_cycle = this.last_sync_cycle + SYNC_INTERVAL;
    assert(this.gameboy !== null);
    assert(this.synchronized_gameboy !== null);
    assert(this.input_history[0].cycle === target_cycle);
    assert(this.input_history[1].cycle === target_cycle);
    this.gameboy = this.synchronized_gameboy;
    this.gameboy.set_apu_callback(() => {});
    this.cycle = this.last_sync_cycle;
    while (this.cycle < target_cycle) {
      while (this.input_history[0].history.length > 0) {
        const input = this.input_history[0].history[0];
        if (this.cycle !== input.cycle) break;
        this.input_history[0].history.shift();
        if (input.down) this.gameboy.key_down(input.code);
        else this.gameboy.key_up(input.code);
      }
      while (this.input_history[1].history.length > 0) {
        const input = this.input_history[1].history[0];
        if (this.cycle !== input.cycle) break;
        this.input_history[1].history.shift();
        if (input.down) this.gameboy.key_down2(input.code);
        else this.gameboy.key_up2(input.code);
      }
      this.emulate_cycle();
    }
    assert(this.input_history[0].history.length === 0);
    assert(this.input_history[1].history.length === 0);
    assert(this.cycle === target_cycle);
    this.last_sync_cycle = target_cycle;
    this.synchronized_gameboy = this.gameboy._clone();
    this.gameboy.set_apu_callback((buffer) => this.audio.append(buffer));
  }
}

class GameBoyRunner {
  constructor() {
    ctx.fillStyle = "black";
    ctx.fillRect(0.0, 0.0, canvas.width, canvas.height);
    this.gameboy = new GameBoyManager();
    this.dom = new Dom();
    this.main_loop_id = null;

    document.getElementById("server").onclick = (_) => {
      if (this.dom.socket === null) this.initialize_socket();
      if (this.dom.socket.connected) dom.socket.disconnect();
      else this.dom.socket.connect();
    };

    document.getElementById("power").onclick = (_) => {
      if (this.gameboy.is_on()) {
        document.getElementById("power").classList.remove('on');
        this.gameboy.power_off();
        ctx.fillRect(0.0, 0.0, canvas.width, canvas.height);
        return;
      }
      if (this.dom.rom === null) {
        alert("Specify rom file.")
        return;
      }
      document.getElementById("power").classList.add('on');

      this.gameboy.init(this.dom.rom, this.dom.sav);

      this.main_loop_id = setInterval(() => this.main_loop(), 15);
    };

    document.getElementById("save").onclick = (_) => {
      if (!this.gameboy.is_on()) return;
      const sav_data = this.gameboy.gameboy.save();
      if (sav_data.length === 0) return;
      var a = document.createElement("a");
      a.style = "display: none";
      document.body.appendChild(a);

      var blob = new Blob([sav_data.buffer], {type: "octet/stream"}),
      url = window.URL.createObjectURL(blob);

      a.href = url;
      a.download = this.gameboy.gameboy.title() + ".SAV";
      a.click();
      window.URL.revokeObjectURL(url);
    };

    document.onkeydown = (e) => {
      if (this.gameboy.is_on()) this.gameboy.key_down(e.code);
    };

    document.onkeyup = (e) => {
      if (this.gameboy.is_on()) this.gameboy.key_up(e.code);
    };
  }

  main_loop() {
    if (!this.gameboy.is_on()) {
      clearInterval(main_loop_id);
      return;
    }
    if (this.gameboy.audio.length() < 15) {
      let vblank = false;
      while (true) {
        if (this.gameboy.cycle === this.gameboy.last_sync_cycle + SYNC_INTERVAL) {
          this.gameboy.input_history[0].cycle = this.gameboy.cycle;
          this.dom.socket.emit('input_history', this.gameboy.input_history[0]);
          if (this.gameboy.input_history[1].cycle ===  this.gameboy.cycle) {
            this.gameboy.sync();
            this.dom.synchronized();
          } else {
            clearInterval(this.main_loop_id);
            this.dom.synchronizing();
          }
        }
        vblank = this.gameboy.emulate_cycle();
        if (vblank) break;
      }
      if (vblank) {
        let image_data = new ImageData(this.gameboy.gameboy.frame_buffer(), 160, 144);
        createImageBitmap(image_data, {
          resizeQuality: "pixelated",
          resizeWidth: canvas.width,
          resizeHeight: canvas.height,
        }).then((bitmap) => {
          ctx.drawImage(bitmap, 0.0, 0.0);
        });
      }
    }
  }

  initialize_socket() {
    this.dom.socket = io();
    this.dom.socket.on('input_history', (data) => {
      if (!this.gameboy.is_on()) return;
      if (this.gameboy.synchronized_gameboy === null) return;
      this.gameboy.input_history[1] = data;
      if (this.gameboy.input_history[0].cycle === this.gameboy.last_sync_cycle + SYNC_INTERVAL) {
        this.gameboy.sync();
        this.dom.synchronized();
        this.main_loop_id = setInterval(() => this.main_loop(), 15);
      } else this.dom.synchronizing();
    });
    this.dom.socket.on('connect', () => this.dom.server_connected());
    this.dom.socket.on('disconnect', () => {
      this.dom.server_disconnected();
      this.dom.leaved();
      if (this.gameboy.is_on()) this.gameboy.disconnect();
    });
    this.dom.socket.on('syncinit1', (data) => {
      assert(this.gameboy.is_on());
      if (this.gameboy.synchronized_gameboy !== null) return;
      this.dom.joined(data.id);
      this.gameboy.sync_init(data.gameboy);
      this.dom.socket.emit('syncinit2', this.gameboy.gameboy.to_json());
    });
    this.dom.socket.on('syncinit2', (data) => {
      assert(this.gameboy.is_on());
      if (this.gameboy.synchronized_gameboy !== null) return;
      this.dom.joined(data.id);
      this.gameboy.sync_init(data.gameboy);
    });
    this.dom.socket.on('leave', () => {
      this.dom.leaved();
      if (this.gameboy.is_on()) this.gameboy.disconnect();
    });
    document.getElementById("connect").onsubmit = (e) => {
      e.preventDefault();
      if (this.dom.socket === null || !this.dom.socket.connected) {
        alert("You are disconnected.");
        return;
      }
      if (!this.gameboy.is_on()) {
        alert("Your gameboy is off.");
        return;
      }
      if (this.gameboy.synchronized_gameboy !== null) {
        alert("Your gameboy is already connected to another one.")
        return;
      }
      this.dom.socket.emit('syncinit1', {id: document.getElementById("connect").data.value, gameboy: this.gameboy.gameboy.to_json()});
    };
    document.getElementById("disconnect").onclick = (e) => {
      e.preventDefault();
      if (this.dom.socket === null || !this.dom.socket.connected) return;
      this.dom.socket.emit('leave');
    };
  }
}

async function main() {
  await init();

  resize_canvas();

  new GameBoyRunner();
}

main();

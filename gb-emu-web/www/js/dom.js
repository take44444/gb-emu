export const canvas = document.getElementById("canvas");
export const ctx = canvas.getContext("2d");

export function resize_canvas() {
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

export class Dom {
  constructor() {
    this.socket = null;
    this.rom = null;
    this.sav = new Uint8Array();
    window.onresize = resize_canvas;
    document.getElementById("rom_input").oninput = (_) => {
      let reader1 = new FileReader();
      reader1.readAsArrayBuffer(document.getElementById("rom_input").files[0]);
      reader1.onloadend = (_) => {
        this.rom = new Uint8Array(reader1.result);
      };
      document.getElementById("rom_button").classList.add('specified');
    };
    document.getElementById("sav_input").oninput = (_) => {
      let reader2 = new FileReader();
      reader2.readAsArrayBuffer(document.getElementById("sav_input").files[0]);
      reader2.onloadend = (_) => {
        this.sav = new Uint8Array(reader2.result);
      }
      document.getElementById("sav_button").classList.add('specified');
    };
    document.getElementById("connect").onsubmit = (e) => e.preventDefault();
    document.getElementById("disconnect").onclick = (e) => e.preventDefault();
    document.getElementById("connection-modal").onclick = (_) => {
      document.getElementById("connection-modal").style.visibility = 'hidden';
      document.getElementById("connection-modal-window").style.visibility = 'hidden';
      document.getElementById("connection-modal-window").classList.remove('on');
    };
    document.getElementById("player").onclick = (_) => {
      if (this.socket === null || !this.socket.connected) return;
      document.getElementById("connection-modal").style.visibility = 'visible';
      document.getElementById("connection-modal-window").style.visibility = 'visible';
      document.getElementById("connection-modal-window").classList.add('on');
    };
  }

  server_connected() {
    document.getElementById('me').textContent = this.socket.id;
    document.getElementById("server").classList.add('connected');
  }

  server_disconnected() {
    document.getElementById('me').textContent = '-';
    document.getElementById("server").classList.remove('connected');
  }

  joined(id) {
    document.getElementById('connection').textContent = id;
    document.getElementById("player").classList.add('connected');
  }

  leaved() {
    document.getElementById('connection').textContent = '-';
    document.getElementById("player").classList.remove('connected');
  }

  synchronizing() {
    document.getElementById("sync").classList.add('synchronizing');
  }

  synchronized() {
    document.getElementById("sync").classList.remove('synchronizing');
  }
}

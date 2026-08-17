// isoteri-vm.js
//
// Interpreter JavaScript untuk bytecode Isoteri format "isoteri-web-bytecode-v1"
// (dihasilkan oleh `isoteri ekspor-web program.iso -o program.isoweb.json`).
//
// Ini BUKAN kompilasi Isoteri -> WASM asli. Ini adalah VM bytecode yang
// semantiknya ditulis ulang persis mengikuti VM Rust di src/lib.rs
// (fungsi eksekusi/eksekusi_satu), supaya perilaku program identik di
// browser maupun native, tanpa perlu target wasm32-unknown-unknown.
// Lihat docs/FILOSOFI.md bagian Fase 3 untuk alasan pendekatan ini.
//
// Belum didukung di sini (lihat runtime/web/README.md untuk detail):
//   - 'ulang selaras' (JalankanSelaras) -- tidak ikut diekspor dari Rust,
//     lihat instr_ke_json() di src/lib.rs.
//   - unduh(), baca_berkas(), tulis_berkas() -- I/O sistem/jaringan sinkron,
//     tidak relevan/sengaja tidak diimplementasi buat sandbox browser.
//
// Pemakaian dasar:
//   const vm = new IsoteriVM(bundleJson, { tampilkan: (baris) => console.log(baris) });
//   vm.jalankan();

class IsoteriError extends Error {}

class IsoteriVM {
  /**
   * @param {object} bundle - hasil parse JSON dari `isoteri ekspor-web`.
   * @param {object} [opsi]
   * @param {(baris: string) => void} [opsi.tampilkan] - dipanggil tiap kali
   *   program Isoteri mengeksekusi `tampilkan ...`. Default: console.log.
   */
  constructor(bundle, opsi = {}) {
    if (bundle.format !== "isoteri-web-bytecode-v1") {
      throw new IsoteriError(`Format bundel tidak dikenal: ${bundle.format}`);
    }
    this.tampilkanFn = opsi.tampilkan || ((baris) => console.log(baris));
    this.konstanta = bundle.konstanta.map(decodeValue);
    this.fungsi = bundle.fungsi.map((f) => ({
      nama: f.nama,
      paramCount: f.param_count,
      localSlotCount: f.local_slot_count,
      kode: f.kode,
      paramFlat: f.param_flat, // array of (null | string[])
    }));
    this.namaKeIndeks = bundle.nama_ke_indeks;
    this.topKode = bundle.top_kode;

    this.globals = new Array(bundle.global_slot_count).fill(KOSONG);
    this.domRegistry = new Map(); // id -> Element asli (lihat panggilDom)
    this._domIdCounter = 0;
    this.stack = [];
    this.locals = [];
    this.iterStack = []; // {items: Value[], pos: number}[]
    this.handlerStack = []; // {stackBase, target, slotKind:'Local'|'Global', slot}[]
    this.barisSekarang = 0;
  }

  jalankan() {
    this.eksekusi(this.topKode, 0);
  }

  /** Setara fn eksekusi() di Rust: satu "frame" (top-level ATAU satu pemanggilan fungsi). */
  eksekusi(kode, localsBase) {
    const baseStack = this.stack.length;
    const baseHandler = this.handlerStack.length;
    let pc = 0;

    while (pc < kode.length) {
      try {
        const hasil = this.eksekusiSatu(kode, localsBase, pc);
        if (hasil.selesai) {
          this.stack.length = baseStack;
          this.handlerStack.length = baseHandler;
          return hasil.nilai;
        }
        pc = hasil.pc;
      } catch (e) {
        if (!(e instanceof IsoteriError)) throw e;
        const pesan = e.message.startsWith("Baris ") ? e.message : `Baris ${this.barisSekarang}: ${e.message}`;
        if (this.handlerStack.length > baseHandler) {
          const handler = this.handlerStack.pop();
          this.stack.length = handler.stackBase;
          if (handler.slotKind === "Local") this.locals[localsBase + handler.slot] = teks(pesan);
          else this.globals[handler.slot] = teks(pesan);
          pc = handler.target;
        } else {
          this.stack.length = baseStack;
          this.handlerStack.length = baseHandler;
          throw new IsoteriError(pesan);
        }
      }
    }
    this.stack.length = baseStack;
    this.handlerStack.length = baseHandler;
    return null;
  }

  /** Jalankan SATU instruksi. Return {selesai:false, pc} atau {selesai:true, nilai}. */
  eksekusiSatu(kode, localsBase, pc) {
    const instr = kode[pc];
    const op = instr[0];
    const S = this.stack;

    switch (op) {
      case "TandaiBaris": this.barisSekarang = instr[1]; return { selesai: false, pc: pc + 1 };
      case "PushK": S.push(this.konstanta[instr[1]]); return { selesai: false, pc: pc + 1 };
      case "LoadGlobal": S.push(this.globals[instr[1]]); return { selesai: false, pc: pc + 1 };
      case "StoreGlobal": this.globals[instr[1]] = S.pop(); return { selesai: false, pc: pc + 1 };
      case "LoadLocal": S.push(this.locals[localsBase + instr[1]]); return { selesai: false, pc: pc + 1 };
      case "StoreLocal": this.locals[localsBase + instr[1]] = S.pop(); return { selesai: false, pc: pc + 1 };
      case "BinOp": {
        const r = S.pop(), l = S.pop();
        S.push(evalBinOp(l, instr[1], r));
        return { selesai: false, pc: pc + 1 };
      }
      case "Lompat": return { selesai: false, pc: instr[1] };
      case "LompatJikaSalah": {
        const v = S.pop();
        return { selesai: false, pc: truthy(v) ? pc + 1 : instr[1] };
      }
      case "MakeDaftar": {
        const n = instr[1];
        const items = S.splice(S.length - n, n);
        S.push(daftar(items));
        return { selesai: false, pc: pc + 1 };
      }
      case "MakePeta": {
        const kunci = instr[1];
        const nilai = S.splice(S.length - kunci.length, kunci.length);
        S.push(peta(kunci.map((k, i) => [k, nilai[i]])));
        return { selesai: false, pc: pc + 1 };
      }
      case "Indeks": {
        const i = S.pop(), t = S.pop();
        S.push(indeksValue(t, i));
        return { selesai: false, pc: pc + 1 };
      }
      case "AmbilField": {
        const t = S.pop();
        if (t.t !== "Instans") throw new IsoteriError(`Akses field ".${instr[1]}" hanya berlaku untuk instans 'bentuk', ditemukan ${tampilkanStr(t)}`);
        const entri = t.v.find(([k]) => k === instr[1]);
        if (!entri) throw new IsoteriError(`Bentuk "${t.nama}" tidak punya field "${instr[1]}".`);
        S.push(entri[1]);
        return { selesai: false, pc: pc + 1 };
      }
      case "BuatInstans": {
        const [, nama, fieldNama] = instr;
        const nilai = S.splice(S.length - fieldNama.length, fieldNama.length);
        S.push({ t: "Instans", nama, v: fieldNama.map((k, i) => [k, nilai[i]]) });
        return { selesai: false, pc: pc + 1 };
      }
      case "SetField": {
        const baru = S.pop(), t = S.pop();
        if (t.t !== "Instans") throw new IsoteriError(`Tidak bisa mengubah field ".${instr[1]}" pada nilai ${tampilkanStr(t)} (bukan instans 'bentuk').`);
        if (!t.v.some(([k]) => k === instr[1])) throw new IsoteriError(`Bentuk "${t.nama}" tidak punya field "${instr[1]}".`);
        const baruEntries = t.v.map(([k, v]) => (k === instr[1] ? [k, baru] : [k, v]));
        S.push({ t: "Instans", nama: t.nama, v: baruEntries });
        return { selesai: false, pc: pc + 1 };
      }
      case "Dup": S.push(S[S.length - 1]); return { selesai: false, pc: pc + 1 };
      case "BuatFungsi": {
        const [, idx, jumlahTangkapan] = instr;
        const tangkapan = S.splice(S.length - jumlahTangkapan, jumlahTangkapan);
        S.push({ t: "Fungsi", idx, tangkapan });
        return { selesai: false, pc: pc + 1 };
      }
      case "PanggilNilai": {
        const argc = instr[1];
        const argumenPanggilan = S.splice(S.length - argc, argc);
        const callee = S.pop();
        if (callee.t !== "Fungsi") throw new IsoteriError(`Nilai ini bukan fungsi, gak bisa dipanggil: ${tampilkanStr(callee)}`);
        const argumenLengkap = callee.tangkapan.concat(argumenPanggilan);
        S.push(this.panggilFungsiDenganArgumen(callee.idx, argumenLengkap));
        return { selesai: false, pc: pc + 1 };
      }
      case "Tampilkan": this.tampilkanFn(tampilkanStr(S.pop())); return { selesai: false, pc: pc + 1 };
      case "Pop": S.pop(); return { selesai: false, pc: pc + 1 };
      case "PanggilFungsi": {
        const [, idx, argc] = instr;
        const f = this.fungsi[idx];
        if (f.paramCount !== argc) throw new IsoteriError(`Fungsi mengharapkan ${f.paramCount} argumen, tapi diberikan ${argc}.`);
        const base = this.locals.length;
        this.locals.length = base + f.localSlotCount;
        this.locals.fill(KOSONG, base, base + f.localSlotCount);
        const argsStart = S.length - argc;
        for (let i = 0; i < argc; i++) this.locals[base + i] = S[argsStart + i];
        S.length = argsStart;
        const hasil = this.eksekusi(f.kode, base) ?? KOSONG;
        this.locals.length = base;
        S.push(hasil);
        return { selesai: false, pc: pc + 1 };
      }
      case "PanggilBawaan": {
        const [, nama, argc] = instr;
        const argsStart = S.length - argc;
        const args = S.slice(argsStart);
        const hasil = this.panggilBawaan(nama, args);
        S.length = argsStart;
        S.push(hasil);
        return { selesai: false, pc: pc + 1 };
      }
      case "IterMulai": {
        const v = S.pop();
        if (v.t !== "Daftar") throw new IsoteriError(`'ulang setiap' butuh Daftar, ditemukan ${tampilkanStr(v)}`);
        this.iterStack.push({ items: v.v, pos: 0 });
        return { selesai: false, pc: pc + 1 };
      }
      case "IterLanjutLocal": {
        const [, slot, target] = instr;
        const frame = this.iterStack[this.iterStack.length - 1];
        if (frame.pos < frame.items.length) {
          this.locals[localsBase + slot] = frame.items[frame.pos];
          frame.pos += 1;
          return { selesai: false, pc: pc + 1 };
        }
        this.iterStack.pop();
        return { selesai: false, pc: target };
      }
      case "IterLanjutGlobal": {
        const [, slot, target] = instr;
        const frame = this.iterStack[this.iterStack.length - 1];
        if (frame.pos < frame.items.length) {
          this.globals[slot] = frame.items[frame.pos];
          frame.pos += 1;
          return { selesai: false, pc: pc + 1 };
        }
        this.iterStack.pop();
        return { selesai: false, pc: target };
      }
      case "MulaiCobaLocal": {
        const [, target, slot] = instr;
        this.handlerStack.push({ stackBase: S.length, target, slotKind: "Local", slot });
        return { selesai: false, pc: pc + 1 };
      }
      case "MulaiCobaGlobal": {
        const [, target, slot] = instr;
        this.handlerStack.push({ stackBase: S.length, target, slotKind: "Global", slot });
        return { selesai: false, pc: pc + 1 };
      }
      case "SelesaiCoba": this.handlerStack.pop(); return { selesai: false, pc: pc + 1 };
      case "Kembalikan": return { selesai: true, nilai: S.pop() };
      default:
        throw new IsoteriError(`Instruksi tidak didukung di web runtime: ${op}`);
    }
  }

  /** Setara panggil_fungsi_dengan_argumen() -- dipakai closure (PanggilNilai) & callback bawaan. */
  panggilFungsiDenganArgumen(idx, argumen) {
    const f = this.fungsi[idx];
    if (argumen.length !== f.paramCount) throw new IsoteriError(`Fungsi ini butuh ${f.paramCount} argumen, tapi diberi ${argumen.length}.`);
    const base = this.locals.length;
    this.locals.length = base + f.localSlotCount;
    for (let i = 0; i < argumen.length; i++) this.locals[base + i] = argumen[i];
    const hasil = this.eksekusi(f.kode, base) ?? KOSONG;
    this.locals.length = base;
    return hasil;
  }

  /** Setara panggil_fungsi_1_arg() -- membongkar instans 'bentuk' kalau parameter callback flattened. */
  panggilFungsi1Arg(idx, arg) {
    const f = this.fungsi[idx];
    if (f.paramFlat.length !== 1) throw new IsoteriError(`Fungsi callback butuh tepat 1 parameter (item-nya sendiri), tapi fungsi ini punya ${f.paramFlat.length} parameter.`);
    const fieldUrut = f.paramFlat[0];
    let argumen;
    if (fieldUrut) {
      if (arg.t !== "Instans") throw new IsoteriError(`Fungsi callback ini butuh instans 'bentuk', ditemukan ${tampilkanStr(arg)}`);
      argumen = fieldUrut.map((fnama) => {
        const entri = arg.v.find(([k]) => k === fnama);
        if (!entri) throw new IsoteriError(`Instans tidak punya field "${fnama}" yang dibutuhkan fungsi callback.`);
        return entri[1];
      });
    } else {
      argumen = [arg];
    }
    return this.panggilFungsiDenganArgumen(idx, argumen);
  }

  panggilBawaan(nama, args) {
    if ((nama === "petakan" || nama === "saring") && args.length === 2) {
      if (args[1].t !== "Teks") throw new IsoteriError(`${nama}(daftar, nama_fungsi): argumen kedua harus Teks berisi nama fungsi, ditemukan ${tampilkanStr(args[1])}`);
      if (args[0].t !== "Daftar") throw new IsoteriError(`${nama}(daftar, nama_fungsi): argumen pertama harus Daftar, ditemukan ${tampilkanStr(args[0])}`);
      const idx = this.namaKeIndeks[args[1].v];
      if (idx === undefined) throw new IsoteriError(`${nama}(): fungsi "${args[1].v}" tidak ditemukan.`);
      if (nama === "petakan") {
        return daftar(args[0].v.map((item) => this.panggilFungsi1Arg(idx, item)));
      }
      return daftar(args[0].v.filter((item) => {
        const r = this.panggilFungsi1Arg(idx, item);
        if (r.t !== "Bool") throw new IsoteriError(`saring(): fungsi penyaring harus mengembalikan Bool, ditemukan ${tampilkanStr(r)}`);
        return r.v;
      }));
    }
    if (nama === "urutkan" && (args.length === 1 || args.length === 2)) {
      if (args[0].t !== "Daftar") throw new IsoteriError(`urutkan(): argumen pertama harus Daftar, ditemukan ${tampilkanStr(args[0])}`);
      if (args.length === 2) {
        if (args[1].t !== "Teks") throw new IsoteriError(`urutkan(daftar, nama_fungsi): argumen kedua harus Teks berisi nama fungsi, ditemukan ${tampilkanStr(args[1])}`);
        const idx = this.namaKeIndeks[args[1].v];
        if (idx === undefined) throw new IsoteriError(`urutkan(): fungsi "${args[1].v}" tidak ditemukan.`);
        const berkunci = args[0].v.map((item) => [this.panggilFungsi1Arg(idx, item), item]);
        berkunci.sort((a, b) => bandingkanNilai(a[0], b[0]));
        return daftar(berkunci.map(([, v]) => v));
      }
      const d = args[0].v.slice();
      d.sort(bandingkanNilai);
      return daftar(d);
    }
    if (DOM_FUNGSI.has(nama)) return this.panggilDom(nama, args);
    return panggilBawaanMurni(nama, args);
  }

  // ---------------------------------------------------------------------
  // Milestone B -- DOM/Event/Fetch (docs/FILOSOFI.md). SENGAJA berupa fungsi
  // bawaan bernama datar (dom_pilih, dom_atur_teks, dst), BUKAN sintaks
  // "objek.metode()" bergaya method-call -- parser Isoteri saat ini cuma
  // dukung `nama_fungsi(args)` langsung, belum panggilan setelah akses field
  // (`Expr::Field` diikuti `(`). Menggeneralisasi grammar buat itu sekarang
  // berisiko & belum perlu -- Hukum 1 "Simplicity over cleverness": fungsi
  // bawaan datar SEPENUHNYA cukup buat visi "Isoteri terasa seperti bahasa
  // web sungguhan" tanpa menyentuh compiler inti sama sekali (persis Hukum
  // 3 "layer platform terpisah dari core": semua ini hidup di runtime/web/,
  // nol baris Rust berubah).
  //
  // Representasi elemen DOM sebagai Value: `Instans("ElemenDOM", [["_id", Teks(id)]])`
  // -- id adalah kunci ke `this.domRegistry` (Map id -> Element asli), BUKAN
  // Element itu sendiri (Value harus tetap serializable/JSON-safe secara umum,
  // dan supaya perbandingan `sama_dengan` dua pegangan ke elemen yang sama
  // tetap masuk akal by-value).
  panggilDom(nama, args) {
    const butuhDocument = !nama.startsWith("ws_") && nama !== "unduh_async";
    if (butuhDocument && typeof document === "undefined") {
      throw new IsoteriError(`${nama}() butuh browser (ada \`document\`) -- tidak berlaku di Node.js/runtime non-browser.`);
    }
    if (nama.startsWith("ws_") && typeof WebSocket === "undefined") {
      throw new IsoteriError(`${nama}() butuh WebSocket (browser atau Node dengan polyfill) -- tidak tersedia di runtime ini.`);
    }
    const elemenDari = (v) => {
      if (v.t !== "Instans" || v.nama !== "ElemenDOM") throw new IsoteriError(`${nama}(): argumen harus elemen DOM (hasil dom_pilih/dom_pilih_semua), ditemukan ${tampilkanStr(v)}`);
      const id = v.v.find(([k]) => k === "_id")[1].v;
      const el = this.domRegistry.get(id);
      if (!el) throw new IsoteriError(`${nama}(): elemen DOM sudah tidak valid (id "${id}" tidak ditemukan).`);
      return el;
    };
    const bungkusElemen = (el) => {
      if (!this._domIdCounter) this._domIdCounter = 0;
      const id = `el${this._domIdCounter++}`;
      this.domRegistry.set(id, el);
      return { t: "Instans", nama: "ElemenDOM", v: [["_id", teks(id)]] };
    };
    const konteksDari = (v) => {
      if (v.t !== "Instans" || v.nama !== "Konteks2D") throw new IsoteriError(`${nama}(): argumen harus konteks kanvas (hasil dom_konteks_2d), ditemukan ${tampilkanStr(v)}`);
      const id = v.v.find(([k]) => k === "_id")[1].v;
      const ctx = this.domRegistry.get(id);
      if (!ctx) throw new IsoteriError(`${nama}(): konteks kanvas sudah tidak valid (id "${id}" tidak ditemukan).`);
      return ctx;
    };
    const bungkusKonteks = (ctx) => {
      if (!this._domIdCounter) this._domIdCounter = 0;
      const id = `el${this._domIdCounter++}`;
      this.domRegistry.set(id, ctx);
      return { t: "Instans", nama: "Konteks2D", v: [["_id", teks(id)]] };
    };
    const soketDari = (v) => {
      if (v.t !== "Instans" || v.nama !== "WebSocket") throw new IsoteriError(`${nama}(): argumen harus koneksi WebSocket (hasil ws_buka), ditemukan ${tampilkanStr(v)}`);
      const id = v.v.find(([k]) => k === "_id")[1].v;
      const ws = this.domRegistry.get(id);
      if (!ws) throw new IsoteriError(`${nama}(): koneksi WebSocket sudah tidak valid (id "${id}" tidak ditemukan).`);
      return ws;
    };
    const bungkusSoket = (ws) => {
      if (!this._domIdCounter) this._domIdCounter = 0;
      const id = `el${this._domIdCounter++}`;
      this.domRegistry.set(id, ws);
      return { t: "Instans", nama: "WebSocket", v: [["_id", teks(id)]] };
    };
    const angkaArg = (v, label) => { if (v.t !== "Angka" && v.t !== "Desimal") throw new IsoteriError(`${nama}(): ${label} harus Angka/Desimal, ditemukan ${tampilkanStr(v)}`); return v.v; };
    const cariFungsi = (v, label) => {
      if (v.t !== "Teks") throw new IsoteriError(`${nama}(): ${label} harus Teks berisi nama fungsi (konvensi sama seperti petakan/saring/urutkan).`);
      const idx = this.namaKeIndeks[v.v];
      if (idx === undefined) throw new IsoteriError(`${nama}(): fungsi "${v.v}" tidak ditemukan.`);
      return idx;
    };
    const teksArg = (v, label) => { if (v.t !== "Teks") throw new IsoteriError(`${nama}(): ${label} harus Teks, ditemukan ${tampilkanStr(v)}`); return v.v; };

    switch (nama) {
      case "dom_pilih": {
        const el = document.querySelector(teksArg(args[0], "selector"));
        return el ? bungkusElemen(el) : KOSONG;
      }
      case "dom_pilih_semua": {
        const semua = Array.from(document.querySelectorAll(teksArg(args[0], "selector")));
        return daftar(semua.map(bungkusElemen));
      }
      case "dom_teks": return teks(elemenDari(args[0]).textContent ?? "");
      case "dom_atur_teks": elemenDari(args[0]).textContent = teksArg(args[1], "teks"); return args[0];
      case "dom_html": return teks(elemenDari(args[0]).innerHTML ?? "");
      case "dom_atur_html": elemenDari(args[0]).innerHTML = teksArg(args[1], "html"); return args[0];
      case "dom_atribut": { const v = elemenDari(args[0]).getAttribute(teksArg(args[1], "nama atribut")); return v === null ? KOSONG : teks(v); }
      case "dom_atur_atribut": elemenDari(args[0]).setAttribute(teksArg(args[1], "nama atribut"), teksArg(args[2], "nilai atribut")); return args[0];
      case "dom_tambah_kelas": elemenDari(args[0]).classList.add(teksArg(args[1], "nama kelas")); return args[0];
      case "dom_hapus_kelas": elemenDari(args[0]).classList.remove(teksArg(args[1], "nama kelas")); return args[0];
      case "dom_punya_kelas": return bool(elemenDari(args[0]).classList.contains(teksArg(args[1], "nama kelas")));
      case "dom_buat": return bungkusElemen(document.createElement(teksArg(args[0], "nama tag")));
      case "dom_tambah_anak": elemenDari(args[0]).appendChild(elemenDari(args[1])); return args[0];
      case "dom_hapus": elemenDari(args[0]).remove(); return KOSONG;
      case "dom_ketika": {
        if (args[2].t !== "Teks") throw new IsoteriError('dom_ketika(elemen, nama_event, "nama_fungsi_penangan"): argumen ketiga harus Teks berisi nama fungsi (konvensi sama seperti petakan/saring/urutkan).');
        const el = elemenDari(args[0]);
        const namaEvent = teksArg(args[1], "nama event");
        const idx = this.namaKeIndeks[args[2].v];
        if (idx === undefined) throw new IsoteriError(`dom_ketika(): fungsi "${args[2].v}" tidak ditemukan.`);
        el.addEventListener(namaEvent, () => {
          try {
            this.panggilFungsiDenganArgumen(idx, []);
          } catch (e) {
            console.error(`Kesalahan di dalam pawang event "${namaEvent}":`, e.message || e);
          }
        });
        return KOSONG;
      }
      case "simpan_lokal": window.localStorage.setItem(teksArg(args[0], "kunci"), tampilkanStr(args[1])); return KOSONG;
      case "ambil_lokal": { const v = window.localStorage.getItem(teksArg(args[0], "kunci")); return v === null ? KOSONG : teks(v); }
      case "hapus_lokal": window.localStorage.removeItem(teksArg(args[0], "kunci")); return KOSONG;
      case "unduh_async": {
        const url = teksArg(args[0], "url");
        if (args[1].t !== "Teks") throw new IsoteriError('unduh_async(url, "nama_fungsi_sukses", "nama_fungsi_gagal"?): argumen kedua harus Teks berisi nama fungsi.');
        const idxSukses = this.namaKeIndeks[args[1].v];
        if (idxSukses === undefined) throw new IsoteriError(`unduh_async(): fungsi "${args[1].v}" tidak ditemukan.`);
        const idxGagal = args.length > 2 && args[2].t === "Teks" ? this.namaKeIndeks[args[2].v] : undefined;
        fetch(url).then((r) => r.text()).then((isi) => {
          this.panggilFungsiDenganArgumen(idxSukses, [teks(isi)]);
        }).catch((e) => {
          if (idxGagal !== undefined) this.panggilFungsiDenganArgumen(idxGagal, [teks(String(e.message || e))]);
          else console.error("unduh_async() gagal (tanpa fungsi_gagal):", e.message || e);
        });
        return KOSONG;
      }

      // --- Canvas 2D (lanjutan Milestone B) ---
      case "dom_konteks_2d": {
        const el = elemenDari(args[0]);
        const ctx = el.getContext && el.getContext("2d");
        if (!ctx) throw new IsoteriError("dom_konteks_2d(): elemen bukan <canvas> atau getContext('2d') tidak tersedia.");
        return bungkusKonteks(ctx);
      }
      case "kanvas_isi_gaya": konteksDari(args[0]).fillStyle = teksArg(args[1], "warna"); return args[0];
      case "kanvas_garis_gaya": konteksDari(args[0]).strokeStyle = teksArg(args[1], "warna"); return args[0];
      case "kanvas_lebar_garis": konteksDari(args[0]).lineWidth = angkaArg(args[1], "lebar"); return args[0];
      case "kanvas_font": konteksDari(args[0]).font = teksArg(args[1], "font"); return args[0];
      case "kanvas_isi_persegi": konteksDari(args[0]).fillRect(angkaArg(args[1], "x"), angkaArg(args[2], "y"), angkaArg(args[3], "lebar"), angkaArg(args[4], "tinggi")); return args[0];
      case "kanvas_garis_persegi": konteksDari(args[0]).strokeRect(angkaArg(args[1], "x"), angkaArg(args[2], "y"), angkaArg(args[3], "lebar"), angkaArg(args[4], "tinggi")); return args[0];
      case "kanvas_bersihkan": konteksDari(args[0]).clearRect(angkaArg(args[1], "x"), angkaArg(args[2], "y"), angkaArg(args[3], "lebar"), angkaArg(args[4], "tinggi")); return args[0];
      case "kanvas_isi_teks": konteksDari(args[0]).fillText(teksArg(args[1], "teks"), angkaArg(args[2], "x"), angkaArg(args[3], "y")); return args[0];
      case "kanvas_mulai_jalur": konteksDari(args[0]).beginPath(); return args[0];
      case "kanvas_pindah_ke": konteksDari(args[0]).moveTo(angkaArg(args[1], "x"), angkaArg(args[2], "y")); return args[0];
      case "kanvas_garis_ke": konteksDari(args[0]).lineTo(angkaArg(args[1], "x"), angkaArg(args[2], "y")); return args[0];
      case "kanvas_lingkaran": {
        const [, cx, cy, r, sudutMulai, sudutAkhir] = args;
        konteksDari(args[0]).arc(angkaArg(cx, "x"), angkaArg(cy, "y"), angkaArg(r, "radius"), angkaArg(sudutMulai, "sudut_mulai"), angkaArg(sudutAkhir, "sudut_akhir"));
        return args[0];
      }
      case "kanvas_isi": konteksDari(args[0]).fill(); return args[0];
      case "kanvas_garis": konteksDari(args[0]).stroke(); return args[0];

      // --- WebSocket (lanjutan Milestone B) ---
      case "ws_buka": return bungkusSoket(new WebSocket(teksArg(args[0], "url")));
      case "ws_kirim": soketDari(args[0]).send(teksArg(args[1], "pesan")); return args[0];
      case "ws_tutup": soketDari(args[0]).close(); return KOSONG;
      case "ws_status": {
        const kode = soketDari(args[0]).readyState;
        return teks(["MENYAMBUNG", "TERBUKA", "MENUTUP", "TERTUTUP"][kode] ?? "TIDAK_DIKENAL");
      }
      case "ws_ketika_pesan": {
        const idx = cariFungsi(args[1], "nama fungsi penangan");
        soketDari(args[0]).addEventListener("message", (ev) => {
          try { this.panggilFungsiDenganArgumen(idx, [teks(String(ev.data))]); }
          catch (e) { console.error('Kesalahan di dalam pawang "ws_ketika_pesan":', e.message || e); }
        });
        return args[0];
      }
      case "ws_ketika_buka": {
        const idx = cariFungsi(args[1], "nama fungsi penangan");
        soketDari(args[0]).addEventListener("open", () => {
          try { this.panggilFungsiDenganArgumen(idx, []); }
          catch (e) { console.error('Kesalahan di dalam pawang "ws_ketika_buka":', e.message || e); }
        });
        return args[0];
      }
      case "ws_ketika_tutup": {
        const idx = cariFungsi(args[1], "nama fungsi penangan");
        soketDari(args[0]).addEventListener("close", () => {
          try { this.panggilFungsiDenganArgumen(idx, []); }
          catch (e) { console.error('Kesalahan di dalam pawang "ws_ketika_tutup":', e.message || e); }
        });
        return args[0];
      }
      case "ws_ketika_error": {
        const idx = cariFungsi(args[1], "nama fungsi penangan");
        soketDari(args[0]).addEventListener("error", () => {
          try { this.panggilFungsiDenganArgumen(idx, []); }
          catch (e) { console.error('Kesalahan di dalam pawang "ws_ketika_error":', e.message || e); }
        });
        return args[0];
      }
      default: throw new IsoteriError(`Fungsi DOM "${nama}" tidak dikenal.`);
    }
  }
}

// ---------------------------------------------------------------------
// Representasi Value: objek tag { t, v, ... } -- lihat value_ke_json() di
// src/lib.rs untuk skema yang sama persis dipakai di sisi Rust.
// ---------------------------------------------------------------------

const KOSONG = { t: "Kosong" };
/** Nama fungsi bawaan Milestone B (DOM/Event/Fetch/Storage) -- lihat IsoteriVM.panggilDom. */
const DOM_FUNGSI = new Set([
  "dom_pilih", "dom_pilih_semua", "dom_teks", "dom_atur_teks", "dom_html", "dom_atur_html",
  "dom_atribut", "dom_atur_atribut", "dom_tambah_kelas", "dom_hapus_kelas", "dom_punya_kelas",
  "dom_buat", "dom_tambah_anak", "dom_hapus", "dom_ketika",
  "simpan_lokal", "ambil_lokal", "hapus_lokal", "unduh_async",
  "dom_konteks_2d", "kanvas_isi_gaya", "kanvas_garis_gaya", "kanvas_lebar_garis", "kanvas_font",
  "kanvas_isi_persegi", "kanvas_garis_persegi", "kanvas_bersihkan", "kanvas_isi_teks",
  "kanvas_mulai_jalur", "kanvas_pindah_ke", "kanvas_garis_ke", "kanvas_lingkaran", "kanvas_isi", "kanvas_garis",
  "ws_buka", "ws_kirim", "ws_tutup", "ws_status", "ws_ketika_pesan", "ws_ketika_buka", "ws_ketika_tutup", "ws_ketika_error",
]);
const teks = (s) => ({ t: "Teks", v: s });
const angka = (n) => ({ t: "Angka", v: n });
const desimal = (n) => ({ t: "Desimal", v: n });
const bool = (b) => ({ t: "Bool", v: b });
const daftar = (items) => ({ t: "Daftar", v: items });
const peta = (entries) => ({ t: "Peta", v: entries });

function decodeValue(v) {
  switch (v.t) {
    case "Angka": return angka(v.v);
    case "Desimal": return desimal(v.v);
    case "Teks": return teks(v.v);
    case "Bool": return bool(v.v);
    case "Kosong": return KOSONG;
    case "Daftar": return daftar(v.v.map(decodeValue));
    case "Peta": return peta(v.v.map(([k, vv]) => [k, decodeValue(vv)]));
    case "Instans": return { t: "Instans", nama: v.nama, v: v.v.map(([k, vv]) => [k, decodeValue(vv)]) };
    case "Fungsi": return { t: "Fungsi", idx: v.idx, tangkapan: v.tangkapan.map(decodeValue) };
    default: throw new IsoteriError(`Tipe nilai tidak dikenal di bytecode: ${v.t}`);
  }
}

function truthy(v) {
  switch (v.t) {
    case "Bool": return v.v;
    case "Angka": return v.v !== 0;
    case "Desimal": return v.v !== 0;
    case "Teks": return v.v.length > 0;
    case "Daftar": return v.v.length > 0;
    case "Peta": return v.v.length > 0;
    case "Kosong": return false;
    default: return true; // Instans, Fungsi
  }
}

function keDesimal(v) {
  if (v.t === "Angka" || v.t === "Desimal") return v.v;
  return null;
}

function nilaiSama(l, r) {
  if (l.t === "Angka" && r.t === "Angka") return l.v === r.v;
  if (l.t === "Desimal" && r.t === "Desimal") return l.v === r.v;
  if ((l.t === "Angka" && r.t === "Desimal") || (l.t === "Desimal" && r.t === "Angka")) return l.v === r.v;
  if (l.t === "Teks" && r.t === "Teks") return l.v === r.v;
  if (l.t === "Bool" && r.t === "Bool") return l.v === r.v;
  if (l.t === "Kosong" && r.t === "Kosong") return true;
  if (l.t === "Daftar" && r.t === "Daftar") return l.v.length === r.v.length && l.v.every((x, i) => nilaiSama(x, r.v[i]));
  if (l.t === "Peta" && r.t === "Peta") return l.v.length === r.v.length && l.v.every(([k, v]) => r.v.some(([k2, v2]) => k === k2 && nilaiSama(v, v2)));
  if (l.t === "Instans" && r.t === "Instans") return l.nama === r.nama && l.v.length === r.v.length && l.v.every(([k, v], i) => r.v[i][0] === k && nilaiSama(v, r.v[i][1]));
  return false;
}

function bandingkanNilai(a, b) {
  if (a.t === "Teks" && b.t === "Teks") return a.v < b.v ? -1 : a.v > b.v ? 1 : 0;
  const x = keDesimal(a), y = keDesimal(b);
  if (x !== null && y !== null) return x - y;
  throw new IsoteriError(`urutkan() cuma bisa buat daftar berisi Angka/Desimal atau Teks (gak campur), ditemukan ${tampilkanStr(a)} dan ${tampilkanStr(b)}`);
}

function bandingkan(l, r, f) {
  const a = keDesimal(l), b = keDesimal(r);
  if (a === null || b === null) throw new IsoteriError(`Perbandingan hanya berlaku untuk Angka, ditemukan ${tampilkanStr(l)} dan ${tampilkanStr(r)}`);
  return bool(f(a, b));
}

function evalBinOp(l, op, r) {
  switch (op) {
    case "Tambah":
      if (l.t === "Teks" || r.t === "Teks") return teks(tampilkanStr(l) + tampilkanStr(r));
      if (l.t === "Angka" && r.t === "Angka") return angka(l.v + r.v);
      if ((l.t === "Angka" || l.t === "Desimal") && (r.t === "Angka" || r.t === "Desimal")) return desimal(keDesimal(l) + keDesimal(r));
      throw new IsoteriError(`Tidak bisa menjumlahkan ${tampilkanStr(l)} dengan ${tampilkanStr(r)}`);
    case "Kurang":
      if (l.t === "Angka" && r.t === "Angka") return angka(l.v - r.v);
      if (keDesimal(l) !== null && keDesimal(r) !== null) return desimal(keDesimal(l) - keDesimal(r));
      throw new IsoteriError(`Operator '-' hanya berlaku untuk Angka, ditemukan ${tampilkanStr(l)} dan ${tampilkanStr(r)}`);
    case "Kali":
      if (l.t === "Angka" && r.t === "Angka") return angka(l.v * r.v);
      if (keDesimal(l) !== null && keDesimal(r) !== null) return desimal(keDesimal(l) * keDesimal(r));
      throw new IsoteriError(`Operator '*' hanya berlaku untuk Angka, ditemukan ${tampilkanStr(l)} dan ${tampilkanStr(r)}`);
    case "Bagi":
      if (l.t === "Angka" && r.t === "Angka") {
        if (r.v === 0) throw new IsoteriError("Tidak bisa membagi dengan nol.");
        return angka(Math.trunc(l.v / r.v));
      }
      if (keDesimal(l) !== null && keDesimal(r) !== null) {
        if (keDesimal(r) === 0) throw new IsoteriError("Tidak bisa membagi dengan nol.");
        return desimal(keDesimal(l) / keDesimal(r));
      }
      throw new IsoteriError(`Operator '/' hanya berlaku untuk Angka, ditemukan ${tampilkanStr(l)} dan ${tampilkanStr(r)}`);
    case "SamaDengan": return bool(nilaiSama(l, r));
    case "TidakSama": return bool(!nilaiSama(l, r));
    case "LebihBesar": return bandingkan(l, r, (a, b) => a > b);
    case "LebihBesarSama": return bandingkan(l, r, (a, b) => a >= b);
    case "LebihKecil": return bandingkan(l, r, (a, b) => a < b);
    case "LebihKecilSama": return bandingkan(l, r, (a, b) => a <= b);
    case "Dan": return bool(truthy(l) && truthy(r));
    case "Atau": return bool(truthy(l) || truthy(r));
    default: throw new IsoteriError(`Operator tidak dikenal: ${op}`);
  }
}

function indeksValue(t, i) {
  if (t.t === "Daftar" && i.t === "Angka") {
    if (i.v < 0) throw new IsoteriError(`Indeks tidak boleh negatif: ${i.v}`);
    if (i.v >= t.v.length) throw new IsoteriError(`Indeks ${i.v} di luar jangkauan (panjang daftar: ${t.v.length})`);
    return t.v[i.v];
  }
  if (t.t === "Peta" && i.t === "Teks") {
    const entri = t.v.find(([k]) => k === i.v);
    if (!entri) throw new IsoteriError(`Kunci "${i.v}" tidak ditemukan di Peta.`);
    return entri[1];
  }
  throw new IsoteriError(`Tidak bisa mengindeks ${tampilkanStr(t)} dengan ${tampilkanStr(i)}`);
}

/** Setara impl Display for Value di Rust -- HARUS identik supaya output tampilkan() sama persis. */
function tampilkanStr(v) {
  switch (v.t) {
    case "Angka": return String(v.v);
    case "Desimal": return Number.isFinite(v.v) && Number.isInteger(v.v) ? v.v.toFixed(1) : String(v.v);
    case "Teks": return v.v;
    case "Bool": return v.v ? "benar" : "salah";
    case "Daftar": return `[${v.v.map(tampilkanStr).join(", ")}]`;
    case "Peta": return `{${v.v.map(([k, vv]) => `"${k}": ${tampilkanStr(vv)}`).join(", ")}}`;
    case "Kosong": return "kosong";
    case "Instans": return `${v.nama} {${v.v.map(([k, vv]) => `${k}: ${tampilkanStr(vv)}`).join(", ")}}`;
    case "Fungsi": return "<fungsi>";
    default: return String(v);
  }
}

// ---------------------------------------------------------------------
// Standard library dasar (panggil_bawaan di Rust) -- subset yang masuk
// akal untuk browser: daftar/teks/matematika/JSON. TIDAK termasuk
// baca_berkas/tulis_berkas/unduh (I/O sistem & jaringan sinkron).
// ---------------------------------------------------------------------

function panggilBawaanMurni(nama, args) {
  const butuh = (n, sig) => { if (args.length < n) throw new IsoteriError(`${sig} butuh ${n} argumen`); };
  switch (nama) {
    case "panjang": {
      const a = args[0];
      if (a.t === "Daftar") return angka(a.v.length);
      if (a.t === "Teks") return angka([...a.v].length);
      if (a.t === "Peta") return angka(a.v.length);
      throw new IsoteriError(`panjang() tidak berlaku untuk ${tampilkanStr(a)}`);
    }
    case "gabung": {
      butuh(2, "gabung(daftar, item)");
      if (args[0].t !== "Daftar") throw new IsoteriError(`gabung() argumen pertama harus Daftar, ditemukan ${tampilkanStr(args[0])}`);
      return daftar([...args[0].v, args[1]]);
    }
    case "ambil": {
      butuh(2, "ambil(struktur, kunci)");
      const [s, k] = args;
      if (s.t === "Daftar" && k.t === "Angka") {
        if (k.v < 0) throw new IsoteriError(`Indeks tidak boleh negatif: ${k.v}`);
        if (k.v >= s.v.length) throw new IsoteriError(`Indeks ${k.v} di luar jangkauan (panjang daftar: ${s.v.length})`);
        return s.v[k.v];
      }
      if (s.t === "Peta" && k.t === "Teks") {
        const entri = s.v.find(([kk]) => kk === k.v);
        if (!entri) throw new IsoteriError(`Kunci "${k.v}" tidak ditemukan di Peta.`);
        return entri[1];
      }
      throw new IsoteriError(`ambil() butuh (Daftar, Angka) atau (Peta, Teks), ditemukan ${tampilkanStr(s)} dan ${tampilkanStr(k)}`);
    }
    case "jumlah": {
      if (args[0].t !== "Daftar") throw new IsoteriError(`jumlah() butuh Daftar, ditemukan ${tampilkanStr(args[0])}`);
      let totalF = 0, adaDesimal = false;
      for (const v of args[0].v) {
        if (v.t === "Angka") totalF += v.v;
        else if (v.t === "Desimal") { adaDesimal = true; totalF += v.v; }
        else throw new IsoteriError(`jumlah() hanya untuk daftar berisi Angka, ditemukan ${tampilkanStr(v)}`);
      }
      return adaDesimal ? desimal(totalF) : angka(totalF);
    }
    case "rata_rata": {
      if (args[0].t !== "Daftar") throw new IsoteriError(`rata_rata() butuh Daftar, ditemukan ${tampilkanStr(args[0])}`);
      if (args[0].v.length === 0) throw new IsoteriError("rata_rata() tidak bisa dihitung dari daftar kosong");
      let totalF = 0, adaDesimal = false;
      for (const v of args[0].v) {
        if (v.t === "Angka") totalF += v.v;
        else if (v.t === "Desimal") { adaDesimal = true; totalF += v.v; }
        else throw new IsoteriError(`rata_rata() hanya untuk daftar berisi Angka, ditemukan ${tampilkanStr(v)}`);
      }
      const n = args[0].v.length;
      return adaDesimal ? desimal(totalF / n) : angka(Math.trunc(totalF / n));
    }
    case "kunci_peta": {
      if (args[0].t !== "Peta") throw new IsoteriError(`kunci_peta() butuh Peta, ditemukan ${tampilkanStr(args[0])}`);
      return daftar(args[0].v.map(([k]) => teks(k)));
    }
    case "urai_json": {
      if (args[0].t !== "Teks") throw new IsoteriError(`urai_json() butuh Teks, ditemukan ${tampilkanStr(args[0])}`);
      return jsonUrai(args[0].v);
    }
    case "teks_json": return teks(valueKeJsonStr(args[0]));
    case "gagal_uji": {
      if (args[0] && args[0].t === "Teks") throw new IsoteriError(args[0].v);
      throw new IsoteriError("Uji gagal (gagal_uji() dipanggil tanpa pesan).");
    }
    case "baca_berkas": case "tulis_berkas": case "unduh":
      throw new IsoteriError(`${nama}() belum didukung di web runtime (I/O sistem/jaringan) -- lihat runtime/web/README.md.`);
    case "ke_desimal": {
      if (args[0].t === "Angka") return desimal(args[0].v);
      if (args[0].t === "Desimal") return args[0];
      throw new IsoteriError(`ke_desimal() tidak berlaku untuk ${tampilkanStr(args[0])}`);
    }
    case "ke_bulat": {
      if (args[0].t === "Desimal") return angka(Math.trunc(args[0].v));
      if (args[0].t === "Angka") return args[0];
      throw new IsoteriError(`ke_bulat() tidak berlaku untuk ${tampilkanStr(args[0])}`);
    }
    case "ke_teks": return teks(tampilkanStr(args[0]));
    case "akar": {
      const x = keDesimal(args[0]);
      if (x === null) throw new IsoteriError(`akar() butuh Angka/Desimal, ditemukan ${tampilkanStr(args[0])}`);
      if (x < 0) throw new IsoteriError("akar() tidak berlaku untuk angka negatif.");
      return desimal(Math.sqrt(x));
    }
    case "pangkat": {
      butuh(2, "pangkat(basis, eksponen)");
      const [b, e] = args;
      if (b.t === "Angka" && e.t === "Angka" && e.v >= 0) return angka(Math.round(b.v ** e.v));
      const bf = keDesimal(b), ef = keDesimal(e);
      if (bf === null || ef === null) throw new IsoteriError(`pangkat() butuh Angka/Desimal, ditemukan ${tampilkanStr(b)} dan ${tampilkanStr(e)}`);
      return desimal(bf ** ef);
    }
    case "bulat": case "bulat_bawah": case "bulat_atas": {
      const x = keDesimal(args[0]);
      if (x === null) throw new IsoteriError(`${nama}() butuh Angka/Desimal, ditemukan ${tampilkanStr(args[0])}`);
      if (nama === "bulat") return angka(Math.round(x));
      if (nama === "bulat_bawah") return angka(Math.floor(x));
      return angka(Math.ceil(x));
    }
    case "mutlak": {
      if (args[0].t === "Angka") return angka(Math.abs(args[0].v));
      if (args[0].t === "Desimal") return desimal(Math.abs(args[0].v));
      throw new IsoteriError(`mutlak() tidak berlaku untuk ${tampilkanStr(args[0])}`);
    }
    case "min": case "maks": {
      butuh(2, `${nama}(a, b)`);
      const [a, b] = args;
      const x = keDesimal(a), y = keDesimal(b);
      if (x === null || y === null) throw new IsoteriError(`${nama}() butuh Angka/Desimal, ditemukan ${tampilkanStr(a)} dan ${tampilkanStr(b)}`);
      if (nama === "min") return x <= y ? a : b;
      return x >= y ? a : b;
    }
    case "acak": return desimal(Math.random());
    case "potong": {
      butuh(3, "potong(teks, mulai, akhir)");
      const [s, mulaiV, akhirV] = args;
      if (s.t !== "Teks" || mulaiV.t !== "Angka" || akhirV.t !== "Angka") throw new IsoteriError("potong(teks, mulai, akhir) tipe argumen salah");
      const chars = [...s.v];
      const mulai = Math.max(0, mulaiV.v);
      const akhir = Math.min(chars.length, Math.max(0, akhirV.v));
      if (mulai > akhir) throw new IsoteriError(`potong(): 'mulai' (${mulai}) tidak boleh lebih besar dari 'akhir' (${akhir})`);
      return teks(chars.slice(mulai, akhir).join(""));
    }
    case "ganti": {
      butuh(3, "ganti(teks, dari, ke)");
      const [s, dari, ke] = args;
      if (s.t !== "Teks" || dari.t !== "Teks" || ke.t !== "Teks") throw new IsoteriError("ganti(teks, dari, ke) butuh Teks semua");
      return teks(s.v.split(dari.v).join(ke.v));
    }
    case "huruf_besar": if (args[0].t !== "Teks") throw new IsoteriError(`huruf_besar() butuh Teks, ditemukan ${tampilkanStr(args[0])}`); return teks(args[0].v.toUpperCase());
    case "huruf_kecil": if (args[0].t !== "Teks") throw new IsoteriError(`huruf_kecil() butuh Teks, ditemukan ${tampilkanStr(args[0])}`); return teks(args[0].v.toLowerCase());
    case "pangkas": if (args[0].t !== "Teks") throw new IsoteriError(`pangkas() butuh Teks, ditemukan ${tampilkanStr(args[0])}`); return teks(args[0].v.trim());
    case "pisah": {
      butuh(2, "pisah(teks, pemisah)");
      const [s, pemisah] = args;
      if (s.t !== "Teks" || pemisah.t !== "Teks") throw new IsoteriError("pisah(teks, pemisah) butuh Teks semua");
      const bagian = pemisah.v === "" ? [...s.v] : s.v.split(pemisah.v);
      return daftar(bagian.map(teks));
    }
    case "satukan": {
      butuh(2, "satukan(daftar, pemisah)");
      const [d, pemisah] = args;
      if (d.t !== "Daftar" || pemisah.t !== "Teks") throw new IsoteriError("satukan(daftar, pemisah) argumen salah");
      return teks(d.v.map((v) => { if (v.t !== "Teks") throw new IsoteriError(`satukan() cuma bisa buat daftar berisi Teks, ditemukan ${tampilkanStr(v)}`); return v.v; }).join(pemisah.v));
    }
    case "mengandung": {
      butuh(2, "mengandung(teks, sub)");
      const [s, sub] = args;
      if (s.t === "Teks" && sub.t === "Teks") return bool(s.v.includes(sub.v));
      if (s.t === "Daftar") return bool(s.v.some((v) => nilaiSama(v, sub)));
      throw new IsoteriError(`mengandung() argumen pertama harus Teks/Daftar, ditemukan ${tampilkanStr(s)}`);
    }
    case "diawali": {
      butuh(2, "diawali(teks, sub)");
      if (args[0].t !== "Teks" || args[1].t !== "Teks") throw new IsoteriError("diawali(teks, sub) butuh Teks semua");
      return bool(args[0].v.startsWith(args[1].v));
    }
    case "diakhiri": {
      butuh(2, "diakhiri(teks, sub)");
      if (args[0].t !== "Teks" || args[1].t !== "Teks") throw new IsoteriError("diakhiri(teks, sub) butuh Teks semua");
      return bool(args[0].v.endsWith(args[1].v));
    }
    default:
      throw new IsoteriError(`Fungsi "${nama}" tidak ditemukan.`);
  }
}

// Parser JSON tulisan tangan (BUKAN JSON.parse bawaan) supaya perilakunya identik dengan
// json_urai() di src/lib.rs: Angka vs Desimal ditentukan dari literal APA ADANYA (ada '.'
// atau 'e'/'E' di teks sumbernya, bukan dari nilai hasil) -- JSON.parse mengaburkan bedanya
// karena "150.0" dan "150" sama-sama jadi number JS 150.
function jsonUrai(s) {
  const c = [...s];
  const pos = { i: 0 };
  jsonSkipWs(c, pos);
  return jsonNilai(c, pos);
}
function jsonSkipWs(c, pos) { while (pos.i < c.length && /\s/.test(c[pos.i])) pos.i++; }
function jsonNilai(c, pos) {
  jsonSkipWs(c, pos);
  if (pos.i >= c.length) throw new IsoteriError("JSON tidak lengkap.");
  const ch = c[pos.i];
  if (ch === "{") return jsonObjek(c, pos);
  if (ch === "[") return jsonLarik(c, pos);
  if (ch === '"') return teks(jsonString(c, pos));
  if (ch === "t") { jsonHarapKata(c, pos, "true"); return bool(true); }
  if (ch === "f") { jsonHarapKata(c, pos, "false"); return bool(false); }
  if (ch === "n") { jsonHarapKata(c, pos, "null"); return KOSONG; }
  return jsonAngka(c, pos);
}
function jsonHarapKata(c, pos, kata) {
  for (const ch of kata) {
    if (pos.i >= c.length || c[pos.i] !== ch) throw new IsoteriError(`JSON tidak valid, diharapkan "${kata}".`);
    pos.i++;
  }
}
function jsonString(c, pos) {
  pos.i++;
  let s = "";
  while (pos.i < c.length && c[pos.i] !== '"') {
    if (c[pos.i] === "\\" && pos.i + 1 < c.length) {
      const n = c[pos.i + 1];
      if (n === '"') { s += '"'; pos.i += 2; }
      else if (n === "\\") { s += "\\"; pos.i += 2; }
      else if (n === "n") { s += "\n"; pos.i += 2; }
      else if (n === "t") { s += "\t"; pos.i += 2; }
      else if (n === "r") { s += "\r"; pos.i += 2; }
      else if (n === "/") { s += "/"; pos.i += 2; }
      else if (n === "u") {
        if (pos.i + 5 < c.length) {
          const hex = c.slice(pos.i + 2, pos.i + 6).join("");
          s += String.fromCodePoint(parseInt(hex, 16));
          pos.i += 6;
        } else pos.i += 2;
      } else { s += n; pos.i += 2; }
    } else { s += c[pos.i]; pos.i++; }
  }
  if (pos.i >= c.length) throw new IsoteriError("Teks JSON tidak ditutup dengan tanda kutip.");
  pos.i++;
  return s;
}
function jsonAngka(c, pos) {
  const mulai = pos.i;
  if (pos.i < c.length && c[pos.i] === "-") pos.i++;
  while (pos.i < c.length && /[0-9]/.test(c[pos.i])) pos.i++;
  let desimalFlag = false;
  if (pos.i < c.length && c[pos.i] === ".") { desimalFlag = true; pos.i++; while (pos.i < c.length && /[0-9]/.test(c[pos.i])) pos.i++; }
  if (pos.i < c.length && (c[pos.i] === "e" || c[pos.i] === "E")) {
    desimalFlag = true; pos.i++;
    if (pos.i < c.length && (c[pos.i] === "+" || c[pos.i] === "-")) pos.i++;
    while (pos.i < c.length && /[0-9]/.test(c[pos.i])) pos.i++;
  }
  const teksAngka = c.slice(mulai, pos.i).join("");
  if (teksAngka === "" || teksAngka === "-") throw new IsoteriError("JSON tidak valid: angka kosong.");
  const n = Number(teksAngka);
  if (Number.isNaN(n)) throw new IsoteriError("JSON tidak valid: format angka salah.");
  return desimalFlag ? desimal(n) : angka(Math.trunc(n));
}
function jsonLarik(c, pos) {
  pos.i++;
  const items = [];
  jsonSkipWs(c, pos);
  if (pos.i < c.length && c[pos.i] === "]") { pos.i++; return daftar(items); }
  for (;;) {
    items.push(jsonNilai(c, pos));
    jsonSkipWs(c, pos);
    if (pos.i < c.length && c[pos.i] === ",") { pos.i++; jsonSkipWs(c, pos); continue; }
    break;
  }
  jsonSkipWs(c, pos);
  if (pos.i >= c.length || c[pos.i] !== "]") throw new IsoteriError("JSON larik tidak ditutup dengan ']'.");
  pos.i++;
  return daftar(items);
}
function jsonObjek(c, pos) {
  pos.i++;
  const entries = [];
  jsonSkipWs(c, pos);
  if (pos.i < c.length && c[pos.i] === "}") { pos.i++; return peta(entries); }
  for (;;) {
    jsonSkipWs(c, pos);
    if (pos.i >= c.length || c[pos.i] !== '"') throw new IsoteriError("JSON objek: kunci harus berupa teks berpetik dua.");
    const kunci = jsonString(c, pos);
    jsonSkipWs(c, pos);
    if (pos.i >= c.length || c[pos.i] !== ":") throw new IsoteriError("JSON objek: diharapkan ':' setelah kunci.");
    pos.i++;
    const nilai = jsonNilai(c, pos);
    entries.push([kunci, nilai]);
    jsonSkipWs(c, pos);
    if (pos.i < c.length && c[pos.i] === ",") { pos.i++; continue; }
    break;
  }
  jsonSkipWs(c, pos);
  if (pos.i >= c.length || c[pos.i] !== "}") throw new IsoteriError("JSON objek tidak ditutup dengan '}'.");
  pos.i++;
  return peta(entries);
}

function valueKeJsonStr(v) {
  switch (v.t) {
    case "Angka": return String(v.v);
    case "Desimal": return String(v.v);
    case "Teks": return JSON.stringify(v.v);
    case "Bool": return v.v ? "true" : "false";
    case "Daftar": return `[${v.v.map(valueKeJsonStr).join(",")}]`;
    case "Peta": case "Instans":
      return `{${v.v.map(([k, vv]) => `${JSON.stringify(k)}:${valueKeJsonStr(vv)}`).join(",")}}`;
    case "Kosong": return "null";
    case "Fungsi": return "null";
    default: return "null";
  }
}

if (typeof module !== "undefined" && module.exports) {
  module.exports = { IsoteriVM, IsoteriError };
}

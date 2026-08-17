#!/usr/bin/env node
// Jalankan bundel bytecode Isoteri (hasil `isoteri ekspor-web`) di Node.js.
// Pemakaian: node runtime/web/jalankan-node.js program.isoweb.json
const fs = require("fs");
const { IsoteriVM } = require("./isoteri-vm.js");

const path = process.argv[2];
if (!path) {
  console.error("pakai: node jalankan-node.js program.isoweb.json");
  process.exit(1);
}

const bundle = JSON.parse(fs.readFileSync(path, "utf8"));
const vm = new IsoteriVM(bundle);
try {
  vm.jalankan();
} catch (e) {
  console.error(e.message);
  process.exit(1);
}

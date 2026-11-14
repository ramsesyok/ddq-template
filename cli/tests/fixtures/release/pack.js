// tests/release.rs の実ビルドの経路で `npm run package` / `npm run package:win32-x64` の代わりに
// VSIX（無圧縮の zip）を作る。vsce は使わない（通信も依存も要らないように）。
//   node pack.js <出力.vsix> [<zip 内の名前>=<ファイル> ...]
const fs = require('fs');

const table = new Uint32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc32 = (buf) => {
  let c = 0xffffffff;
  for (const b of buf) c = table[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};

const [out, ...specs] = process.argv.slice(2);
const entries = [['extension/package.json', fs.readFileSync('package.json')]];
for (const spec of specs) {
  const [name, file] = spec.split('=');
  entries.push([name, fs.readFileSync(file)]);
}

const parts = [];
const central = [];
let offset = 0;
for (const [name, data] of entries) {
  const nameBuf = Buffer.from(name);
  const crc = crc32(data);
  const local = Buffer.alloc(30);
  local.writeUInt32LE(0x04034b50, 0);
  local.writeUInt16LE(20, 4);
  local.writeUInt32LE(crc, 14);
  local.writeUInt32LE(data.length, 18);
  local.writeUInt32LE(data.length, 22);
  local.writeUInt16LE(nameBuf.length, 26);
  const head = Buffer.alloc(46);
  head.writeUInt32LE(0x02014b50, 0);
  head.writeUInt16LE(20, 4);
  head.writeUInt16LE(20, 6);
  head.writeUInt32LE(crc, 16);
  head.writeUInt32LE(data.length, 20);
  head.writeUInt32LE(data.length, 24);
  head.writeUInt16LE(nameBuf.length, 28);
  head.writeUInt32LE(offset, 42);
  parts.push(local, nameBuf, data);
  central.push(head, nameBuf);
  offset += local.length + nameBuf.length + data.length;
}
const dir = Buffer.concat(central);
const end = Buffer.alloc(22);
end.writeUInt32LE(0x06054b50, 0);
end.writeUInt16LE(entries.length, 8);
end.writeUInt16LE(entries.length, 10);
end.writeUInt32LE(dir.length, 12);
end.writeUInt32LE(offset, 16);
fs.writeFileSync(out, Buffer.concat([...parts, dir, end]));

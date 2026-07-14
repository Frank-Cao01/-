import { readFile } from "node:fs/promises";

const packageJson = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
const tauriConfig = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const cargoToml = await readFile(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];

const versions = [packageJson.version, tauriConfig.version, cargoVersion];
if (new Set(versions).size !== 1) {
  console.error(`版本不一致：package=${versions[0]}, tauri=${versions[1]}, cargo=${versions[2]}`);
  process.exit(1);
}
const expectedTag = `v${versions[0]}`;
if (process.env.GITHUB_REF_TYPE === "tag" && process.env.GITHUB_REF_NAME !== expectedTag) {
  console.error(`版本标签不匹配：当前标签=${process.env.GITHUB_REF_NAME}，应为=${expectedTag}`);
  process.exit(1);
}
console.log(`版本一致：${versions[0]}`);

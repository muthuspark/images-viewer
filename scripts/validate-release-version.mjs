import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const stableSemverPattern = "(?:0|[1-9]\\d*)\\.(?:0|[1-9]\\d*)\\.(?:0|[1-9]\\d*)";
const stableTagRegex = new RegExp(`^v(${stableSemverPattern})$`);

function packageVersionFromToml(contents, packageName) {
  const packageBlocks = contents.split(/^\[\[package\]\]\s*$/m);

  for (const block of packageBlocks) {
    const name = block.match(/^name\s*=\s*"([^"]+)"\s*$/m)?.[1];
    if (name === packageName) {
      const version = block.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
      if (version) return version;
    }
  }

  throw new Error(`Could not find package ${packageName} in Cargo.lock`);
}

function manifestVersionFromToml(contents) {
  const packageHeader = "[package]";
  const packageStart = contents.indexOf(packageHeader);
  if (packageStart === -1) throw new Error("Could not find [package] in Cargo.toml");

  const afterHeader = contents.slice(packageStart + packageHeader.length);
  const nextSection = afterHeader.search(/^\[/m);
  const packageSection = nextSection === -1 ? afterHeader : afterHeader.slice(0, nextSection);
  const version = packageSection?.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];

  if (!version) throw new Error("Could not find [package].version in Cargo.toml");
  return version;
}

export function collectReleaseVersions(rootDir) {
  const readJson = (relativePath) =>
    JSON.parse(readFileSync(resolve(rootDir, relativePath), "utf8"));

  const tauriConfig = readJson("src-tauri/tauri.conf.json");
  const packageJson = readJson("package.json");
  const packageLock = readJson("package-lock.json");
  const cargoManifest = readFileSync(resolve(rootDir, "src-tauri/Cargo.toml"), "utf8");
  const cargoLock = readFileSync(resolve(rootDir, "src-tauri/Cargo.lock"), "utf8");

  return {
    "src-tauri/tauri.conf.json": tauriConfig.version,
    "package.json": packageJson.version,
    "package-lock.json": packageLock.packages?.[""]?.version,
    "src-tauri/Cargo.toml": manifestVersionFromToml(cargoManifest),
    "src-tauri/Cargo.lock": packageVersionFromToml(cargoLock, packageJson.name),
  };
}

export function validateReleaseVersion(tag, versions) {
  const match = stableTagRegex.exec(tag ?? "");
  if (!match) {
    throw new Error(`Release tag must match v<major>.<minor>.<patch>; received ${tag ?? "<empty>"}`);
  }

  const tagVersion = match[1];
  const mismatches = Object.entries(versions).filter(([, version]) => version !== tagVersion);

  if (mismatches.length > 0) {
    const details = mismatches
      .map(([source, version]) => `${source} declares ${version ?? "<missing>"}`)
      .join("; ");
    throw new Error(`Release tag ${tag} does not match every application version: ${details}`);
  }

  return tagVersion;
}

const scriptPath = fileURLToPath(import.meta.url);
if (process.argv[1] && resolve(process.argv[1]) === scriptPath) {
  try {
    const rootDir = resolve(dirname(scriptPath), "..");
    const tag = process.argv[2] ?? process.env.GITHUB_REF_NAME;
    const version = validateReleaseVersion(tag, collectReleaseVersions(rootDir));
    console.log(`Validated release tag v${version}`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  }
}

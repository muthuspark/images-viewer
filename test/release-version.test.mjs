import assert from "node:assert/strict";
import { test } from "node:test";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  collectReleaseVersions,
  validateReleaseVersion,
} from "../scripts/validate-release-version.mjs";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repositoryVersions = collectReleaseVersions(rootDir);

test("accepts a stable tag matching every repository version", () => {
  assert.equal(validateReleaseVersion("v0.1.0", repositoryVersions), "0.1.0");
});

test("rejects tags outside the stable v<semver> convention", () => {
  for (const tag of ["0.1.0", "v01.1.0", "v0.1", "v0.1.0-beta.1", "release-v0.1.0"]) {
    assert.throws(() => validateReleaseVersion(tag, repositoryVersions), /must match/);
  }
});

test("rejects a tag that differs from repository versions", () => {
  assert.throws(
    () => validateReleaseVersion("v0.2.0", repositoryVersions),
    /does not match every application version/,
  );
});

test("reports a mismatched individual version source", () => {
  assert.throws(
    () =>
      validateReleaseVersion("v0.1.0", {
        ...repositoryVersions,
        "src-tauri/Cargo.toml": "0.0.9",
      }),
    /src-tauri\/Cargo\.toml declares 0\.0\.9/,
  );
});

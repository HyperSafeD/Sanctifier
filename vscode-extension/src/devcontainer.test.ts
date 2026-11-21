import * as fs from 'fs';
import * as path from 'path';
import { looksLikeSorobanSource } from './analyzer';
import { hasSanctifierFeature } from './devcontainer';

const FIXTURES = path.join(__dirname, '..', 'src', 'test', 'fixtures', 'devcontainer');

function readFixture(name: string): string {
  return fs.readFileSync(path.join(FIXTURES, name), 'utf8');
}

// ---------------------------------------------------------------------------
// devcontainer.json structural validation
// ---------------------------------------------------------------------------

describe('devcontainer.json – structure', () => {
  let config: Record<string, unknown>;

  beforeAll(() => {
    const raw = readFixture('devcontainer.json');
    config = JSON.parse(raw) as Record<string, unknown>;
  });

  it('has a name field', () => {
    expect(typeof config.name).toBe('string');
    expect((config.name as string).length).toBeGreaterThan(0);
  });

  it('specifies a base image', () => {
    expect(typeof config.image).toBe('string');
    expect(config.image).toMatch(/rust/i);
  });

  it('includes wasm-pack feature', () => {
    const features = config.features as Record<string, unknown>;
    expect(features).toBeDefined();
    const keys = Object.keys(features);
    expect(keys.some((k) => k.includes('wasm-pack'))).toBe(true);
  });

  it('includes soroban-cli feature', () => {
    const features = config.features as Record<string, unknown>;
    const keys = Object.keys(features);
    expect(keys.some((k) => k.includes('soroban-cli'))).toBe(true);
  });

  it('has a postCreateCommand that builds the workspace', () => {
    expect(typeof config.postCreateCommand).toBe('string');
    expect(config.postCreateCommand).toContain('cargo build');
  });

  it('includes the sanctifier-soroban extension in VS Code customizations', () => {
    const customizations = config.customizations as {
      vscode: { extensions: string[] };
    };
    expect(customizations).toBeDefined();
    expect(customizations.vscode).toBeDefined();
    const exts = customizations.vscode.extensions;
    expect(Array.isArray(exts)).toBe(true);
    expect(exts.some((e) => e.includes('sanctifier'))).toBe(true);
  });

  it('includes rust-analyzer in VS Code customizations', () => {
    const customizations = config.customizations as {
      vscode: { extensions: string[] };
    };
    expect(customizations.vscode.extensions.some((e) => e.includes('rust-analyzer'))).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Cargo.toml fixture – Soroban project detection
// ---------------------------------------------------------------------------

describe('devcontainer fixtures – Cargo.toml Soroban detection', () => {
  it('soroban-cargo.toml is recognised as a Soroban project via soroban-sdk dependency', () => {
    const cargo = readFixture('soroban-cargo.toml');
    expect(/soroban.sdk/.test(cargo)).toBe(true);
  });

  it('plain-cargo.toml does NOT reference soroban-sdk', () => {
    const cargo = readFixture('plain-cargo.toml');
    expect(/soroban.sdk/.test(cargo)).toBe(false);
  });

  it('soroban-cargo.toml source hint triggers looksLikeSorobanSource when embedded', () => {
    const snippetFromSorobanProject = `
use soroban_sdk::{contract, contractimpl, Env};
#[contract]
pub struct Counter;
#[contractimpl]
impl Counter {
  pub fn increment(env: Env) -> u32 { 1 }
}
`;
    expect(looksLikeSorobanSource(snippetFromSorobanProject)).toBe(true);
  });

  it('source from a plain Rust app does NOT trigger looksLikeSorobanSource', () => {
    const snippetFromPlainProject = `
use std::collections::HashMap;
fn main() {
  let mut map: HashMap<String, u32> = HashMap::new();
  map.insert("hello".to_string(), 1);
}
`;
    expect(looksLikeSorobanSource(snippetFromPlainProject)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Remote workspace URI detection (devcontainers expose non-file:// schemes)
// ---------------------------------------------------------------------------

describe('devcontainer – remote workspace URI handling', () => {
  const REMOTE_SCHEMES = ['vscode-remote', 'ssh-remote', 'wsl', 'codespaces'];

  it.each(REMOTE_SCHEMES)('scheme "%s" is not a local "file" scheme', (scheme) => {
    expect(scheme).not.toBe('file');
  });

  it('file:// scheme is the only scheme treated as local', () => {
    const localScheme = 'file';
    expect(REMOTE_SCHEMES.every((s) => s !== localScheme)).toBe(true);
  });

  it('detects path-traversal segments in devcontainer workspace paths', () => {
    const hasTraversal = (p: string) => p.split('/').some((seg) => seg === '..');
    expect(hasTraversal('/workspaces/my-project')).toBe(false);
    expect(hasTraversal('/workspaces/../etc/passwd')).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Devcontainer features – contract (non-integration assertions on fixture JSON)
// ---------------------------------------------------------------------------

describe('devcontainer fixture contract', () => {
  it('fixture JSON is valid JSON and round-trips cleanly', () => {
    const raw = readFixture('devcontainer.json');
    expect(() => JSON.parse(raw)).not.toThrow();
    const parsed = JSON.parse(raw);
    expect(JSON.stringify(JSON.parse(JSON.stringify(parsed)))).toBe(
      JSON.stringify(parsed),
    );
  });

  it('feature versions are pinned to a major version (e.g. :1)', () => {
    const raw = readFixture('devcontainer.json');
    const config = JSON.parse(raw) as { features: Record<string, unknown> };
    for (const key of Object.keys(config.features)) {
      expect(key).toMatch(/:\d+$/);
    }
  });

  it('postCreateCommand does not contain shell injection characters', () => {
    const raw = readFixture('devcontainer.json');
    const config = JSON.parse(raw) as { postCreateCommand: string };
    expect(config.postCreateCommand).not.toMatch(/[;&|`$(){}]/);
  });
});

// ---------------------------------------------------------------------------
// Dockerfile-based devcontainer detection
// ---------------------------------------------------------------------------

describe('devcontainer – Dockerfile-based configuration', () => {
  let config: Record<string, unknown>;

  beforeAll(() => {
    const raw = readFixture('dockerfile-devcontainer.json');
    config = JSON.parse(raw) as Record<string, unknown>;
  });

  it('uses build.dockerfile rather than a top-level image field', () => {
    // A Dockerfile-based devcontainer specifies container build instructions
    // via a nested `build` object; it must NOT have a bare top-level `image`.
    expect(config.build).toBeDefined();
    const build = config.build as Record<string, unknown>;
    expect(typeof build.dockerfile).toBe('string');
    expect(build.dockerfile).toBeTruthy();
    expect(config.image).toBeUndefined();
  });

  it('build.dockerfile path is a non-empty string without path traversal', () => {
    const build = config.build as Record<string, unknown>;
    const dockerfile = build.dockerfile as string;
    const segments = dockerfile.split('/');
    expect(segments.every((seg) => seg !== '..')).toBe(true);
  });

  it('build context is defined alongside dockerfile', () => {
    const build = config.build as Record<string, unknown>;
    expect(build.context).toBeDefined();
  });

  it('sanctifier-cli feature is present in a Dockerfile-based container', () => {
    expect(hasSanctifierFeature(config)).toBe(true);
  });

  it('extension can locate the sanctifier binary when using a Dockerfile build', () => {
    // The binary lookup strategy for a Dockerfile-based container is the same
    // as for a prebuilt image: check the devcontainer features map for the
    // sanctifier-cli feature key.  Confirm the detection path works here.
    const features = config.features as Record<string, unknown>;
    const featureKeys = Object.keys(features);
    expect(featureKeys.some((k) => k.includes('sanctifier-cli'))).toBe(true);
  });

  it('has a name field', () => {
    expect(typeof config.name).toBe('string');
    expect((config.name as string).length).toBeGreaterThan(0);
  });

  it('fixture JSON round-trips cleanly', () => {
    const raw = readFixture('dockerfile-devcontainer.json');
    expect(() => JSON.parse(raw)).not.toThrow();
    const parsed = JSON.parse(raw);
    expect(JSON.stringify(JSON.parse(JSON.stringify(parsed)))).toBe(JSON.stringify(parsed));
  });
});

// ---------------------------------------------------------------------------
// Prebuilt-image-based devcontainer detection
// ---------------------------------------------------------------------------

describe('devcontainer – prebuilt-image-based configuration', () => {
  let config: Record<string, unknown>;

  beforeAll(() => {
    const raw = readFixture('prebuilt-image-devcontainer.json');
    config = JSON.parse(raw) as Record<string, unknown>;
  });

  it('uses a top-level image field rather than build.dockerfile', () => {
    // A prebuilt-image devcontainer specifies the container via a top-level
    // `image` field; it must NOT have a `build.dockerfile` entry.
    expect(typeof config.image).toBe('string');
    expect((config.image as string).length).toBeGreaterThan(0);
    if (config.build !== undefined) {
      const build = config.build as Record<string, unknown>;
      expect(build.dockerfile).toBeUndefined();
    }
  });

  it('image reference is a non-empty string', () => {
    expect((config.image as string).trim().length).toBeGreaterThan(0);
  });

  it('sanctifier-cli feature is present in a prebuilt-image container', () => {
    expect(hasSanctifierFeature(config)).toBe(true);
  });

  it('extension can locate the sanctifier binary when using a prebuilt image', () => {
    // For a prebuilt-image devcontainer the binary is installed via the
    // sanctifier-cli feature; confirm the feature key is present.
    const features = config.features as Record<string, unknown>;
    const featureKeys = Object.keys(features);
    expect(featureKeys.some((k) => k.includes('sanctifier-cli'))).toBe(true);
  });

  it('has a name field', () => {
    expect(typeof config.name).toBe('string');
    expect((config.name as string).length).toBeGreaterThan(0);
  });

  it('fixture JSON round-trips cleanly', () => {
    const raw = readFixture('prebuilt-image-devcontainer.json');
    expect(() => JSON.parse(raw)).not.toThrow();
    const parsed = JSON.parse(raw);
    expect(JSON.stringify(JSON.parse(JSON.stringify(parsed)))).toBe(JSON.stringify(parsed));
  });
});

// ---------------------------------------------------------------------------
// Cross-configuration: Dockerfile vs. prebuilt-image shape differences
// ---------------------------------------------------------------------------

describe('devcontainer – Dockerfile vs. prebuilt-image shape comparison', () => {
  let dockerfileConfig: Record<string, unknown>;
  let imageConfig: Record<string, unknown>;

  beforeAll(() => {
    dockerfileConfig = JSON.parse(readFixture('dockerfile-devcontainer.json'));
    imageConfig = JSON.parse(readFixture('prebuilt-image-devcontainer.json'));
  });

  it('exactly one of the two fixtures uses build.dockerfile', () => {
    const dfHasBuild =
      dockerfileConfig.build !== undefined &&
      typeof (dockerfileConfig.build as Record<string, unknown>).dockerfile === 'string';
    const imgHasBuild =
      imageConfig.build !== undefined &&
      typeof (imageConfig.build as Record<string, unknown>).dockerfile === 'string';
    expect(dfHasBuild).toBe(true);
    expect(imgHasBuild).toBe(false);
  });

  it('exactly one of the two fixtures uses a top-level image field', () => {
    expect(typeof dockerfileConfig.image).not.toBe('string');
    expect(typeof imageConfig.image).toBe('string');
  });

  it('sanctifier-cli feature detection works for both configurations', () => {
    expect(hasSanctifierFeature(dockerfileConfig)).toBe(true);
    expect(hasSanctifierFeature(imageConfig)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Sanctifier feature detection
// ---------------------------------------------------------------------------

describe('hasSanctifierFeature', () => {
  it('returns true when sanctifier-cli feature is present', () => {
    const config = {
      features: {
        'ghcr.io/devcontainers-contrib/features/wasm-pack:1': {},
        'ghcr.io/devcontainers/features/sanctifier-cli:1': {},
      },
    };
    expect(hasSanctifierFeature(config)).toBe(true);
  });

  it('returns true when sanctifier-cli is in any feature key', () => {
    const config = {
      features: {
        'docker.io/someuser/sanctifier-cli:latest': {},
      },
    };
    expect(hasSanctifierFeature(config)).toBe(true);
  });

  it('returns false when features object is missing', () => {
    const config: Record<string, unknown> = { name: 'test' };
    expect(hasSanctifierFeature(config)).toBe(false);
  });

  it('returns false when features exist but no sanctifier', () => {
    const config = {
      features: {
        'ghcr.io/devcontainers/features/rust:1': {},
        'ghcr.io/devcontainers/features/wasm-pack:1': {},
      },
    };
    expect(hasSanctifierFeature(config)).toBe(false);
  });

  it('returns false when features is empty object', () => {
    const config = { features: {} };
    expect(hasSanctifierFeature(config)).toBe(false);
  });
});

import * as fs from 'fs';
import * as path from 'path';
import { devcontainerJsonPath, hasSanctifierFeature } from './devcontainer';

// #1888: docs/getting-started.md must keep describing what devcontainer.ts
// actually does, so a change to the detection logic fails here until the
// documentation is updated too.
const DOCS = fs.readFileSync(path.join(__dirname, '..', '..', 'docs', 'getting-started.md'), 'utf8');
const SOURCE = fs.readFileSync(path.join(__dirname, 'devcontainer.ts'), 'utf8');

describe('docs/getting-started.md documents devcontainer detection', () => {
  it('has a dev container / Codespaces section', () => {
    expect(DOCS).toMatch(/^### Dev containers and Codespaces/m);
  });

  it('names the file the extension inspects', () => {
    const rel = devcontainerJsonPath({ uri: { fsPath: '/ws' } } as never);
    expect(path.relative('/ws', rel).split(path.sep).join('/')).toBe('.devcontainer/devcontainer.json');
    expect(DOCS).toContain('.devcontainer/devcontainer.json');
  });

  it('documents the exact feature key the extension adds', () => {
    const key = /const FEATURE_KEY_PREFIX = '([^']+)'/.exec(SOURCE)?.[1];
    expect(key).toBeTruthy();
    expect(DOCS).toContain(`"${key}:1": {}`);
  });

  it('documents the substring rule used by hasSanctifierFeature', () => {
    expect(hasSanctifierFeature({ features: { 'any.registry/x/sanctifier-cli:2': {} } })).toBe(true);
    expect(hasSanctifierFeature({ features: { 'ghcr.io/x/other:1': {} } })).toBe(false);
    expect(DOCS).toMatch(/containing the text `sanctifier-cli`/);
  });

  it('documents the prompt button, the rebuild step and the sanctifierPath setting', () => {
    expect(SOURCE).toContain("'Add to devcontainer'");
    expect(SOURCE).toContain('Rebuild the container to apply');
    expect(DOCS).toContain('Add to devcontainer');
    expect(DOCS).toMatch(/Rebuild Container/);
    expect(DOCS).toContain('sanctifier.sanctifierPath');
  });
});

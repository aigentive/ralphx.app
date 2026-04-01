import { describe, it, expect } from 'vitest';
import {
  PluginManifestSchema,
  PluginAuthorSchema,
  parsePluginManifest,
  safeParsePluginManifest,
  type PluginManifest,
  type PluginAuthor,
} from './plugin';

describe('PluginAuthorSchema', () => {
  it('should validate author with only name', () => {
    const author: PluginAuthor = { name: 'RalphX' };
    expect(PluginAuthorSchema.parse(author)).toEqual(author);
  });

  it('should validate author with all fields', () => {
    const author: PluginAuthor = {
      name: 'RalphX',
      email: 'test@example.com',
      url: 'https://example.com',
    };
    expect(PluginAuthorSchema.parse(author)).toEqual(author);
  });

  it('should reject empty name', () => {
    expect(() => PluginAuthorSchema.parse({ name: '' })).toThrow();
  });

  it('should reject invalid email', () => {
    expect(() =>
      PluginAuthorSchema.parse({ name: 'RalphX', email: 'not-an-email' })
    ).toThrow();
  });

  it('should reject invalid url', () => {
    expect(() =>
      PluginAuthorSchema.parse({ name: 'RalphX', url: 'not-a-url' })
    ).toThrow();
  });
});

describe('PluginManifestSchema', () => {
  const validManifest: PluginManifest = {
    name: 'ralphx',
    description: 'Autonomous development loop with extensible workflows',
    version: '1.0.0',
    author: { name: 'RalphX' },
  };

  it('should validate minimal manifest', () => {
    expect(PluginManifestSchema.parse(validManifest)).toEqual(validManifest);
  });

  it('should validate manifest with all component paths', () => {
    const manifest: PluginManifest = {
      ...validManifest,
      agents: './agents/',
      skills: './skills/',
      hooks: './hooks/hooks.json',
      mcpServers: './.mcp.json',
    };
    expect(PluginManifestSchema.parse(manifest)).toEqual(manifest);
  });

  it('should reject empty name', () => {
    expect(() =>
      PluginManifestSchema.parse({ ...validManifest, name: '' })
    ).toThrow();
  });

  it('should reject empty description', () => {
    expect(() =>
      PluginManifestSchema.parse({ ...validManifest, description: '' })
    ).toThrow();
  });

  it('should reject invalid semver version', () => {
    expect(() =>
      PluginManifestSchema.parse({ ...validManifest, version: '1.0' })
    ).toThrow();
  });

  it('should reject version with prefix', () => {
    expect(() =>
      PluginManifestSchema.parse({ ...validManifest, version: 'v1.0.0' })
    ).toThrow();
  });

  it('should reject missing author', () => {
    const { author: _author, ...noAuthor } = validManifest;
    expect(() => PluginManifestSchema.parse(noAuthor)).toThrow();
  });
});

describe('parsePluginManifest', () => {
  it('should parse valid manifest', () => {
    const json = {
      name: 'ralphx',
      description: 'Test plugin',
      version: '1.0.0',
      author: { name: 'Test' },
    };
    const result = parsePluginManifest(json);
    expect(result.name).toBe('ralphx');
    expect(result.version).toBe('1.0.0');
  });

  it('should throw on invalid manifest', () => {
    expect(() => parsePluginManifest({})).toThrow();
  });
});

describe('safeParsePluginManifest', () => {
  it('should return manifest on valid input', () => {
    const json = {
      name: 'ralphx',
      description: 'Test plugin',
      version: '1.0.0',
      author: { name: 'Test' },
    };
    const result = safeParsePluginManifest(json);
    expect(result).not.toBeNull();
    expect(result?.name).toBe('ralphx');
  });

  it('should return null on invalid input', () => {
    expect(safeParsePluginManifest({})).toBeNull();
    expect(safeParsePluginManifest(null)).toBeNull();
    expect(safeParsePluginManifest(undefined)).toBeNull();
  });
});

describe('RalphX plugin.json validation', () => {
  it('should validate the actual RalphX plugin manifest', () => {
    const ralphxManifest: PluginManifest = {
      name: 'ralphx',
      description: 'Autonomous development loop with extensible workflows',
      version: '1.0.0',
      author: { name: 'RalphX' },
      agents: './agents/',
      skills: './skills/',
      hooks: './hooks/hooks.json',
      mcpServers: './.mcp.json',
    };
    expect(PluginManifestSchema.parse(ralphxManifest)).toEqual(ralphxManifest);
  });
});

#!/usr/bin/env node
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const bridgeRoot = path.resolve(__dirname, '..');
const aliciaRoot = path.resolve(bridgeRoot, '..');

const schemaPath = path.join(bridgeRoot, 'schema', 'runtime-contract.json');
const snapshotPath = path.join(bridgeRoot, 'generated', 'runtime-contract.snapshot.json');
const frontendOutputPath = path.join(
  aliciaRoot,
  'frontend',
  'lib',
  'tauri-bridge',
  'generated',
  'runtime-contract.ts',
);
const backendOutputPath = path.join(
  aliciaRoot,
  'backend',
  'src',
  'generated',
  'runtime_contract.rs',
);

const args = new Set(process.argv.slice(2));
const writeExternal = args.has('--write-external');

function fail(message) {
  console.error(`[generate-runtime-contract] ${message}`);
  process.exit(1);
}

function assertStringArray(value, fieldName) {
  if (!Array.isArray(value)) {
    fail(`'${fieldName}' must be an array`);
  }

  for (const entry of value) {
    if (typeof entry !== 'string' || entry.trim().length === 0) {
      fail(`'${fieldName}' must contain only non-empty strings`);
    }
  }
}

function splitSegments(value) {
  return value
    .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    .split(/[^A-Za-z0-9]+/g)
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0);
}

function toCamelCase(value) {
  const [first, ...rest] = splitSegments(value);
  if (!first) {
    return '';
  }

  const tail = rest
    .map((entry) => entry.charAt(0).toUpperCase() + entry.slice(1).toLowerCase())
    .join('');

  return first.toLowerCase() + tail;
}

function toUpperSnakeCase(value) {
  return splitSegments(value)
    .map((entry) => entry.toUpperCase())
    .join('_');
}

function ensureValidTsIdentifier(label, identifier, sourceValue) {
  if (!/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(identifier)) {
    fail(`${label} '${sourceValue}' maps to invalid TS identifier '${identifier}'`);
  }
}

function ensureUniqueKey(map, label, key, sourceValue) {
  const existing = map.get(key);
  if (existing) {
    fail(`${label} key collision for '${key}': '${existing}' and '${sourceValue}'`);
  }
  map.set(key, sourceValue);
}

function deriveCommandEntries(commands) {
  const seen = new Map();
  return commands.map((command) => {
    const key = toCamelCase(command);
    ensureValidTsIdentifier('tauriCommands', key, command);
    ensureUniqueKey(seen, 'tauriCommands', key, command);
    return [key, command];
  });
}

function deriveChannelEntries(channels) {
  const seen = new Map();
  return channels.map((channel) => {
    const key = toCamelCase(channel);
    ensureValidTsIdentifier('tauriEventChannels', key, channel);
    ensureUniqueKey(seen, 'tauriEventChannels', key, channel);
    return [key, channel];
  });
}

function deriveRustChannelEntries(channels) {
  const seen = new Map();
  return channels.map((channel) => {
    const key = `EVENT_CHANNEL_${toUpperSnakeCase(channel)}`;
    if (!/^EVENT_CHANNEL_[A-Z0-9_]+$/.test(key)) {
      fail(`tauriEventChannels '${channel}' maps to invalid Rust identifier '${key}'`);
    }
    ensureUniqueKey(seen, 'tauriEventChannels', key, channel);
    return [key, channel];
  });
}

function parseSchema(raw) {
  let schema;
  try {
    schema = JSON.parse(raw);
  } catch (error) {
    fail(`failed to parse schema JSON: ${error.message}`);
  }

  if (!schema || typeof schema !== 'object' || Array.isArray(schema)) {
    fail('schema root must be an object');
  }

  if (typeof schema.contractVersion !== 'string' || schema.contractVersion.trim().length === 0) {
    fail(`'contractVersion' must be a non-empty string`);
  }

  if (!Array.isArray(schema.runtimeMethods)) {
    fail(`'runtimeMethods' must be an array`);
  }

  const runtimeMethods = schema.runtimeMethods.map((entry, index) => {
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
      fail(`runtimeMethods[${index}] must be an object`);
    }

    const name = entry.name;
    if (typeof name !== 'string' || name.trim().length === 0) {
      fail(`runtimeMethods[${index}].name must be a non-empty string`);
    }

    let aliases = [];
    if (entry.aliases !== undefined) {
      if (!Array.isArray(entry.aliases)) {
        fail(`runtimeMethods[${index}].aliases must be an array when present`);
      }
      aliases = entry.aliases.map((alias, aliasIndex) => {
        if (typeof alias !== 'string' || alias.trim().length === 0) {
          fail(`runtimeMethods[${index}].aliases[${aliasIndex}] must be a non-empty string`);
        }
        return alias;
      });
    }

    let aliasOf;
    if (entry.aliasOf !== undefined) {
      if (typeof entry.aliasOf !== 'string' || entry.aliasOf.trim().length === 0) {
        fail(`runtimeMethods[${index}].aliasOf must be a non-empty string when present`);
      }
      aliasOf = entry.aliasOf;
    }

    return { name, aliases, aliasOf };
  });

  assertStringArray(schema.tauriCommands, 'tauriCommands');
  assertStringArray(schema.tauriEventChannels, 'tauriEventChannels');

  return {
    contractVersion: schema.contractVersion,
    runtimeMethods,
    tauriCommands: schema.tauriCommands,
    tauriEventChannels: schema.tauriEventChannels,
  };
}

function renderTsArrayConst(arrayName, values) {
  const lines = values.map((value) => `  ${JSON.stringify(value)},`).join('\n');
  return `export const ${arrayName} = [\n${lines}\n] as const`;
}

function renderTsObjectConst(objectName, entries) {
  const lines = entries
    .map(([key, value]) => `  ${key}: ${JSON.stringify(value)},`)
    .join('\n');
  return `export const ${objectName} = {\n${lines}\n} as const`;
}

function renderTs(schema) {
  const runtimeMethods = schema.runtimeMethods.map((entry) => entry.name);
  const commandEntries = deriveCommandEntries(schema.tauriCommands);
  const channelEntries = deriveChannelEntries(schema.tauriEventChannels);

  return `// Auto-generated by alicia/codex-bridge/generators/generate-runtime-contract.mjs\n\nexport const RUNTIME_CONTRACT_VERSION = ${JSON.stringify(schema.contractVersion)} as const\n\n${renderTsArrayConst('RUNTIME_METHODS', runtimeMethods)}\n\n${renderTsObjectConst('RUNTIME_CHANNELS', channelEntries)}\n\n${renderTsObjectConst('RUNTIME_COMMANDS', commandEntries)}\n`;
}

function renderRustStringSlice(arrayName, values) {
  const lines = values.map((value) => `    ${JSON.stringify(value)},`).join('\n');
  return `pub(crate) const ${arrayName}: &[&str] = &[\n${lines}\n];`;
}

function renderRust(schema) {
  const runtimeMethods = schema.runtimeMethods.map((entry) => entry.name);
  const channelEntries = deriveRustChannelEntries(schema.tauriEventChannels);
  const channelConstants = channelEntries
    .map(([constant, channel]) => `pub(crate) const ${constant}: &str = ${JSON.stringify(channel)};`)
    .join('\n');

  return `// Auto-generated by alicia/codex-bridge/generators/generate-runtime-contract.mjs\n\npub(crate) const RUNTIME_CONTRACT_VERSION: &str = ${JSON.stringify(schema.contractVersion)};\n\n${renderRustStringSlice('RUNTIME_METHOD_KEYS', runtimeMethods)}\n\n${channelConstants}\n`;
}

async function writeGeneratedFile(filePath, content) {
  await mkdir(path.dirname(filePath), { recursive: true });
  await writeFile(filePath, content, 'utf8');
}

async function main() {
  const schemaRaw = await readFile(schemaPath, 'utf8');
  const schema = parseSchema(schemaRaw);

  await writeGeneratedFile(snapshotPath, `${JSON.stringify(schema, null, 2)}\n`);

  if (writeExternal) {
    await writeGeneratedFile(frontendOutputPath, renderTs(schema));
    await writeGeneratedFile(backendOutputPath, renderRust(schema));
  }

  const generatedTargets = [snapshotPath];
  if (writeExternal) {
    generatedTargets.push(frontendOutputPath, backendOutputPath);
  }

  console.log('[generate-runtime-contract] generated files:');
  for (const target of generatedTargets) {
    console.log(`- ${path.relative(aliciaRoot, target)}`);
  }

  if (!writeExternal) {
    console.log('[generate-runtime-contract] external outputs skipped (use --write-external to materialize frontend/backend artifacts)');
  }
}

main().catch((error) => {
  fail(error instanceof Error ? error.message : String(error));
});

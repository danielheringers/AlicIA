#!/usr/bin/env node
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const bridgeRoot = path.resolve(__dirname, '..');
const schemaPath = path.join(bridgeRoot, 'schema', 'runtime-contract.json');

function fail(message) {
  console.error(`[check-runtime-contract] ${message}`);
  process.exit(1);
}

function requireString(value, label) {
  if (typeof value !== 'string' || value.trim().length === 0) {
    fail(`${label} must be a non-empty string`);
  }
}

function ensureUniqueStrings(values, label) {
  const seen = new Set();
  for (const value of values) {
    requireString(value, `${label} entry`);
    if (seen.has(value)) {
      fail(`duplicate value in ${label}: ${value}`);
    }
    seen.add(value);
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

function ensureUniqueDerivedKeys(values, label, deriveFn) {
  const seen = new Map();
  for (const value of values) {
    const key = deriveFn(value);
    if (typeof key !== 'string' || key.length === 0) {
      fail(`${label} value '${value}' maps to an empty key`);
    }
    const existing = seen.get(key);
    if (existing) {
      fail(`${label} key collision for '${key}': '${existing}' and '${value}'`);
    }
    seen.set(key, value);
  }
}

async function main() {
  const raw = await readFile(schemaPath, 'utf8');

  let schema;
  try {
    schema = JSON.parse(raw);
  } catch (error) {
    fail(`invalid JSON: ${error.message}`);
  }

  if (!schema || typeof schema !== 'object' || Array.isArray(schema)) {
    fail('schema root must be an object');
  }

  requireString(schema.contractVersion, 'contractVersion');

  if (!Array.isArray(schema.runtimeMethods)) {
    fail('runtimeMethods must be an array');
  }
  if (!Array.isArray(schema.tauriCommands)) {
    fail('tauriCommands must be an array');
  }
  if (!Array.isArray(schema.tauriEventChannels)) {
    fail('tauriEventChannels must be an array');
  }

  const runtimeMethodNames = [];
  const aliasOfMap = new Map();
  const aliasesByCanonical = new Map();

  schema.runtimeMethods.forEach((entry, index) => {
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
      fail(`runtimeMethods[${index}] must be an object`);
    }

    requireString(entry.name, `runtimeMethods[${index}].name`);
    runtimeMethodNames.push(entry.name);

    if (entry.aliasOf !== undefined) {
      requireString(entry.aliasOf, `runtimeMethods[${index}].aliasOf`);
      if (entry.aliasOf === entry.name) {
        fail(`runtimeMethods[${index}] aliasOf cannot reference itself (${entry.name})`);
      }
      aliasOfMap.set(entry.name, entry.aliasOf);
    }

    if (entry.aliases !== undefined) {
      if (!Array.isArray(entry.aliases)) {
        fail(`runtimeMethods[${index}].aliases must be an array`);
      }
      ensureUniqueStrings(entry.aliases, `runtimeMethods[${index}].aliases`);
      aliasesByCanonical.set(entry.name, entry.aliases);
    }
  });

  ensureUniqueStrings(runtimeMethodNames, 'runtimeMethods');
  ensureUniqueStrings(schema.tauriCommands, 'tauriCommands');
  ensureUniqueStrings(schema.tauriEventChannels, 'tauriEventChannels');

  const runtimeMethodSet = new Set(runtimeMethodNames);

  for (const [alias, canonical] of aliasOfMap.entries()) {
    if (!runtimeMethodSet.has(canonical)) {
      fail(`aliasOf target does not exist: ${alias} -> ${canonical}`);
    }
  }

  for (const [canonical, aliases] of aliasesByCanonical.entries()) {
    for (const alias of aliases) {
      if (!runtimeMethodSet.has(alias)) {
        fail(`alias reference does not exist as runtime method: ${canonical} -> ${alias}`);
      }
      const aliasTarget = aliasOfMap.get(alias);
      if (aliasTarget !== canonical) {
        fail(`alias mismatch: ${canonical} declares ${alias}, but aliasOf points to ${aliasTarget ?? 'undefined'}`);
      }
    }
  }

  for (const channel of schema.tauriEventChannels) {
    if (!channel.includes('://')) {
      fail(`tauriEventChannels must use '<namespace>://<channel>' format: ${channel}`);
    }
  }

  ensureUniqueDerivedKeys(schema.tauriCommands, 'tauriCommands->tsKey', (command) => {
    const key = toCamelCase(command);
    if (!/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(key)) {
      fail(`tauriCommands value '${command}' maps to invalid TS key '${key}'`);
    }
    return key;
  });

  ensureUniqueDerivedKeys(schema.tauriEventChannels, 'tauriEventChannels->tsKey', (channel) => {
    const key = toCamelCase(channel);
    if (!/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(key)) {
      fail(`tauriEventChannels value '${channel}' maps to invalid TS key '${key}'`);
    }
    return key;
  });

  ensureUniqueDerivedKeys(
    schema.tauriEventChannels,
    'tauriEventChannels->rustConst',
    (channel) => {
      const key = `EVENT_CHANNEL_${toUpperSnakeCase(channel)}`;
      if (!/^EVENT_CHANNEL_[A-Z0-9_]+$/.test(key)) {
        fail(`tauriEventChannels value '${channel}' maps to invalid Rust const '${key}'`);
      }
      return key;
    },
  );

  console.log('[check-runtime-contract] schema is valid');
  console.log(`[check-runtime-contract] runtimeMethods=${schema.runtimeMethods.length}, tauriCommands=${schema.tauriCommands.length}, tauriEventChannels=${schema.tauriEventChannels.length}`);
}

main().catch((error) => {
  fail(error instanceof Error ? error.message : String(error));
});

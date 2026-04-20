import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SCRIPT = path.join(__dirname, '..', 'mj-sre-page.js');
const SAMPLE = path.join(__dirname, 'sample.html');

function run(args = '') {
  return execSync(`node ${SCRIPT} ${args} ${SAMPLE}`, {
    encoding: 'utf8',
    timeout: 90000,
  });
}

// ---------------------------------------------------------------------------
// Non-SRE tests
// ---------------------------------------------------------------------------

test('default (no options): wraps math in mjx-data with no output elements', () => {
  const out = run();
  assert.ok(out.includes('<mjx-data>'), 'should contain mjx-data');
  assert.ok(!out.includes('<svg'), 'should not contain SVG by default');
  assert.ok(!out.includes('<math'), 'should not contain MathML by default');
  assert.ok(!out.includes('<mjx-speech'), 'should not contain mjx-speech by default');
  assert.ok(!out.includes('<mjx-braille'), 'should not contain mjx-braille by default');
});

test('--svg: produces SVG elements inside mjx-data', () => {
  const out = run('--svg');
  assert.ok(out.includes('<mjx-data>'), 'should contain mjx-data');
  assert.ok(out.includes('<svg'), 'should contain SVG elements');
});

test('--mathml: produces MathML elements inside mjx-data', () => {
  const out = run('--mathml');
  assert.ok(out.includes('<mjx-data>'), 'should contain mjx-data');
  assert.ok(out.includes('<math'), 'should contain MathML elements');
});

test('--fontPaths: SVG uses glyph paths instead of defs/use caching', () => {
  const withCache = run('--svg');
  const withPaths = run('--svg --fontPaths');
  assert.ok(withPaths.includes('<svg'), 'should contain SVG');
  // With cached paths the SVG uses <defs> and <use>; with fontPaths it uses raw <path> elements.
  assert.ok(!withPaths.includes('<use '), 'fontPaths output should not use <use> elements');
  assert.ok(withCache.includes('<use '), 'default svg output should use cached <use> elements');
});

test('--em 32: processes document without error at larger em size', () => {
  const out = run('--svg --em 32');
  assert.ok(out.includes('<svg'), 'should contain SVG');
  assert.ok(out.includes('</html>'), 'should produce complete HTML document');
});

test('--packages "base, ams": processes with restricted TeX packages', () => {
  const out = run('--svg --packages "base, ams"');
  assert.ok(out.includes('<svg'), 'should contain SVG');
});

test('--packages default: all packages produce valid SVG output', () => {
  const out = run('--svg');
  assert.ok(out.includes('<svg'), 'should contain SVG');
  assert.ok(out.includes('</html>'), 'output should be a full HTML document');
});

// ---------------------------------------------------------------------------
// SRE-dependent tests (slower — allow up to 90 s via the run() timeout)
// ---------------------------------------------------------------------------

test('--speech: produces mjx-speech elements', { timeout: 90000 }, () => {
  const out = run('--speech');
  assert.ok(out.includes('<mjx-data>'), 'should contain mjx-data');
  assert.ok(out.includes('<mjx-speech>'), 'should contain mjx-speech elements');
});

test('--speech --rules clearspeak: speech uses clearspeak rule set', { timeout: 90000 }, () => {
  const outMathspeak = run('--speech --rules mathspeak');
  const outClearspeak = run('--speech --rules clearspeak');
  assert.ok(outClearspeak.includes('<mjx-speech>'), 'should contain mjx-speech');
  // Clearspeak and mathspeak produce different verbalizations
  assert.notEqual(outMathspeak, outClearspeak, 'clearspeak output should differ from mathspeak');
});

test('--speech --locale de: speech uses German locale', { timeout: 90000 }, () => {
  const outEn = run('--speech --locale en');
  const outDe = run('--speech --locale de');
  assert.ok(outDe.includes('<mjx-speech>'), 'should contain mjx-speech');
  assert.notEqual(outEn, outDe, 'German locale output should differ from English');
});

test('--braille: produces mjx-braille elements', { timeout: 90000 }, () => {
  const out = run('--braille');
  assert.ok(out.includes('<mjx-data>'), 'should contain mjx-data');
  assert.ok(out.includes('<mjx-braille>'), 'should contain mjx-braille elements');
});

test('--svgenhanced: produces speech-enriched SVG output', { timeout: 90000 }, () => {
  const out = run('--svgenhanced');
  assert.ok(out.includes('<mjx-data>'), 'should contain mjx-data');
  assert.ok(out.includes('<svg'), 'should contain SVG elements');
});

test('--svgenhanced --depth verbose: verbose speech depth changes SVG annotations', { timeout: 90000 }, () => {
  const shallow = run('--svgenhanced --depth shallow');
  const verbose = run('--svgenhanced --depth verbose');
  assert.ok(verbose.includes('<svg'), 'should contain SVG');
  assert.notEqual(shallow, verbose, 'verbose depth output should differ from shallow');
});

const { createCanvas } = require('canvas');
const fs = require('fs');
const path = require('path');

const iconsDir = path.join(__dirname, 'app/src-tauri/icons');

function makeIcon(size, filename, options = {}) {
  const canvas = createCanvas(size, size);
  const ctx = canvas.getContext('2d');

  if (options.bg) {
    const r = size * 0.2;
    ctx.beginPath();
    ctx.moveTo(r, 0);
    ctx.lineTo(size - r, 0);
    ctx.quadraticCurveTo(size, 0, size, r);
    ctx.lineTo(size, size - r);
    ctx.quadraticCurveTo(size, size, size - r, size);
    ctx.lineTo(r, size);
    ctx.quadraticCurveTo(0, size, 0, size - r);
    ctx.lineTo(0, r);
    ctx.quadraticCurveTo(0, 0, r, 0);
    ctx.closePath();
    ctx.fillStyle = options.bg;
    ctx.fill();
  }

  ctx.fillStyle = options.color || '#000000';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';

  const text = options.text || 'C';
  const fontSize = Math.round(size * (options.fontScale || 0.65));
  ctx.font = `bold ${fontSize}px Arial`;
  ctx.fillText(text, size / 2, size / 2 + size * 0.02);

  const buf = canvas.toBuffer('image/png');
  fs.writeFileSync(path.join(iconsDir, filename), buf);
  console.log(`${filename} (${size}x${size})`);
}

// Tray icon - big "C" black on transparent
makeIcon(32, 'tray.png', { color: '#000000', text: 'C', fontScale: 0.7 });

// App icons - "C" white on accent
const accent = '#e94560';
makeIcon(32, '32x32.png', { bg: accent, color: '#ffffff', text: 'C', fontScale: 0.65 });
makeIcon(128, '128x128.png', { bg: accent, color: '#ffffff', text: 'C', fontScale: 0.65 });
makeIcon(256, '128x128@2x.png', { bg: accent, color: '#ffffff', text: 'C', fontScale: 0.65 });
makeIcon(1024, 'icon.png', { bg: accent, color: '#ffffff', text: 'C', fontScale: 0.65 });

console.log('Done!');

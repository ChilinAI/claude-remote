const { createCanvas } = require('canvas');
const fs = require('fs');
const path = require('path');

function generateBackground(scale, outputFile) {
  const w = 660 * scale;
  const h = 400 * scale;
  const canvas = createCanvas(w, h);
  const ctx = canvas.getContext('2d');

  // Dark background
  ctx.fillStyle = '#2d2d2d';
  ctx.fillRect(0, 0, w, h);

  // Subtle grid
  ctx.strokeStyle = 'rgba(255,255,255,0.03)';
  ctx.lineWidth = 1 * scale;
  const gridSize = 60 * scale;
  for (let x = 0; x <= w; x += gridSize) {
    ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, h); ctx.stroke();
  }
  for (let y = 0; y <= h; y += gridSize) {
    ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke();
  }

  // Arrow: dashed line from app to Applications
  const arrowY = 200 * scale;
  const arrowStartX = 240 * scale;
  const arrowEndX = 430 * scale;

  ctx.strokeStyle = '#d4764e';
  ctx.lineWidth = 3 * scale;
  ctx.setLineDash([12 * scale, 8 * scale]);
  ctx.beginPath();
  ctx.moveTo(arrowStartX, arrowY);
  ctx.lineTo(arrowEndX - 15 * scale, arrowY);
  ctx.stroke();
  ctx.setLineDash([]);

  // Arrowhead
  ctx.fillStyle = '#d4764e';
  ctx.beginPath();
  ctx.moveTo(arrowEndX, arrowY);
  ctx.lineTo(arrowEndX - 18 * scale, arrowY - 10 * scale);
  ctx.lineTo(arrowEndX - 18 * scale, arrowY + 10 * scale);
  ctx.closePath();
  ctx.fill();

  // Text — light color on dark bg
  ctx.fillStyle = 'rgba(255, 255, 255, 0.75)';
  ctx.font = `${14 * scale}px -apple-system, "SF Pro Display", "Helvetica Neue", sans-serif`;
  ctx.textAlign = 'center';
  ctx.fillText('Drag to Applications', 330 * scale, (arrowY + 28 * scale));

  const buf = canvas.toBuffer('image/png');
  fs.writeFileSync(outputFile, buf);
  console.log(`Generated: ${outputFile} (${w}x${h})`);
}

const outDir = path.join(__dirname, 'app/src-tauri/icons');
generateBackground(1, path.join(outDir, 'dmg-background.png'));
generateBackground(2, path.join(outDir, 'dmg-background@2x.png'));

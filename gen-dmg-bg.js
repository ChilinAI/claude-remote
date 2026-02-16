const { createCanvas } = require('canvas');
const fs = require('fs');
const path = require('path');

const width = 660;
const height = 400;
const canvas = createCanvas(width, height);
const ctx = canvas.getContext('2d');

// Background gradient
const grad = ctx.createLinearGradient(0, 0, 0, height);
grad.addColorStop(0, '#2d2d2d');
grad.addColorStop(1, '#1a1a1a');
ctx.fillStyle = grad;
ctx.fillRect(0, 0, width, height);

// Subtle grid pattern
ctx.strokeStyle = 'rgba(255,255,255,0.03)';
ctx.lineWidth = 1;
for (let x = 0; x < width; x += 40) {
  ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, height); ctx.stroke();
}
for (let y = 0; y < height; y += 40) {
  ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(width, y); ctx.stroke();
}

// Arrow from left (app position ~180) to right (Applications position ~480)
const arrowY = 200;
const arrowStartX = 240;
const arrowEndX = 420;

// Arrow body
ctx.strokeStyle = 'rgba(212, 118, 78, 0.6)';
ctx.lineWidth = 3;
ctx.setLineDash([10, 6]);
ctx.beginPath();
ctx.moveTo(arrowStartX, arrowY);
ctx.lineTo(arrowEndX - 15, arrowY);
ctx.stroke();
ctx.setLineDash([]);

// Arrow head
ctx.fillStyle = 'rgba(212, 118, 78, 0.7)';
ctx.beginPath();
ctx.moveTo(arrowEndX, arrowY);
ctx.lineTo(arrowEndX - 20, arrowY - 10);
ctx.lineTo(arrowEndX - 20, arrowY + 10);
ctx.closePath();
ctx.fill();

// Text "Drag to Applications"
ctx.fillStyle = 'rgba(255,255,255,0.5)';
ctx.font = '14px Arial';
ctx.textAlign = 'center';
ctx.fillText('Drag to Applications', (arrowStartX + arrowEndX) / 2, arrowY + 35);

const buf = canvas.toBuffer('image/png');
const outDir = path.join(__dirname, 'app/src-tauri/icons');
fs.writeFileSync(path.join(outDir, 'dmg-background.png'), buf);
console.log(`dmg-background.png (${width}x${height})`);

// Also create @2x version
const canvas2 = createCanvas(width * 2, height * 2);
const ctx2 = canvas2.getContext('2d');
ctx2.scale(2, 2);

// Same drawing at 2x
const grad2 = ctx2.createLinearGradient(0, 0, 0, height);
grad2.addColorStop(0, '#2d2d2d');
grad2.addColorStop(1, '#1a1a1a');
ctx2.fillStyle = grad2;
ctx2.fillRect(0, 0, width, height);

ctx2.strokeStyle = 'rgba(255,255,255,0.03)';
ctx2.lineWidth = 1;
for (let x = 0; x < width; x += 40) {
  ctx2.beginPath(); ctx2.moveTo(x, 0); ctx2.lineTo(x, height); ctx2.stroke();
}
for (let y = 0; y < height; y += 40) {
  ctx2.beginPath(); ctx2.moveTo(0, y); ctx2.lineTo(width, y); ctx2.stroke();
}

ctx2.strokeStyle = 'rgba(212, 118, 78, 0.6)';
ctx2.lineWidth = 3;
ctx2.setLineDash([10, 6]);
ctx2.beginPath();
ctx2.moveTo(arrowStartX, arrowY);
ctx2.lineTo(arrowEndX - 15, arrowY);
ctx2.stroke();
ctx2.setLineDash([]);

ctx2.fillStyle = 'rgba(212, 118, 78, 0.7)';
ctx2.beginPath();
ctx2.moveTo(arrowEndX, arrowY);
ctx2.lineTo(arrowEndX - 20, arrowY - 10);
ctx2.lineTo(arrowEndX - 20, arrowY + 10);
ctx2.closePath();
ctx2.fill();

ctx2.fillStyle = 'rgba(255,255,255,0.5)';
ctx2.font = '14px Arial';
ctx2.textAlign = 'center';
ctx2.fillText('Drag to Applications', (arrowStartX + arrowEndX) / 2, arrowY + 35);

const buf2 = canvas2.toBuffer('image/png');
fs.writeFileSync(path.join(outDir, 'dmg-background@2x.png'), buf2);
console.log(`dmg-background@2x.png (${width*2}x${height*2})`);
